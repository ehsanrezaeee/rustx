#[path = "app.rustx"]
pub mod app;

rustx_web::export_app!(crate::app::app);

#[cfg(not(target_arch = "wasm32"))]
pub fn native() {
    let http = rustx::HttpClient::default();
    http.connect(rustx::HostCapabilities::get_cancel()).unwrap();
    let services = rustx::ServiceClient::default();
    let tasks = rustx::TaskQueue::default();
    let host_http = http.clone();
    let host_services = services.clone();
    let host_tasks = tasks.clone();
    let engine = rustx_engine::Engine::new(move |cx| {
        cx.provide(host_http.clone());
        cx.provide(host_services.clone());
        cx.provide(host_tasks.clone());
        app::app(cx)
    });
    rustx_engine::host::run_with_services(engine, http, services, tasks)
        .expect("Native host failed");
}
