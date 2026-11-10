//! Actual Rust PNG decoding and renderer admission; no browser/GPU claim.
use rustx_core::{AssetData, AssetEntry, AssetError, AssetKind, Node};
use rustx_engine::{
    assets::{decode_png, AssetRegistry},
    Engine,
};

fn png_bytes(width: u32, color: png::ColorType, depth: png::BitDepth, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![];
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, 1);
        encoder.set_color(color);
        encoder.set_depth(depth);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
    }
    bytes
}

fn asset(id: &str, revision: &str, bytes: Vec<u8>) -> AssetData {
    AssetData {
        entry: AssetEntry {
            id: id.into(),
            url: "/assets/test.png".into(),
            revision: revision.into(),
            kind: AssetKind::PngImage,
            max_bytes: 1_048_576,
        },
        bytes,
    }
}

#[test]
fn png_decoder_expands_color_alpha_palette_and_sixteen_bit_samples() {
    for (color, depth, input, expected) in [
        (
            png::ColorType::Rgba,
            png::BitDepth::Eight,
            vec![10, 20, 30, 40],
            vec![10, 20, 30, 40],
        ),
        (
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            vec![10, 20, 30],
            vec![10, 20, 30, 255],
        ),
        (
            png::ColorType::GrayscaleAlpha,
            png::BitDepth::Eight,
            vec![80, 120],
            vec![80, 80, 80, 120],
        ),
        (
            png::ColorType::Grayscale,
            png::BitDepth::Eight,
            vec![90],
            vec![90, 90, 90, 255],
        ),
        (
            png::ColorType::Grayscale,
            png::BitDepth::Sixteen,
            vec![0x90, 0xab],
            vec![0x90, 0x90, 0x90, 255],
        ),
    ] {
        let bytes = png_bytes(1, color, depth, &input);
        assert_eq!(decode_png(&bytes), Ok((1, 1, expected)));
    }
    let mut palette = vec![];
    {
        let mut encoder = png::Encoder::new(&mut palette, 2, 1);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(vec![255, 0, 0, 0, 255, 0]);
        encoder.set_trns(vec![0, 128]);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[0, 1])
            .unwrap();
    }
    assert_eq!(
        decode_png(&palette),
        Ok((2, 1, vec![255, 0, 0, 0, 0, 255, 0, 128]))
    );
}

#[test]
fn png_decoder_rejects_malformed_truncated_and_oversized_images() {
    assert_eq!(decode_png(b"not a PNG"), Err(AssetError::Decode));
    let mut valid = png_bytes(1, png::ColorType::Rgba, png::BitDepth::Eight, &[1, 2, 3, 4]);
    valid.truncate(valid.len() / 2);
    assert_eq!(decode_png(&valid), Err(AssetError::Decode));
    let wide = png_bytes(
        4097,
        png::ColorType::Grayscale,
        png::BitDepth::Eight,
        &vec![0; 4097],
    );
    assert_eq!(decode_png(&wide), Err(AssetError::ResourceLimit));
    assert_eq!(
        decode_png(&vec![0; 1_048_577]),
        Err(AssetError::ResourceLimit)
    );
}

#[test]
fn asset_registry_replaces_revisions_removes_images_and_rolls_back_failed_installs() {
    let mut engine = Engine::new(|_| Node::text("Assets"));
    let mut registry = AssetRegistry::default();
    let first = asset(
        "icon",
        "one",
        png_bytes(1, png::ColorType::Rgba, png::BitDepth::Eight, &[1, 2, 3, 4]),
    );
    registry.install(&mut engine, first.clone()).unwrap();
    registry.install(&mut engine, first).unwrap();
    assert_eq!(registry.len(), 1);
    assert_eq!(engine.painter.resource_stats().0, 1);
    let replacement = asset(
        "icon",
        "two",
        png_bytes(
            2,
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            &[1, 2, 3, 4, 5, 6],
        ),
    );
    registry.install(&mut engine, replacement).unwrap();
    assert_eq!(engine.painter.resource_stats().1, 8);
    assert_eq!(
        registry.install(&mut engine, asset("icon", "three", vec![0])),
        Err(AssetError::Decode)
    );
    assert_eq!(registry.len(), 1);
    assert_eq!(
        engine.painter.resource_stats().1,
        8,
        "failed replacement must retain the old image"
    );
    registry.remove_image(&mut engine, "icon").unwrap();
    assert!(registry.is_empty());
    assert_eq!(engine.painter.resource_stats().0, 0);
    assert_eq!(
        registry.remove_image(&mut engine, "icon"),
        Err(AssetError::UnknownId)
    );

    let bytes = png_bytes(
        1,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &[1, 2, 3, 255],
    );
    for index in 0..64 {
        registry
            .install(
                &mut engine,
                asset(&format!("image-{index}"), "one", bytes.clone()),
            )
            .unwrap();
    }
    assert_eq!(
        registry.install(&mut engine, asset("overflow", "one", bytes)),
        Err(AssetError::ResourceLimit)
    );
    assert_eq!(registry.len(), 64);
    assert_eq!(engine.painter.resource_stats().0, 64);
}

#[test]
fn asset_registry_rejects_resource_kind_changes_without_orphaning_resources() {
    let mut engine = Engine::new(|_| Node::text("Assets"));
    let mut registry = AssetRegistry::default();
    let image_bytes = png_bytes(
        1,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &[1, 2, 3, 255],
    );
    let font_bytes = include_bytes!("../assets/DejaVuSans.ttf").to_vec();
    let initial_fonts = engine.text.font_count();

    registry
        .install(&mut engine, asset("icon", "one", image_bytes.clone()))
        .unwrap();
    let mut font = asset("icon", "two", font_bytes.clone());
    font.entry.kind = AssetKind::Font;
    assert_eq!(
        registry.install(&mut engine, font),
        Err(AssetError::Unsupported)
    );
    assert_eq!(engine.text.font_count(), initial_fonts);
    assert_eq!(engine.painter.resource_stats().0, 1);
    registry.remove_image(&mut engine, "icon").unwrap();
    assert!(registry.is_empty());

    let mut font = asset("body", "one", font_bytes);
    font.entry.kind = AssetKind::Font;
    registry.install(&mut engine, font).unwrap();
    let installed_fonts = engine.text.font_count();
    assert_eq!(
        registry.install(&mut engine, asset("body", "two", image_bytes)),
        Err(AssetError::Unsupported)
    );
    assert_eq!(registry.len(), 1);
    assert_eq!(engine.text.font_count(), installed_fonts);
    assert_eq!(engine.painter.resource_stats().0, 0);
    assert_eq!(
        registry.remove_image(&mut engine, "body"),
        Err(AssetError::Unsupported)
    );
}
