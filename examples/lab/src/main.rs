#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let (e, http) = rustx_lab::engine();
    rustx_engine::host::run_with_http(e, http).expect("native host failed");
}
#[cfg(target_arch = "wasm32")]
fn main() {}
