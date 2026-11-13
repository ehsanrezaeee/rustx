//! Stage 2 conformance scene: mixed fonts, paths, shadow, transformed clip and alpha.
use rustx::prelude::*;
use rustx_engine::{
    cpu::CpuRenderer,
    vello::{
        self,
        peniko::{Blob, Image, ImageFormat},
        util::RenderContext,
        wgpu,
    },
    Engine,
};
use std::{fs::File, sync::Arc};
fn main() {
    let out = std::env::args().nth(1).unwrap_or("docs/evidence".into());
    std::fs::create_dir_all(&out).unwrap();
    let mut e = Engine::new(|_| {
        rustx! {
            <Div style={ Style::column().padding(30.).gap(20.).background(0x172033) }>
                <Text style={ Style::column().font_size(30.) }>
                    { "RustX Stage 2 — سلام دنیا 🦀 🚀" }
                </Text>
                <Div
                    style={
                        Style::column()
                            .height(130.)
                            .clip()
                            .radius(24.)
                            .background(0x425377)
                            .shadow(0x000000, 6., (3., 4.))
                    }
                >
                    <Div
                        style={
                            Style::row()
                                .width(900.)
                                .height(180.)
                                .translate(25., 15.)
                                .gradient(0xc1f392, 0x4b74ca)
                                .opacity(0.6)
                        }
                    >
                        <Text style={ Style::column().padding(20.).font_size(24.).color(0x101820) }>
                            { "Nested clip · translation · opacity group" }
                        </Text>
                    </Div>
                </Div>
                <Image
                    src="probe"
                    style={
                        Style::column()
                            .width(180.)
                            .height(120.)
                            .radius(20.)
                            .opacity(0.65)
                    }
                />
                <Canvas
                    drawing={ rustx_lab::visual::conformance_scene() }
                    style={
                        Style::column()
                            .height(200.)
                            .clip()
                            .radius(16.)
                            .background(0x0b1018)
                    }
                />
            </Div>
        }
    });
    let pixels: Vec<u8> = (0..64)
        .flat_map(|i| {
            if (i / 8 + i % 8) % 2 == 0 {
                [181, 243, 107, 128]
            } else {
                [75, 116, 202, 255]
            }
        })
        .collect();
    e.painter
        .image(
            "probe",
            Image::new(
                Blob::new(Arc::new(pixels.clone())),
                ImageFormat::Rgba8,
                8,
                8,
            ),
        )
        .unwrap();
    e.frame(900., 720., 1.);
    let mut cpu = CpuRenderer::new(900, 720);
    cpu.image("probe", 8, 8, pixels);
    cpu.draw(e.boxes.as_ref().unwrap(), &mut e.text, 900, 720, 1.);
    cpu.pixmap.save_png(format!("{out}/probe-cpu.png")).unwrap();
    println!("Stage 2 CPU shared scene rendered");
    if std::env::args().any(|a| a == "--gpu") {
        pollster::block_on(gpu(&e, &out));
    }
}
async fn gpu(e: &Engine, out: &str) {
    let mut context = RenderContext::new();
    let id = context.device(None).await.expect("GPU adapter unavailable");
    let dev = &context.devices[id];
    println!("Adapter: {:?}", dev.adapter().get_info());
    let mut renderer = vello::Renderer::new(
        &dev.device,
        vello::RendererOptions {
            antialiasing_support: vello::AaSupport::area_only(),
            ..Default::default()
        },
    )
    .unwrap();
    let (w, h) = (900u32, 720u32);
    let texture = dev.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("probe"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    renderer
        .render_to_texture(
            &dev.device,
            &dev.queue,
            &e.scene,
            &texture.create_view(&Default::default()),
            &vello::RenderParams {
                base_color: rustx_engine::color(0x0b1018),
                width: w,
                height: h,
                antialiasing_method: vello::AaConfig::Area,
            },
        )
        .unwrap();
    let stride = (w * 4).div_ceil(256) * 256;
    let buffer = dev.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (stride * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = dev.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
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
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    dev.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    dev.device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range();
    let pixels: Vec<u8> = mapped
        .chunks(stride as usize)
        .flat_map(|row| row[..(w * 4) as usize].iter().copied())
        .collect();
    let mut encoder =
        png::Encoder::new(File::create(format!("{out}/probe-gpu.png")).unwrap(), w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    println!("Offscreen Vello probe passed; not hardware/browser presentation certification");
}
