#[path = "app.rustx"]
mod app;
use rustx::prelude::*;
pub fn engine() -> (rustx_engine::Engine, HttpClient) {
    let h = HttpClient::default();
    let client = h.clone();
    (
        rustx_engine::Engine::new(move |cx| app::root(cx, client.clone())),
        h,
    )
}
#[cfg(target_arch = "wasm32")]
mod browser {
    use super::*;
    use rustx_web::{gpu::WebGpu, WebApp};
    use std::cell::RefCell;
    use wasm_bindgen::prelude::*;
    thread_local! {static APP:RefCell<Option<WebApp>>=const{RefCell::new(None)};static GPU:RefCell<Option<WebGpu>>=const{RefCell::new(None)};}
    #[wasm_bindgen]
    pub fn init_app() {
        let (e, h) = engine();
        APP.with(|a| a.replace(Some(WebApp::new(e, h))));
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
    pub fn response(id: f64, status: u16, body: Vec<u8>, error: String) {
        let Some(id) = rustx_web::host_id(id) else {
            return;
        };
        APP.with(|a| {
            a.borrow_mut()
                .as_mut()
                .unwrap()
                .response(id, status, body, &error)
        });
    }
    #[wasm_bindgen]
    pub fn needs_update() -> bool {
        APP.with(|a| a.borrow().as_ref().unwrap().engine.runtime.needs_update())
    }
}
