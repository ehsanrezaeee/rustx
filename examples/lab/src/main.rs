#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let services = rustx::ServiceClient::default();
    let tasks = rustx::TaskQueue::default();
    let (e, http) = rustx_lab::engine_with_environment(
        rustx::HostCapabilities::get_cancel(),
        services.clone(),
        tasks.clone(),
    )
    .expect("native services");
    rustx_engine::host::run_with_services(e, http, services, tasks).expect("native host failed");
}
#[cfg(target_arch = "wasm32")]
fn main() {}
