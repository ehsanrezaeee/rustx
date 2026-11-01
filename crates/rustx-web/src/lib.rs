pub fn host_id(value: f64) -> Option<u64> {
    (value.is_finite()
        && value >= 1.
        && value <= rustx_core::MAX_HOST_ID as f64
        && value.fract() == 0.)
        .then_some(value as u64)
}
#[test]
fn host_ids_are_exact() {
    for bad in [
        f64::NAN,
        f64::INFINITY,
        -1.,
        0.,
        1.5,
        9_007_199_254_740_992.,
    ] {
        assert_eq!(host_id(bad), None);
    }
    assert_eq!(host_id(4_294_967_300.), Some(4_294_967_300));
}
use rustx_core::{HostError, HttpClient, HttpResponse};
use rustx_engine::{cpu::CpuRenderer, Engine};
pub struct WebApp {
    pub engine: Engine,
    pub cpu: CpuRenderer,
    pub http: HttpClient,
}
impl WebApp {
    pub fn new(engine: Engine, http: HttpClient) -> Self {
        Self {
            engine,
            cpu: CpuRenderer::new(1, 1),
            http,
        }
    }
    pub fn frame(&mut self, w: f32, h: f32, dpr: f32) -> bool {
        let dirty = self.engine.frame(w, h, dpr as f64);
        if dirty {
            if let Some(b) = &self.engine.boxes {
                self.cpu.draw(
                    b,
                    &mut self.engine.text,
                    (w * dpr).round().max(1.) as u32,
                    (h * dpr).round().max(1.) as u32,
                    dpr,
                );
                self.cpu.focus(b, self.engine.focused_node(), dpr);
            }
        }
        dirty
    }
    pub fn inspect(&self) -> String {
        let mut nodes = vec![];
        fn walk(b: &rustx_engine::layout::BoxNode, nodes: &mut Vec<serde_json::Value>) {
            nodes.push(serde_json::json!({"id":b.node.app_id,"node":b.node.id,"text":b.node.text_content(),"rect":[b.rect.x0,b.rect.y0,b.rect.width(),b.rect.height()]}));
            for c in &b.children {
                walk(c, nodes)
            }
        }
        if let Some(b) = &self.engine.boxes {
            walk(b, &mut nodes)
        }
        serde_json::json!({"nodes":nodes,"counts":self.engine.runtime.counts(),"pending":self.http.pending()}).to_string()
    }
    pub fn response(&mut self, id: u64, status: u16, body: Vec<u8>, error: &str) {
        let result = match error {
            "" => Ok(HttpResponse { status, body }),
            "cancelled" => Err(HostError::Cancelled),
            "timeout" => Err(HostError::Timeout),
            "too_large" => Err(HostError::TooLarge),
            e => Err(HostError::Transport(e.into())),
        };
        self.http.complete(id, result);
    }
}
#[cfg(target_arch = "wasm32")]
pub use web_sys::HtmlCanvasElement;
#[cfg(target_arch = "wasm32")]
pub mod gpu {
    use rustx_engine::{
        vello::{
            util::{RenderContext, RenderSurface},
            wgpu, AaConfig, AaSupport, RenderParams, Renderer, RendererOptions,
        },
        Engine,
    };
    pub struct WebGpu {
        context: RenderContext,
        surface: RenderSurface<'static>,
        renderer: Renderer,
    }
    impl WebGpu {
        pub async fn new(canvas: web_sys::HtmlCanvasElement) -> Result<Self, String> {
            let mut context = RenderContext::new();
            let surface = context
                .create_surface(
                    wgpu::SurfaceTarget::Canvas(canvas),
                    1,
                    1,
                    wgpu::PresentMode::AutoVsync,
                )
                .await
                .map_err(|e| e.to_string())?;
            let renderer = Renderer::new(
                &context.devices[surface.dev_id].device,
                RendererOptions {
                    antialiasing_support: AaSupport::area_only(),
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
            Ok(Self {
                context,
                surface,
                renderer,
            })
        }
        pub fn draw(&mut self, e: &Engine, w: u32, h: u32) -> Result<(), String> {
            if self.surface.config.width != w || self.surface.config.height != h {
                self.context
                    .resize_surface(&mut self.surface, w.max(1), h.max(1));
            }
            let frame = self
                .surface
                .surface
                .get_current_texture()
                .map_err(|e| e.to_string())?;
            let d = &self.context.devices[self.surface.dev_id];
            self.renderer
                .render_to_texture(
                    &d.device,
                    &d.queue,
                    &e.scene,
                    &self.surface.target_view,
                    &RenderParams {
                        base_color: rustx_engine::color(0x0b1018),
                        width: w,
                        height: h,
                        antialiasing_method: AaConfig::Area,
                    },
                )
                .map_err(|e| e.to_string())?;
            let mut enc = d.device.create_command_encoder(&Default::default());
            self.surface.blitter.copy(
                &d.device,
                &mut enc,
                &self.surface.target_view,
                &frame.texture.create_view(&Default::default()),
            );
            d.queue.submit([enc.finish()]);
            frame.present();
            Ok(())
        }
    }
}
