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
    pub services: rustx_core::ServiceClient,
    pub tasks: rustx_core::TaskQueue,
}
impl WebApp {
    pub fn new(engine: Engine, http: HttpClient) -> Self {
        Self {
            engine,
            cpu: CpuRenderer::new(1, 1),
            http,
            services: rustx_core::ServiceClient::default(),
            tasks: rustx_core::TaskQueue::default(),
        }
    }
    pub fn frame(&mut self, w: f32, h: f32, dpr: f32) -> bool {
        self.tasks.poll();
        let dirty = self.engine.frame(w, h, dpr as f64);
        if dirty {
            self.cpu.sync_images(&self.engine.painter);
            self.cpu.clear_color = self.engine.layout.theme.background;
            if let Some(b) = &self.engine.boxes {
                self.cpu.draw(
                    b,
                    &mut self.engine.text,
                    (w * dpr).round().max(1.) as u32,
                    (h * dpr).round().max(1.) as u32,
                    dpr,
                );
                self.cpu.focus(b, self.engine.focused_node(), dpr);
                self.cpu
                    .editing(b, self.engine.focused_node(), &mut self.engine.text, dpr);
            }
        }
        dirty
    }
    pub fn inspect(&self) -> String {
        let mut nodes = vec![];
        fn walk(
            b: &rustx_engine::layout::BoxNode,
            nodes: &mut Vec<serde_json::Value>,
            clip: rustx_engine::vello::kurbo::Rect,
            scroll: Option<u64>,
        ) {
            let local_clip = if b.node.style.clip {
                clip.intersect(b.rect)
            } else {
                clip
            };
            let scroll = if b.node.role == Some("list") {
                Some(b.node.id)
            } else {
                scroll
            };
            let visible = b.rect.intersect(local_clip);
            nodes.push(serde_json::json!({"id":b.node.app_id,"node":b.node.id,"kind":format!("{:?}",b.node.kind),"role":b.node.role,"text":b.node.text_content(),"editing":b.node.editing,"rect":[b.rect.x0,b.rect.y0,b.rect.width(),b.rect.height()],"visible_rect":[visible.x0,visible.y0,visible.width().max(0.),visible.height().max(0.)],"scroll_parent":scroll,"position":b.node.position,"drawing_count":b.node.drawing.as_ref().map_or(0,|d|d.len())}));
            for child in &b.children {
                walk(child, nodes, local_clip, scroll);
            }
        }
        let (w, h) = self.engine.viewport();
        if let Some(b) = &self.engine.boxes {
            walk(
                b,
                &mut nodes,
                rustx_engine::vello::kurbo::Rect::new(0., 0., w as f64, h as f64),
                None,
            );
        }
        let text_cache = self.engine.text.cache_stats();
        let resources = self.engine.painter.resource_stats();
        let glyphs = self.cpu.glyph_stats();
        serde_json::json!({"nodes":nodes,"focused":self.engine.focused_node(),"service_pending":self.services.pending(),"services":self.services.capabilities(),"tasks":self.tasks.pending(),"counts":self.engine.runtime.counts(),"pending":self.http.pending(),"host":self.http.capabilities(),"queued_commands":self.http.queued(),"theme":format!("{:06x}",self.engine.layout.theme.background),"metrics":{"layout_nodes":self.engine.layout.node_count(),"layout_changed":self.engine.layout.changed,"shapes_visible":self.engine.painter.drawing_visible,"scene_painted":self.engine.painter.painted,"scene_reused":self.engine.painter.reused,"text_shapes":self.engine.text.shape_count,"text_cache_entries":text_cache.0,"text_cache_bytes":text_cache.1,"image_entries":resources.0,"image_bytes":resources.1,"scene_cache_entries":resources.2,"scene_cache_bytes":resources.3,"glyph_entries":glyphs.0,"glyph_bytes":glyphs.1,"cpu_visited":self.cpu.visited,"cpu_culled":self.cpu.culled}}).to_string()
    }
    pub fn response(&mut self, id: u64, status: u16, body: Vec<u8>, error: &str, version: u32) {
        self.response_with_headers(id, status, body, vec![], error, version);
    }
    pub fn response_with_headers(
        &mut self,
        id: u64,
        status: u16,
        body: Vec<u8>,
        headers: Vec<(String, String)>,
        error: &str,
        version: u32,
    ) -> bool {
        let result = match error {
            "" => Ok(HttpResponse {
                status,
                body,
                headers,
            }),
            "cancelled" => Err(HostError::Cancelled),
            "timeout" => Err(HostError::Timeout),
            "too_large" => Err(HostError::TooLarge),
            "busy" => Err(HostError::Busy),
            e => Err(HostError::Transport(e.into())),
        };
        self.http.complete(version, id, result)
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
        pub fn destroy(&self) {
            self.context.devices[self.surface.dev_id].device.destroy();
        }
        pub fn draw(&mut self, e: &Engine, w: u32, h: u32) -> Result<(), String> {
            if self.surface.config.width != w || self.surface.config.height != h {
                self.context
                    .resize_surface(&mut self.surface, w.max(1), h.max(1));
            }
            let frame = match self.surface.surface.get_current_texture() {
                Ok(f) => f,
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    self.context
                        .resize_surface(&mut self.surface, w.max(1), h.max(1));
                    self.surface
                        .surface
                        .get_current_texture()
                        .map_err(|e| e.to_string())?
                }
                Err(e) => return Err(e.to_string()),
            };
            let d = &self.context.devices[self.surface.dev_id];
            self.renderer
                .render_to_texture(
                    &d.device,
                    &d.queue,
                    &e.scene,
                    &self.surface.target_view,
                    &RenderParams {
                        base_color: rustx_engine::color(e.layout.theme.background),
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
