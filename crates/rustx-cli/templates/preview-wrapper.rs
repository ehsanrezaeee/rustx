//! Trusted CPU-only preview ABI. Visitor code lives in main.rustx.
#[path = "main.rustx"]
mod app;
use rustx::prelude::*;
use rustx_web::WebApp;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
thread_local! {static APP:RefCell<Option<WebApp>>=const{RefCell::new(None)};}
fn with<T>(f: impl FnOnce(&mut WebApp) -> T) -> T {
    APP.with(|a| f(a.borrow_mut().as_mut().expect("preview initialized")))
}
#[wasm_bindgen]
pub fn init_app(report: &str) -> Result<(), JsValue> {
    let capabilities =
        serde_json::from_str(report).map_err(|_| JsValue::from_str("Invalid capabilities"))?;
    let http = HttpClient::default();
    http.connect(capabilities)
        .map_err(|_| JsValue::from_str("HTTP unavailable"))?;
    let services = ServiceClient::default();
    services
        .connect(ServiceCapabilities {
            version: SERVICE_VERSION,
            clipboard: CapabilityState::Unavailable,
            files: CapabilityState::Unavailable,
            storage: CapabilityState::Unavailable,
            navigation: CapabilityState::Unavailable,
            websocket: CapabilityState::Unavailable,
            stream: CapabilityState::Unavailable,
            scheduling: CapabilityState::Unavailable,
            print: CapabilityState::Unavailable,
            max_bytes: 1048576,
            max_resources: 32,
        })
        .map_err(|_| JsValue::from_str("Invalid services"))?;
    let tasks = TaskQueue::default();
    let h = http.clone();
    let s = services.clone();
    let t = tasks.clone();
    let engine = rustx_engine::Engine::new(move |cx| {
        cx.provide(h.clone());
        cx.provide(s.clone());
        cx.provide(t.clone());
        app::app(cx)
    });
    let mut web = WebApp::new(engine, http);
    web.services = services;
    web.tasks = tasks;
    APP.with(|a| a.replace(Some(web)));
    Ok(())
}
#[wasm_bindgen]
pub fn frame(w: f32, h: f32, dpr: f32) -> bool {
    with(|a| a.frame(w, h, dpr))
}
#[wasm_bindgen]
pub fn pixels() -> Vec<u8> {
    with(|a| a.cpu.pixmap.data().to_vec())
}
#[wasm_bindgen]
pub fn inspect() -> String {
    with(|a| a.inspect())
}
#[wasm_bindgen]
pub fn semantics() -> String {
    with(|a| serde_json::to_string(&a.engine.runtime.semantics()).unwrap())
}
#[wasm_bindgen]
pub fn needs_update() -> bool {
    with(|a| a.engine.runtime.needs_update())
}
#[wasm_bindgen]
pub fn commands() -> String {
    with(|a| serde_json::to_string(&a.http.drain()).unwrap())
}
#[wasm_bindgen]
pub fn response_meta(
    id: f64,
    status: u16,
    body: Vec<u8>,
    error: &str,
    version: u32,
    headers: &str,
) -> bool {
    let Some(id) = rustx_web::host_id(id) else {
        return false;
    };
    let headers = serde_json::from_str(headers).unwrap_or_default();
    with(|a| {
        let result = a.response_with_headers(id, status, body, headers, error, version);
        a.tasks.poll();
        result
    })
}
#[wasm_bindgen]
pub fn activate(id: f64) {
    if let Some(id) = rustx_web::host_id(id) {
        with(|a| {
            a.engine.runtime.click(id, 0., 0.);
            a.engine.invalidate();
        });
    }
}
#[wasm_bindgen]
pub fn focus(id: f64) {
    if let Some(id) = rustx_web::host_id(id) {
        with(|a| a.engine.focus_node(id));
    }
}
#[wasm_bindgen]
pub fn blur(id: f64) {
    if let Some(id) = rustx_web::host_id(id) {
        with(|a| a.engine.blur_node(id));
    }
}
#[wasm_bindgen]
pub fn dispatch(id: f64, json: &str) -> String {
    let Some(id) = rustx_web::host_id(id) else {
        return "{}".into();
    };
    let Ok(event) = serde_json::from_str(json) else {
        return "{}".into();
    };
    with(|a| serde_json::to_string(&a.engine.dispatch(id, event)).unwrap())
}
#[wasm_bindgen]
pub fn edit_sync(id: f64, value: String, anchor: u32, focus: u32, composing: bool) -> bool {
    let Some(id) = rustx_web::host_id(id) else {
        return false;
    };
    with(|a| {
        let r = a
            .engine
            .runtime
            .edit_sync(id, value, anchor as usize, focus as usize, composing);
        a.engine.invalidate();
        r
    })
}
#[wasm_bindgen]
pub fn edit_value(id: f64) -> String {
    let Some(id) = rustx_web::host_id(id) else {
        return String::new();
    };
    with(|a| a.engine.runtime.edit_value(id).unwrap_or_default())
}
#[wasm_bindgen]
pub fn scroll(delta: f64) {
    with(|a| a.engine.scroll(delta));
}
#[wasm_bindgen]
pub fn scroll_node(id: f64, delta: f64) -> bool {
    let Some(id) = rustx_web::host_id(id) else {
        return false;
    };
    with(|a| {
        let handled = a.engine.runtime.scroll(id, delta);
        if handled {
            a.engine.invalidate();
        }
        handled
    })
}
#[wasm_bindgen]
pub fn canvas_pan(id: f64, dx: f64, dy: f64) {
    if let Some(id) = rustx_web::host_id(id) {
        with(|a| {
            a.engine
                .canvas_event(id, rustx::drawing::CanvasEvent::Pan { dx, dy });
            a.engine.invalidate();
        });
    }
}
#[wasm_bindgen]
pub fn canvas_pick(id: f64, x: f64, y: f64) {
    if let Some(id) = rustx_web::host_id(id) {
        with(|a| {
            if let Some(rect) = a.engine.canvas_at(id) {
                a.engine.canvas_event(
                    id,
                    rustx::drawing::CanvasEvent::Pick {
                        x: x - rect.x0,
                        y: y - rect.y0,
                    },
                );
                a.engine.invalidate();
            }
        });
    }
}
#[wasm_bindgen]
pub fn canvas_zoom(id: f64, x: f64, y: f64, factor: f64) {
    if let Some(id) = rustx_web::host_id(id) {
        with(|a| {
            if let Some(rect) = a.engine.canvas_at(id) {
                a.engine.canvas_event(
                    id,
                    rustx::drawing::CanvasEvent::Zoom {
                        x: x - rect.x0,
                        y: y - rect.y0,
                        factor,
                    },
                );
                a.engine.invalidate();
            }
        });
    }
}
