#[path = "app.rustx"]
mod app;
#[cfg(feature = "instrumentation")]
#[global_allocator]
static ALLOCATOR: rustx::allocation::CountingAllocator<std::alloc::System> =
    rustx::allocation::CountingAllocator(std::alloc::System);
#[path = "applications.rustx"]
pub mod applications;
#[path = "playground.rustx"]
pub mod playground;
#[path = "visual.rustx"]
pub mod visual;
use rustx::prelude::*;
pub fn start_page() -> i32 {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("RUSTX_DEMO").as_deref() == Ok("engine") {
        return 3;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("RUSTX_DEMO").as_deref() == Ok("applications") {
        return 4;
    }
    0
}
pub fn engine() -> (rustx_engine::Engine, HttpClient) {
    engine_with_host(HostCapabilities::get_cancel()).expect("native GET/cancel host")
}
pub fn engine_with_host(
    capabilities: HostCapabilities,
) -> Result<(rustx_engine::Engine, HttpClient), HostError> {
    engine_with_environment(capabilities, ServiceClient::default(), TaskQueue::default())
}
pub fn engine_with_environment(
    capabilities: HostCapabilities,
    services: ServiceClient,
    tasks: TaskQueue,
) -> Result<(rustx_engine::Engine, HttpClient), HostError> {
    let h = HttpClient::default();
    h.connect(capabilities)?;
    let client = h.clone();
    Ok((
        rustx_engine::Engine::new(move |cx| {
            let endpoint = cx.signal(String::new());
            cx.provide(playground::CompilerEndpoint(endpoint));
            cx.provide(services.clone());
            cx.provide(tasks.clone());
            app::root(cx, client.clone())
        }),
        h,
    ))
}
#[cfg(target_arch = "wasm32")]
mod browser {
    use super::*;
    use rustx_web::{gpu::WebGpu, WebApp};
    use std::cell::RefCell;
    use wasm_bindgen::prelude::*;
    thread_local! {static APP:RefCell<Option<WebApp>>=const{RefCell::new(None)};static GPU:RefCell<Option<WebGpu>>=const{RefCell::new(None)};}
    #[wasm_bindgen]
    pub fn init_app(capabilities: &str) -> Result<(), JsValue> {
        let capabilities = serde_json::from_str(capabilities)
            .map_err(|e| JsValue::from_str(&format!("Invalid host report: {e}")))?;
        let services = ServiceClient::default();
        let tasks = TaskQueue::default();
        let (e, h) = engine_with_environment(capabilities, services.clone(), tasks.clone())
            .map_err(|e| JsValue::from_str(&format!("Host handshake failed: {e:?}")))?;
        GPU.with(|g| g.replace(None));
        let mut app = WebApp::new(e, h);
        app.services = services;
        app.tasks = tasks;
        APP.with(|a| a.replace(Some(app)));
        Ok(())
    }
    #[wasm_bindgen]
    pub async fn init_gpu(canvas: rustx_web::HtmlCanvasElement) -> Result<(), JsValue> {
        let gpu = WebGpu::new(canvas)
            .await
            .map_err(|e| JsValue::from_str(&e))?;
        GPU.with(|g| g.replace(Some(gpu)));
        Ok(())
    }
    #[wasm_bindgen]
    pub fn frame(w: f32, h: f32, dpr: f32) -> Result<bool, JsValue> {
        APP.with(|a| {
            let mut a = a.borrow_mut();
            let a = a.as_mut().unwrap();
            GPU.with(|g| {
                let mut g = g.borrow_mut();
                if let Some(g) = g.as_mut() {
                    let dirty = a.engine.frame(w, h, dpr as f64);
                    if dirty {
                        g.draw(
                            &a.engine,
                            (w * dpr).round().max(1.) as u32,
                            (h * dpr).round().max(1.) as u32,
                        )
                        .map_err(|e| JsValue::from_str(&e))?;
                    }
                    Ok(dirty)
                } else {
                    Ok(a.frame(w, h, dpr))
                }
            })
        })
    }
    #[wasm_bindgen]
    pub fn drop_gpu() {
        GPU.with(|g| g.replace(None));
        APP.with(|a| a.borrow_mut().as_mut().unwrap().engine.invalidate());
    }
    #[wasm_bindgen]
    pub fn invalidate() {
        APP.with(|a| a.borrow_mut().as_mut().unwrap().engine.invalidate());
    }
    #[wasm_bindgen]
    pub fn destroy_gpu() {
        GPU.with(|g| {
            if let Some(g) = g.borrow().as_ref() {
                g.destroy();
            }
        });
        APP.with(|a| a.borrow_mut().as_mut().unwrap().engine.invalidate());
    }
    #[wasm_bindgen]
    pub fn set_theme(light: bool) {
        APP.with(|a| {
            a.borrow_mut().as_mut().unwrap().engine.set_theme(if light {
                Theme::LIGHT
            } else {
                Theme::DARK
            })
        });
    }
    #[wasm_bindgen]
    pub fn canvas_pan(id: f64, dx: f64, dy: f64) {
        let Some(id) = rustx_web::host_id(id) else {
            return;
        };
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        APP.with(|a| {
            a.borrow_mut()
                .as_mut()
                .unwrap()
                .engine
                .canvas_event(id, rustx::drawing::CanvasEvent::Pan { dx, dy });
        });
    }
    #[wasm_bindgen]
    pub fn canvas_zoom(id: f64, x: f64, y: f64, factor: f64) {
        let Some(id) = rustx_web::host_id(id) else {
            return;
        };
        APP.with(|a| {
            let mut a = a.borrow_mut();
            let e = &mut a.as_mut().unwrap().engine;
            if let Some(rect) = e.canvas_at(id) {
                e.canvas_event(
                    id,
                    rustx::drawing::CanvasEvent::Zoom {
                        x: x - rect.x0,
                        y: y - rect.y0,
                        factor,
                    },
                );
            }
        });
    }
    #[wasm_bindgen]
    pub fn canvas_pick(id: f64, x: f64, y: f64) {
        let Some(id) = rustx_web::host_id(id) else {
            return;
        };
        APP.with(|a| {
            let mut a = a.borrow_mut();
            let e = &mut a.as_mut().unwrap().engine;
            if let Some(rect) = e.canvas_at(id) {
                if rect.contains(rustx_engine::vello::kurbo::Point::new(x, y)) {
                    e.canvas_event(
                        id,
                        rustx::drawing::CanvasEvent::Pick {
                            x: x - rect.x0,
                            y: y - rect.y0,
                        },
                    );
                }
            }
        });
    }
    #[wasm_bindgen]
    pub fn scroll_node(id: f64, dy: f64) -> bool {
        let Some(id) = rustx_web::host_id(id) else {
            return false;
        };
        APP.with(|a| {
            a.borrow_mut()
                .as_mut()
                .unwrap()
                .engine
                .runtime
                .scroll(id, dy)
        })
    }
    #[wasm_bindgen]
    pub fn pixels() -> Vec<u8> {
        APP.with(|a| a.borrow().as_ref().unwrap().cpu.pixmap.data().to_vec())
    }
    #[wasm_bindgen]
    pub fn set_diagnostics(enabled: bool) {
        APP.with(|a| {
            a.borrow_mut()
                .as_mut()
                .unwrap()
                .engine
                .diagnostics
                .enable(enabled)
        });
    }
    #[wasm_bindgen]
    pub fn configure_playground(origin: &str) {
        APP.with(|a| {
            let a = a.borrow();
            let a = a.as_ref().unwrap();
            if let Some(scope) = a
                .engine
                .runtime
                .scope_for_node(a.engine.runtime.snapshot().id)
            {
                if let Some(config) = scope.context::<crate::playground::CompilerEndpoint>() {
                    let _ = config.0.set(origin.to_owned());
                }
            }
        });
    }
    #[wasm_bindgen]
    pub fn allocations() -> String {
        serde_json::to_string(&rustx::allocation::stats()).unwrap()
    }
    #[wasm_bindgen]
    pub fn profile() -> String {
        APP.with(|a| a.borrow().as_ref().unwrap().engine.diagnostics.json())
    }
    #[wasm_bindgen]
    pub fn inspect() -> String {
        APP.with(|a| a.borrow().as_ref().unwrap().inspect())
    }
    #[wasm_bindgen]
    pub fn semantics() -> String {
        APP.with(|a| {
            serde_json::to_string(&a.borrow().as_ref().unwrap().engine.runtime.semantics()).unwrap()
        })
    }
    #[wasm_bindgen]
    pub fn activate(id: f64) {
        let Some(id) = rustx_web::host_id(id) else {
            return;
        };
        APP.with(|a| {
            a.borrow_mut()
                .as_mut()
                .unwrap()
                .engine
                .dispatch(id, EventData::new(EventKind::Click));
        });
    }
    #[wasm_bindgen]
    pub fn focus(id: f64) {
        let Some(id) = rustx_web::host_id(id) else {
            return;
        };
        APP.with(|a| a.borrow_mut().as_mut().unwrap().engine.focus_node(id));
    }
    #[wasm_bindgen]
    pub fn input(id: f64, value: String, composing: bool) {
        let Some(id) = rustx_web::host_id(id) else {
            return;
        };
        APP.with(|a| {
            a.borrow_mut()
                .as_mut()
                .unwrap()
                .engine
                .runtime
                .input(id, value, composing);
        });
    }
    #[wasm_bindgen]
    pub fn dispatch(id: f64, event: &str) -> String {
        let Some(id) = rustx_web::host_id(id) else {
            return "{}".into();
        };
        let Ok(event) = serde_json::from_str::<EventData>(event) else {
            return "{}".into();
        };
        APP.with(|a| {
            let mut a = a.borrow_mut();
            let a = a.as_mut().unwrap();
            let result = a.engine.dispatch(id, event);
            a.engine.invalidate();
            a.tasks.poll();
            serde_json::to_string(&result).unwrap()
        })
    }
    #[wasm_bindgen]
    pub fn edit_sync(id: f64, value: String, anchor: u32, focus: u32, composing: bool) -> bool {
        let Some(id) = rustx_web::host_id(id) else {
            return false;
        };
        APP.with(|a| {
            let mut a = a.borrow_mut();
            let a = a.as_mut().unwrap();
            let changed =
                a.engine
                    .runtime
                    .edit_sync(id, value, anchor as usize, focus as usize, composing);
            // Keep committed hit-test geometry until the next frame. The model
            // epoch already requests layout; clearing boxes here would lose
            // pointer selection moves arriving before that frame.
            a.engine.invalidate();
            changed
        })
    }
    #[wasm_bindgen]
    pub fn edit_value(id: f64) -> String {
        let Some(id) = rustx_web::host_id(id) else {
            return String::new();
        };
        APP.with(|a| {
            a.borrow()
                .as_ref()
                .unwrap()
                .engine
                .runtime
                .edit_value(id)
                .unwrap_or_default()
        })
    }
    /// Read the live input model after a synchronous editing default. Layout
    /// inspection is a committed-frame snapshot and can still hold the prior
    /// caret until the next render.
    #[wasm_bindgen]
    pub fn edit_selection(id: f64) -> String {
        let Some(id) = rustx_web::host_id(id) else {
            return "null".into();
        };
        APP.with(|a| {
            let a = a.borrow();
            let selection = a
                .as_ref()
                .unwrap()
                .engine
                .runtime
                .editor(id)
                .map(|editor| editor.selection_utf16());
            serde_json::to_string(&selection).unwrap()
        })
    }
    #[wasm_bindgen]
    pub fn key_event(id: f64, key: &str, shift: bool, ctrl: bool, alt: bool, meta: bool) -> bool {
        let Some(id) = rustx_web::host_id(id) else {
            return false;
        };
        APP.with(|a| {
            let mut a = a.borrow_mut();
            let a = a.as_mut().unwrap();
            let handled = a.engine.key_target(id, key, shift, ctrl, alt, meta);
            a.tasks.poll();
            handled
        })
    }
    #[wasm_bindgen]
    pub fn blur(id: f64) {
        let Some(id) = rustx_web::host_id(id) else {
            return;
        };
        APP.with(|a| a.borrow_mut().as_mut().unwrap().engine.blur_node(id));
    }
    #[wasm_bindgen]
    pub fn service_commands() -> String {
        APP.with(|a| {
            let a = a.borrow();
            let a = a.as_ref().unwrap();
            a.tasks.poll();
            serde_json::to_string(&a.services.drain()).unwrap()
        })
    }
    #[wasm_bindgen]
    pub fn service_response(version: u32, id: f64, result: &str) -> bool {
        let Some(id) = rustx_web::host_id(id) else {
            return false;
        };
        APP.with(|a| {
            let a = a.borrow();
            let a = a.as_ref().unwrap();
            let accepted = a.services.complete_json(version, id, result);
            a.tasks.poll();
            accepted
        })
    }
    #[wasm_bindgen]
    pub fn service_capabilities(report: &str) -> Result<(), JsValue> {
        let report = serde_json::from_str(report)
            .map_err(|_| JsValue::from_str("Invalid service report"))?;
        APP.with(|a| a.borrow().as_ref().unwrap().services.connect(report))
            .map_err(|_| JsValue::from_str("Service handshake failed"))
    }
    #[wasm_bindgen]
    pub fn scroll(dy: f64) {
        APP.with(|a| a.borrow_mut().as_mut().unwrap().engine.scroll(dy));
    }
    #[wasm_bindgen]
    pub fn commands() -> String {
        APP.with(|a| serde_json::to_string(&a.borrow().as_ref().unwrap().http.drain()).unwrap())
    }
    #[wasm_bindgen]
    pub fn response(id: f64, status: u16, body: Vec<u8>, error: String, version: u32) {
        let Some(id) = rustx_web::host_id(id) else {
            return;
        };
        APP.with(|a| {
            a.borrow_mut()
                .as_mut()
                .unwrap()
                .response(id, status, body, &error, version)
        });
    }
    #[wasm_bindgen]
    pub fn response_meta(
        id: f64,
        status: u16,
        body: Vec<u8>,
        error: String,
        version: u32,
        headers: &str,
    ) -> bool {
        let Some(id) = rustx_web::host_id(id) else {
            return false;
        };
        let headers = serde_json::from_str::<Vec<(String, String)>>(headers).unwrap_or_default();
        APP.with(|a| {
            let mut a = a.borrow_mut();
            let a = a.as_mut().unwrap();
            let accepted = a.response_with_headers(id, status, body, headers, &error, version);
            a.tasks.poll();
            accepted
        })
    }
    #[wasm_bindgen]
    pub fn host_event(event: &str) -> String {
        if event.len() > 65536 {
            return "{}".into();
        }
        let Ok(event) = serde_json::from_str::<EventData>(event) else {
            return "{}".into();
        };
        APP.with(|a| {
            let mut a = a.borrow_mut();
            let a = a.as_mut().unwrap();
            let result = a.engine.host_event(event);
            a.tasks.poll();
            serde_json::to_string(&result).unwrap()
        })
    }
    #[wasm_bindgen]
    pub fn needs_update() -> bool {
        APP.with(|a| a.borrow().as_ref().unwrap().engine.runtime.needs_update())
    }
}
