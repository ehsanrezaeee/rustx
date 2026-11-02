//! Native event loop; scoped state remains on the UI thread.
use crate::{cpu::CpuRenderer, gpu::Gpu, Engine};
use rustx_core::{HostCommand, HostError, HttpClient, HttpResponse, HOST_PROTOCOL_VERSION};
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
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    cpu: Option<Software>,
    proxy: EventLoopProxy<()>,
    tx: mpsc::SyncSender<ResultMessage>,
    rx: mpsc::Receiver<ResultMessage>,
    pending: HashMap<u64, Arc<AtomicBool>>,
    pointer: Point,
    shift: bool,
    active_input: Option<u64>,
    frames: u32,
    smoke: u32,
    failure: Option<String>,
}
impl App {
    fn network(&mut self) {
        for (id, result) in self.rx.try_iter() {
            self.pending.remove(&id);
            self.http.complete(HOST_PROTOCOL_VERSION, id, result);
        }
        for cmd in self.http.drain() {
            match cmd {
                HostCommand::Cancel { id, .. } => {
                    if let Some(c) = self.pending.get(&id) {
                        c.store(true, Ordering::Release);
                    }
                }
                HostCommand::Get(req) => {
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
                            let r = match agent.get(&url).call() {
                                Ok(r) => r,
                                Err(ureq::Error::Status(_, r)) => r,
                                Err(e) => return Err(HostError::Transport(e.to_string())),
                            };
                            let status = r.status();
                            let mut body = vec![];
                            r.into_reader()
                                .take(req.max_bytes as u64 + 1)
                                .read_to_end(&mut body)
                                .map_err(|e| HostError::Transport(e.to_string()))?;
                            if body.len() > req.max_bytes {
                                return Err(HostError::TooLarge);
                            }
                            if cancel.load(Ordering::Acquire) {
                                return Err(HostError::Cancelled);
                            }
                            Ok(HttpResponse { status, body })
                        })();
                        let _ = tx.send((req.id, result));
                        let _ = wake.send_event(());
                    });
                }
            }
        }
    }
    fn focus_input(&mut self) {
        fn input(n: &rustx_core::Snapshot, id: u64) -> bool {
            (n.id == id && n.kind == rustx_core::Kind::Input)
                || n.children.iter().any(|c| input(c, id))
        }
        self.active_input = self
            .engine
            .focused_node()
            .filter(|id| input(&self.engine.runtime.snapshot(), *id));
        if let Some(w) = &self.window {
            w.set_ime_allowed(self.active_input.is_some());
        }
    }
    fn edit(&mut self, text: &str, delete: bool) {
        let Some(id) = self.active_input else { return };
        fn value(n: &rustx_core::Snapshot, id: u64) -> Option<String> {
            if n.id == id {
                Some(n.text.clone())
            } else {
                n.children.iter().find_map(|c| value(c, id))
            }
        }
        if let Some(mut v) = value(&self.engine.runtime.snapshot(), id) {
            if delete {
                v.pop();
            } else {
                v.push_str(text);
            }
            self.engine.runtime.input(id, v, false);
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
                .with_title("RustX / Stage 1")
                .with_inner_size(LogicalSize::new(1150., 900.)),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.failure = Some(e.to_string());
                events.exit();
                return;
            }
        };
        if std::env::var("RUSTX_RENDERER").as_deref() != Ok("cpu") {
            match pollster::block_on(Gpu::new(w.clone())) {
                Ok(g) => self.gpu = Some(g),
                Err(e) => report_error(&format!("GPU failed ({e}); trying CPU")),
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
        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.engine.invalidate();
                w.request_redraw();
            }
            WindowEvent::ModifiersChanged(m) => self.shift = m.state().shift_key(),
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer =
                    Point::new(position.x / w.scale_factor(), position.y / w.scale_factor());
                self.engine.pointer_move(self.pointer);
                if self.engine.runtime.needs_update() {
                    w.request_redraw();
                }
            }
            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => self.engine.cancel(),
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
                self.edit(&s, false);
                w.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::Tab) => {
                        self.engine.tab(self.shift);
                        self.focus_input();
                    }
                    Key::Named(NamedKey::Backspace) if self.active_input.is_some() => {
                        self.edit("", true)
                    }
                    Key::Named(NamedKey::Enter | NamedKey::Space)
                        if self.active_input.is_none() =>
                    {
                        self.engine.activate()
                    }
                    Key::Character(ref s) if self.active_input.is_some() => self.edit(s, false),
                    _ => (),
                }
                self.network();
                w.request_redraw();
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
                let r = if let Some(g) = &mut self.gpu {
                    g.resize(size);
                    g.draw(&self.engine)
                } else if let Some(c) = &mut self.cpu {
                    c.draw(&mut self.engine, size.width, size.height, scale as f32)
                } else {
                    Ok(())
                };
                if let Err(error) = r {
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
pub fn run_with_http(mut engine: Engine, http: HttpClient) -> Result<(), String> {
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
        window: None,
        gpu: None,
        cpu: None,
        proxy,
        tx,
        rx,
        pending: HashMap::new(),
        pointer: Point::ZERO,
        shift: false,
        active_input: None,
        frames: 0,
        smoke: std::env::var("RUSTX_SMOKE_FRAMES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        failure: None,
    };
    events.run_app(&mut a).map_err(|e| e.to_string())?;
    for c in a.pending.values() {
        c.store(true, Ordering::Release)
    }
    a.failure.map_or(Ok(()), Err)
}
