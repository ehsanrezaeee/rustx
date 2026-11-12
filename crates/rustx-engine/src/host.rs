//! Native event loop; scoped state remains on the UI thread.
mod services;
use crate::{cpu::CpuRenderer, gpu::Gpu, Engine};
use rustx_core::{
    HostCommand, HostError, HttpClient, HttpResponse, ServiceClient, TaskQueue,
    HOST_PROTOCOL_VERSION,
};
use std::{
    collections::HashMap,
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
};
use vello::kurbo::Point;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};
pub fn report_error(s: &str) {
    eprintln!("RustX: {s}");
}
fn transport_error(error: &(dyn std::error::Error + 'static)) -> HostError {
    let mut cause = Some(error);
    while let Some(error) = cause {
        if let Some(io) = error.downcast_ref::<std::io::Error>() {
            if matches!(
                io.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) {
                return HostError::Timeout;
            }
        }
        cause = error.source();
    }
    HostError::Transport("Network request failed".into())
}
struct Software {
    renderer: CpuRenderer,
    _context: softbuffer::Context<Arc<Window>>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
}
impl Software {
    fn new(w: Arc<Window>) -> Result<Self, String> {
        let c = softbuffer::Context::new(w.clone()).map_err(|e| e.to_string())?;
        let s = softbuffer::Surface::new(&c, w).map_err(|e| e.to_string())?;
        Ok(Self {
            renderer: CpuRenderer::new(1, 1),
            _context: c,
            surface: s,
        })
    }
    fn draw(&mut self, e: &mut Engine, w: u32, h: u32, dpr: f32) -> Result<(), String> {
        let Some(root) = &e.boxes else { return Ok(()) };
        self.renderer.sync_images(&e.painter);
        self.renderer.clear_color = e.layout.theme.background;
        self.renderer.draw(root, &mut e.text, w, h, dpr);
        self.renderer.focus(root, e.focused_node(), dpr);
        self.renderer
            .editing(root, e.focused_node(), &mut e.text, dpr);
        self.surface
            .resize(w.try_into().unwrap(), h.try_into().unwrap())
            .map_err(|e| e.to_string())?;
        let mut b = self.surface.buffer_mut().map_err(|e| e.to_string())?;
        for (dst, src) in b
            .iter_mut()
            .zip(self.renderer.pixmap.data().chunks_exact(4))
        {
            *dst = ((src[0] as u32) << 16) | ((src[1] as u32) << 8) | src[2] as u32;
        }
        b.present().map_err(|e| e.to_string())
    }
}
type ResultMessage = (u64, Result<HttpResponse, HostError>);
struct App {
    engine: Engine,
    http: HttpClient,
    services: ServiceClient,
    native_services: services::NativeServices,
    tasks: TaskQueue,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    cpu: Option<Software>,
    proxy: EventLoopProxy<()>,
    tx: mpsc::SyncSender<ResultMessage>,
    rx: mpsc::Receiver<ResultMessage>,
    pending: HashMap<u64, Arc<AtomicBool>>,
    pointer: Point,
    touch: Option<(u64, Point, bool)>,
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
    active_input: Option<u64>,
    frames: u32,
    smoke: u32,
    failure: Option<String>,
    accessibility: Option<crate::accessibility::NativeAccessibility>,
}
impl App {
    fn resource_failure(
        &mut self,
        source: rustx_core::ResourceSource,
        code: &str,
        status: Option<u16>,
    ) {
        let mut event = rustx_core::EventData::new(rustx_core::EventKind::ResourceError);
        event.resource_error = Some(rustx_core::ResourceFailure {
            source,
            code: code.into(),
            status,
        });
        self.engine.host_event(event);
    }
    fn network(&mut self) {
        if let Some(accessibility) = &mut self.accessibility {
            accessibility.dispatch_actions(&mut self.engine);
        }
        for failure in self.native_services.pump(&self.services, &self.proxy) {
            let mut event = rustx_core::EventData::new(rustx_core::EventKind::ResourceError);
            event.resource_error = Some(failure);
            self.engine.host_event(event);
        }
        for (id, result) in self.rx.try_iter() {
            self.pending.remove(&id);
            let failure = match &result {
                Err(HostError::Cancelled) => None,
                Err(HostError::Timeout) => Some(("timeout", None)),
                Err(HostError::TooLarge) => Some(("too_large", None)),
                Err(_) => Some(("transport", None)),
                _ => None,
            };
            let accepted = self.http.complete(HOST_PROTOCOL_VERSION, id, result);
            if let Some((code, status)) = failure.filter(|_| accepted) {
                let mut event = rustx_core::EventData::new(rustx_core::EventKind::ResourceError);
                event.resource_error = Some(rustx_core::ResourceFailure {
                    source: rustx_core::ResourceSource::Http,
                    code: code.into(),
                    status,
                });
                self.engine.host_event(event);
            }
        }
        for cmd in self.http.drain() {
            match cmd {
                HostCommand::Cancel { id, .. } => {
                    if let Some(c) = self.pending.get(&id) {
                        c.store(true, Ordering::Release);
                    }
                }
                HostCommand::Get(req) | HostCommand::Request(req) => {
                    if self.pending.len() >= 16 {
                        self.http.complete(
                            HOST_PROTOCOL_VERSION,
                            req.id,
                            Err(HostError::Transport("Too many pending requests".into())),
                        );
                        continue;
                    }
                    let cancel = Arc::new(AtomicBool::new(false));
                    self.pending.insert(req.id, cancel.clone());
                    let tx = self.tx.clone();
                    let wake = self.proxy.clone();
                    std::thread::spawn(move || {
                        let url = if req.url.starts_with('/') {
                            format!("http://127.0.0.1:3000{}", req.url)
                        } else {
                            req.url
                        };
                        let result = (|| {
                            let agent = ureq::AgentBuilder::new()
                                .timeout(std::time::Duration::from_millis(req.timeout_ms as u64))
                                .build();
                            let mut request = agent.request(&req.method, &url);
                            for (name, value) in &req.headers {
                                request = request.set(name, value);
                            }
                            let r = match if req.body.is_empty() {
                                request.call()
                            } else {
                                request.send_bytes(&req.body)
                            } {
                                Ok(r) => r,
                                Err(ureq::Error::Status(_, r)) => r,
                                Err(e) => return Err(transport_error(&e)),
                            };
                            let status = r.status();
                            let headers = r
                                .headers_names()
                                .into_iter()
                                .filter_map(|name| r.header(&name).map(|v| (name, v.to_owned())))
                                .collect();
                            let mut body = vec![];
                            r.into_reader()
                                .take(req.max_bytes as u64 + 1)
                                .read_to_end(&mut body)
                                .map_err(|e| transport_error(&e))?;
                            if body.len() > req.max_bytes {
                                return Err(HostError::TooLarge);
                            }
                            if cancel.load(Ordering::Acquire) {
                                return Err(HostError::Cancelled);
                            }
                            Ok(HttpResponse {
                                status,
                                headers,
                                body,
                            })
                        })();
                        let _ = tx.send((req.id, result));
                        let _ = wake.send_event(());
                    });
                }
            }
        }
        self.tasks.poll();
    }
    fn focus_input(&mut self) {
        self.active_input = self
            .engine
            .focused_node()
            .filter(|id| self.engine.runtime.editor(*id).is_some());
        if let Some(w) = &self.window {
            w.set_ime_allowed(
                self.active_input
                    .and_then(|id| self.engine.runtime.editor(id))
                    .is_some_and(|editor| !editor.is_read_only()),
            );
            if let Some(id) = self.active_input {
                fn find(b: &crate::layout::BoxNode, id: u64) -> Option<&crate::layout::BoxNode> {
                    if b.node.id == id {
                        Some(b)
                    } else {
                        b.children.iter().find_map(|b| find(b, id))
                    }
                }
                if let Some(b) = self.engine.boxes.as_ref().and_then(|b| find(b, id)) {
                    if let Some(editor) = &b.node.editing {
                        let style = &b.node.style;
                        let shaped = self.engine.text.shape_node(
                            &b.node,
                            (b.rect.width() as f32 - style.padding_horizontal()).max(1.),
                        );
                        let caret = crate::editing::caret_rect(
                            &shaped,
                            editor.selection.focus,
                            style.font_size,
                        );
                        w.set_ime_cursor_area(
                            winit::dpi::LogicalPosition::new(
                                b.rect.x0 + style.padding_left() as f64 + caret.x0,
                                b.rect.y0 + style.padding_top() as f64 + caret.y0
                                    - f64::from(editor.scroll_y),
                            ),
                            winit::dpi::LogicalSize::new(caret.width(), caret.height()),
                        );
                    }
                }
            }
        }
    }
}
impl ApplicationHandler<()> for App {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let w = match events.create_window(
            Window::default_attributes()
                .with_title("RustX / Stage 3")
                .with_visible(false)
                .with_inner_size(LogicalSize::new(1150., 900.)),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.failure = Some(e.to_string());
                events.exit();
                return;
            }
        };
        self.accessibility = Some(crate::accessibility::NativeAccessibility::new(
            events,
            &w,
            self.proxy.clone(),
        ));
        w.set_visible(true);
        if std::env::var("RUSTX_RENDERER").as_deref() != Ok("cpu") {
            match pollster::block_on(Gpu::new(w.clone())) {
                Ok(g) => self.gpu = Some(g),
                Err(e) => {
                    self.resource_failure(
                        rustx_core::ResourceSource::Renderer,
                        "gpu_init_failed",
                        None,
                    );
                    report_error(&format!("GPU failed ({e}); trying CPU"));
                }
            }
        }
        if self.gpu.is_none() {
            match Software::new(w.clone()) {
                Ok(c) => self.cpu = Some(c),
                Err(e) => {
                    self.failure = Some(e);
                    events.exit();
                    return;
                }
            }
        }
        w.request_redraw();
        self.window = Some(w);
    }
    fn user_event(&mut self, _: &ActiveEventLoop, _: ()) {
        self.network();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(w) = self.window.clone() else { return };
        if let Some(accessibility) = &mut self.accessibility {
            accessibility.process_event(&w, &event);
        }
        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                let size = w.inner_size();
                let scale = w.scale_factor();
                let mut event = rustx_core::EventData::new(rustx_core::EventKind::Resize);
                event.width = size.width as f64 / scale;
                event.height = size.height as f64 / scale;
                event.scale = scale;
                self.engine.host_event(event);
                self.engine.invalidate();
                w.request_redraw();
            }
            WindowEvent::DroppedFile(path) => {
                if let Some(name) = path.file_name() {
                    let metadata = rustx_core::DroppedFileMetadata {
                        name: name.to_string_lossy().into_owned(),
                        mime: None,
                        size: std::fs::metadata(&path).ok().map(|metadata| metadata.len()),
                    };
                    let mut event = rustx_core::EventData::new(rustx_core::EventKind::FileDrop);
                    event.x = self.pointer.x;
                    event.y = self.pointer.y;
                    event.files.push(metadata);
                    self.engine.host_event(event);
                    self.network();
                    w.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.ctrl = m.state().control_key();
                self.alt = m.state().alt_key();
                self.meta = m.state().super_key();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer =
                    Point::new(position.x / w.scale_factor(), position.y / w.scale_factor());
                self.engine.pointer_move(self.pointer);
                if self.engine.runtime.needs_update()
                    || self.engine.runtime.captured_pointer(0).is_some()
                {
                    w.request_redraw();
                }
            }
            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                self.engine.cancel();
                if let Some(id) = self.active_input {
                    if let Some(editor) = self.engine.runtime.editor(id) {
                        editor.composition_cancel();
                        self.engine.runtime.invalidate_model();
                        self.engine.invalidate();
                    }
                }
            }
            WindowEvent::Touch(touch) => {
                let point = Point::new(
                    touch.location.x / w.scale_factor(),
                    touch.location.y / w.scale_factor(),
                );
                match touch.phase {
                    winit::event::TouchPhase::Started if self.touch.is_none() => {
                        self.touch = Some((touch.id, point, false));
                        self.pointer = point;
                        self.engine.pointer_down(point);
                        self.focus_input();
                    }
                    winit::event::TouchPhase::Moved => {
                        if let Some((id, last, dragged)) = self.touch {
                            if id == touch.id {
                                let dy = point.y - last.y;
                                let changed = dragged || point.distance(last) > 2.;
                                if self
                                    .engine
                                    .focused_node()
                                    .is_some_and(|id| self.engine.canvas_at(id).is_some())
                                {
                                    self.engine.pointer_move(point);
                                } else if changed {
                                    self.engine.cancel();
                                    self.engine.scroll_at(point, -dy);
                                }
                                self.touch = Some((id, point, changed));
                            }
                        }
                    }
                    winit::event::TouchPhase::Ended => {
                        if self.touch.is_some_and(|(id, _, _)| id == touch.id) {
                            self.touch = None;
                            self.engine.pointer_up(point);
                        }
                    }
                    winit::event::TouchPhase::Cancelled => {
                        if self.touch.is_some_and(|(id, _, _)| id == touch.id) {
                            self.touch = None;
                            self.engine.cancel();
                        }
                    }
                    _ => {}
                }
                self.network();
                w.request_redraw();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                if state == ElementState::Pressed {
                    self.engine.pointer_down(self.pointer);
                    self.focus_input();
                } else {
                    self.engine.pointer_up(self.pointer);
                }
                self.network();
                w.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64 * 40.,
                    MouseScrollDelta::PixelDelta(p) => p.y,
                };
                self.engine.scroll_at(self.pointer, -y);
                w.request_redraw();
            }
            WindowEvent::Ime(Ime::Commit(s)) => {
                if let Some(id) = self.active_input {
                    if let Some(editor) = self.engine.runtime.editor(id) {
                        editor.composition_cancel();
                    }
                }
                self.engine.insert_text(&s);
                w.request_redraw();
            }
            WindowEvent::Ime(Ime::Preedit(s, _)) => {
                if let Some(id) = self.active_input {
                    if let Some(editor) = self.engine.runtime.editor(id) {
                        let _ = editor.composition_update(&s);
                        self.engine.runtime.invalidate_model();
                        self.engine.invalidate();
                    }
                }
                w.request_redraw();
            }
            WindowEvent::Ime(Ime::Disabled) => {
                if let Some(id) = self.active_input {
                    if let Some(editor) = self.engine.runtime.editor(id) {
                        editor.composition_cancel();
                        self.engine.runtime.invalidate_model();
                        self.engine.invalidate();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let key = match &event.logical_key {
                    Key::Named(NamedKey::Space) => " ".to_owned(),
                    Key::Named(n) => format!("{n:?}"),
                    Key::Character(s) => s.to_string(),
                    _ => String::new(),
                };
                let handled = self
                    .engine
                    .key(&key, self.shift, self.ctrl, self.alt, self.meta);
                if !handled && (self.ctrl || self.meta) && !self.alt {
                    if let Some(id) = self.active_input {
                        if let Some(editor) = self.engine.runtime.editor(id) {
                            match key.to_lowercase().as_str() {
                                "c" => {
                                    if let Ok(text) = editor.copy() {
                                        let _ = services::clipboard(false, &text);
                                    }
                                }
                                "x" => {
                                    if let Ok(text) = editor.copy() {
                                        if services::clipboard(false, &text).is_ok() {
                                            let _ = editor.cut();
                                            self.engine.runtime.input(id, editor.value(), false);
                                            self.engine.runtime.invalidate_model();
                                            self.engine.invalidate();
                                        }
                                    }
                                }
                                "v" => {
                                    if let Ok(rustx_core::ServiceValue::Text { text }) =
                                        services::clipboard(true, "")
                                    {
                                        self.engine.insert_text(&text);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                self.focus_input();
                self.network();
                w.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(id) = self.engine.focused_node() {
                    let mut data = rustx_core::EventData::new(rustx_core::EventKind::KeyUp);
                    data.key = match event.logical_key {
                        Key::Named(n) => format!("{n:?}"),
                        Key::Character(s) => s.to_string(),
                        _ => String::new(),
                    };
                    self.engine.runtime.dispatch(id, data);
                }
            }
            WindowEvent::RedrawRequested => {
                self.network();
                let size = w.inner_size();
                if size.width > 8192
                    || size.height > 8192
                    || size.width as u64 * size.height as u64 > 16_777_216
                {
                    self.failure =
                        Some("Native surface exceeds the 8192px/16M pixel rendering budget".into());
                    events.exit();
                    return;
                }
                if size.width == 0 || size.height == 0 {
                    return;
                }
                let scale = w.scale_factor();
                self.engine.frame(
                    (size.width as f64 / scale) as f32,
                    (size.height as f64 / scale) as f32,
                    scale,
                );
                self.focus_input();
                if let Some(accessibility) = &mut self.accessibility {
                    accessibility.update(&self.engine);
                }
                let r = if let Some(g) = &mut self.gpu {
                    g.resize(size);
                    g.draw(&self.engine)
                } else if let Some(c) = &mut self.cpu {
                    c.draw(&mut self.engine, size.width, size.height, scale as f32)
                } else {
                    Ok(())
                };
                if let Err(error) = r {
                    self.resource_failure(
                        rustx_core::ResourceSource::Renderer,
                        "presentation_failed",
                        None,
                    );
                    if self.gpu.take().is_some() {
                        report_error(&format!("GPU failed ({error}); recovering through CPU"));
                        match Software::new(w.clone()).and_then(|mut cpu| {
                            cpu.draw(&mut self.engine, size.width, size.height, scale as f32)?;
                            self.cpu = Some(cpu);
                            Ok(())
                        }) {
                            Ok(()) => (),
                            Err(e) => {
                                self.failure = Some(e);
                                events.exit();
                            }
                        }
                    } else {
                        self.failure = Some(error);
                        events.exit();
                    }
                }
                self.frames += 1;
                if self.smoke > 0 {
                    if self.frames >= self.smoke {
                        events.exit()
                    } else {
                        w.request_redraw()
                    }
                } else if self.engine.runtime.needs_update() {
                    w.request_redraw();
                }
            }
            _ => (),
        }
    }
}
pub fn run_with_http(engine: Engine, http: HttpClient) -> Result<(), String> {
    run_with_services(engine, http, ServiceClient::default(), TaskQueue::default())
}
pub fn run_with_services(
    mut engine: Engine,
    http: HttpClient,
    services: ServiceClient,
    tasks: TaskQueue,
) -> Result<(), String> {
    if services.capabilities().is_none() {
        services
            .connect(services::NativeServices::capabilities())
            .map_err(|_| "Native service handshake failed")?;
    }
    let events = EventLoop::<()>::with_user_event()
        .build()
        .map_err(|e| e.to_string())?;
    events.set_control_flow(ControlFlow::Wait);
    let proxy = events.create_proxy();
    let wake = proxy.clone();
    engine.runtime.set_waker(move || {
        let _ = wake.send_event(());
    });
    let (tx, rx) = mpsc::sync_channel(16);
    let mut a = App {
        engine,
        http,
        services,
        native_services: services::NativeServices::default(),
        tasks,
        window: None,
        gpu: None,
        cpu: None,
        proxy,
        tx,
        rx,
        pending: HashMap::new(),
        pointer: Point::ZERO,
        touch: None,
        shift: false,
        ctrl: false,
        alt: false,
        meta: false,
        active_input: None,
        frames: 0,
        smoke: std::env::var("RUSTX_SMOKE_FRAMES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        failure: None,
        accessibility: None,
    };
    events.run_app(&mut a).map_err(|e| e.to_string())?;
    for c in a.pending.values() {
        c.store(true, Ordering::Release)
    }
    a.failure.map_or(Ok(()), Err)
}
