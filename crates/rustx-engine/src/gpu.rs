#![cfg(not(target_arch = "wasm32"))]
//! Direct native presentation. Failures return to the host's CPU recovery path.
use crate::Engine;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use vello::{
    util::{RenderContext, RenderSurface},
    wgpu, AaConfig, AaSupport, RenderParams, Renderer, RendererOptions,
};
use winit::{dpi::PhysicalSize, window::Window};
pub struct Gpu {
    context: RenderContext,
    renderer: Renderer,
    surface: RenderSurface<'static>,
    lost: Arc<AtomicBool>,
}
impl Gpu {
    pub async fn new(window: Arc<Window>) -> Result<Self, String> {
        let wake = Arc::downgrade(&window);
        let mut context = RenderContext::new();
        let size = window.inner_size();
        let surface = context
            .create_surface(
                window,
                size.width.max(1),
                size.height.max(1),
                wgpu::PresentMode::AutoVsync,
            )
            .await
            .map_err(|e| e.to_string())?;
        let lost = Arc::new(AtomicBool::new(false));
        let flag = lost.clone();
        let device = &context.devices[surface.dev_id].device;
        let wake_error = wake.clone();
        device.set_device_lost_callback(move |_, _| {
            flag.store(true, Ordering::Release);
            if let Some(w) = wake.upgrade() {
                w.request_redraw();
            }
        });
        let flag = lost.clone();
        device.on_uncaptured_error(Box::new(move |_| {
            flag.store(true, Ordering::Release);
            if let Some(w) = wake_error.upgrade() {
                w.request_redraw();
            }
        }));
        let renderer = Renderer::new(
            device,
            RendererOptions {
                antialiasing_support: AaSupport::area_only(),
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            context,
            renderer,
            surface,
            lost,
        })
    }
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0
            && size.height > 0
            && (self.surface.config.width != size.width
                || self.surface.config.height != size.height)
        {
            self.context
                .resize_surface(&mut self.surface, size.width, size.height);
        }
    }
    pub fn draw(&mut self, engine: &Engine) -> Result<(), String> {
        if self.lost.load(Ordering::Acquire) {
            return Err("GPU device lost".into());
        }
        let frame = match self.surface.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let (width, height) = (self.surface.config.width, self.surface.config.height);
                self.context
                    .resize_surface(&mut self.surface, width, height);
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
                &engine.scene,
                &self.surface.target_view,
                &RenderParams {
                    base_color: crate::color(engine.layout.theme.background),
                    width: self.surface.config.width,
                    height: self.surface.config.height,
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
