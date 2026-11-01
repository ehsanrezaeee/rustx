#![cfg(not(target_arch = "wasm32"))]
//! GPU lifecycle and presentation, separated from application events and layout.
use crate::Engine;
use std::sync::Arc;
use vello::{
    util::{RenderContext, RenderSurface},
    wgpu, AaConfig, AaSupport, RenderParams, Renderer, RendererOptions,
};
use winit::{dpi::PhysicalSize, window::Window};
pub struct Gpu {
    context: RenderContext,
    renderer: Renderer,
    presenter: Presenter,
}
enum Presenter {
    Surface(Box<RenderSurface<'static>>),
    #[cfg(target_arch = "wasm32")]
    Readback(Readback),
}
#[cfg(target_arch = "wasm32")]
struct Readback {
    context: web_sys::CanvasRenderingContext2d,
    texture: wgpu::Texture,
    size: PhysicalSize<u32>,
    busy: std::rc::Rc<std::cell::Cell<bool>>,
    pending: std::rc::Rc<std::cell::Cell<bool>>,
    window: Arc<Window>,
    dev: usize,
}
impl Gpu {
    pub async fn new(window: Arc<Window>) -> Result<Self, String> {
        let mut context = RenderContext::new();
        let size = crate::host::physical_size(&window);
        let size = PhysicalSize::new(size.width.max(1), size.height.max(1));
        #[cfg(target_arch = "wasm32")]
        if web_sys::window()
            .unwrap()
            .location()
            .search()
            .unwrap_or_default()
            .contains("readback=1")
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowExtWebSys;
            let id = context
                .device(None)
                .await
                .ok_or("No compatible WebGPU device")?;
            let canvas = window.canvas().ok_or("No canvas")?;
            let ctx = canvas
                .get_context("2d")
                .map_err(|_| "Canvas context failed")?
                .ok_or("Canvas context unavailable")?
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .map_err(|_| "Wrong canvas context")?;
            let renderer = Self::renderer(&context.devices[id].device)?;
            let texture = readback_texture(&context.devices[id].device, size);
            return Ok(Self {
                context,
                renderer,
                presenter: Presenter::Readback(Readback {
                    context: ctx,
                    texture,
                    size,
                    busy: Default::default(),
                    pending: Default::default(),
                    window,
                    dev: id,
                }),
            });
        }
        let surface = context
            .create_surface(
                window,
                size.width,
                size.height,
                wgpu::PresentMode::AutoVsync,
            )
            .await
            .map_err(|e| e.to_string())?;
        let renderer = Self::renderer(&context.devices[surface.dev_id].device)?;
        Ok(Self {
            context,
            renderer,
            presenter: Presenter::Surface(Box::new(surface)),
        })
    }
    fn renderer(device: &wgpu::Device) -> Result<Renderer, String> {
        #[cfg(not(target_arch = "wasm32"))]
        device.set_device_lost_callback(|reason, message| {
            crate::host::report_error(&format!("GPU lost ({reason:?}): {message}"))
        });
        device.on_uncaptured_error(Box::new(|e| {
            crate::host::report_error(&format!("GPU: {e}"))
        }));
        Renderer::new(
            device,
            RendererOptions {
                antialiasing_support: AaSupport::area_only(),
                use_cpu: compatibility_preprocessing(),
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())
    }
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        match &mut self.presenter {
            Presenter::Surface(surface) => {
                if surface.config.width != size.width || surface.config.height != size.height {
                    self.context
                        .resize_surface(surface, size.width, size.height);
                }
            }
            #[cfg(target_arch = "wasm32")]
            Presenter::Readback(r) => {
                if let Some(canvas) = r.context.canvas() {
                    if canvas.width() != size.width {
                        canvas.set_width(size.width);
                    }
                    if canvas.height() != size.height {
                        canvas.set_height(size.height);
                    }
                }
                if r.size == size {
                    return;
                }
                r.size = size;
                r.texture = readback_texture(&self.context.devices[r.dev].device, size);
            }
        }
    }
    pub fn draw(&mut self, engine: &Engine) -> Result<(), String> {
        match &mut self.presenter {
            Presenter::Surface(surface) => {
                let frame = match surface.surface.get_current_texture() {
                    Ok(f) => f,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.context.resize_surface(
                            surface,
                            surface.config.width,
                            surface.config.height,
                        );
                        return Ok(());
                    }
                    Err(wgpu::SurfaceError::Timeout) => return Ok(()),
                    Err(e) => return Err(e.to_string()),
                };
                let dev = &self.context.devices[surface.dev_id];
                self.renderer
                    .render_to_texture(
                        &dev.device,
                        &dev.queue,
                        &engine.scene,
                        &surface.target_view,
                        &params(surface.config.width, surface.config.height),
                    )
                    .map_err(|e| e.to_string())?;
                let mut encoder = dev.device.create_command_encoder(&Default::default());
                surface.blitter.copy(
                    &dev.device,
                    &mut encoder,
                    &surface.target_view,
                    &frame.texture.create_view(&Default::default()),
                );
                dev.queue.submit([encoder.finish()]);
                frame.present();
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            Presenter::Readback(r) => {
                if r.busy.replace(true) {
                    r.pending.set(true);
                    return Ok(());
                }
                let dev = &self.context.devices[r.dev];
                let size = r.size;
                if let Err(e) = self.renderer.render_to_texture(
                    &dev.device,
                    &dev.queue,
                    &engine.scene,
                    &r.texture.create_view(&Default::default()),
                    &params(size.width, size.height),
                ) {
                    r.busy.set(false);
                    return Err(e.to_string());
                }
                let stride = (size.width * 4).div_ceil(256) * 256;
                let buffer = dev.device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: u64::from(stride * size.height),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut encoder = dev.device.create_command_encoder(&Default::default());
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: &r.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(stride),
                            rows_per_image: None,
                        },
                    },
                    wgpu::Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    },
                );
                dev.queue.submit([encoder.finish()]);
                let busy = r.busy.clone();
                let pending = r.pending.clone();
                let window = r.window.clone();
                let ctx = r.context.clone();
                let copy = buffer.clone();
                buffer
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, move |result| {
                        if result.is_ok() {
                            let view = copy.slice(..).get_mapped_range();
                            let bytes: Vec<_> = view
                                .chunks(stride as usize)
                                .flat_map(|row| row[..size.width as usize * 4].iter().copied())
                                .collect();
                            if let Ok(image) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
                                wasm_bindgen::Clamped(&bytes),
                                size.width,
                                size.height,
                            ) {
                                let _ = ctx.put_image_data(&image, 0., 0.);
                            }
                            drop(view);
                            copy.unmap();
                        } else {
                            crate::host::report_error("GPU readback failed");
                        }
                        busy.set(false);
                        if pending.replace(false) {
                            window.request_redraw();
                        }
                    });
                Ok(())
            }
        }
    }
}
fn compatibility_preprocessing() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .unwrap()
            .location()
            .search()
            .unwrap_or_default()
            .contains("readback=1")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}
fn params(width: u32, height: u32) -> RenderParams {
    RenderParams {
        base_color: crate::color(0x0b1018),
        width,
        height,
        antialiasing_method: AaConfig::Area,
    }
}
#[cfg(target_arch = "wasm32")]
fn readback_texture(device: &wgpu::Device, size: PhysicalSize<u32>) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Compatibility presentation"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
