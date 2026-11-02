#[path = "app.rustx"]
mod app;
#[path = "visual.rustx"]
pub mod visual;
use rustx::prelude::*;
pub fn start_page() -> i32 {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("RUSTX_DEMO").as_deref() == Ok("engine") {
        return 3;
    }
    0
}
pub fn engine() -> (rustx_engine::Engine, HttpClient) {
    engine_with_host(HostCapabilities::get_cancel()).expect("native GET/cancel host")
}
pub fn engine_with_host(
    capabilities: HostCapabilities,
) -> Result<(rustx_engine::Engine, HttpClient), HostError> {
    let h = HttpClient::default();
    h.connect(capabilities)?;
    let client = h.clone();
    Ok((
        rustx_engine::Engine::new(move |cx| app::root(cx, client.clone())),
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
        let (e, h) = engine_with_host(capabilities)
            .map_err(|e| JsValue::from_str(&format!("Host handshake failed: {e:?}")))?;
        GPU.with(|g| g.replace(None));
        APP.with(|a| a.replace(Some(WebApp::new(e, h))));
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
                .runtime
                .click(id, 0., 0.);
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
    pub fn needs_update() -> bool {
        APP.with(|a| a.borrow().as_ref().unwrap().engine.runtime.needs_update())
    }
}
