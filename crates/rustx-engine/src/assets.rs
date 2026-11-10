//! PNG/font installation into the shared Rust renderer. No browser decoding or
//! absolute platform paths are part of the application asset contract.
use crate::Engine;
use rustx_core::{AssetData, AssetError, AssetKind, MAX_ASSET_ENTRIES, MAX_HTTP_BYTES};
use std::{collections::BTreeMap, io::Cursor, sync::Arc};
use vello::peniko::{Blob, Image, ImageFormat};

#[derive(Default)]
pub struct AssetRegistry {
    installed: BTreeMap<String, (String, AssetKind)>,
}
impl AssetRegistry {
    pub fn len(&self) -> usize {
        self.installed.len()
    }
    pub fn is_empty(&self) -> bool {
        self.installed.is_empty()
    }
    pub fn install(&mut self, engine: &mut Engine, asset: AssetData) -> Result<(), AssetError> {
        if asset.bytes.len() > asset.entry.max_bytes || asset.bytes.len() > MAX_HTTP_BYTES {
            return Err(AssetError::ResourceLimit);
        }
        // One logical ID owns one renderer resource kind. In particular, fonts
        // cannot be unloaded by replacing their registry entry with an image.
        if self
            .installed
            .get(&asset.entry.id)
            .is_some_and(|(_, kind)| *kind != asset.entry.kind)
        {
            return Err(AssetError::Unsupported);
        }
        if self.installed.get(&asset.entry.id)
            == Some(&(asset.entry.revision.clone(), asset.entry.kind))
        {
            return Ok(());
        }
        if !self.installed.contains_key(&asset.entry.id)
            && self.installed.len() >= MAX_ASSET_ENTRIES
        {
            return Err(AssetError::ResourceLimit);
        }
        match asset.entry.kind {
            AssetKind::PngImage => {
                let (width, height, pixels) = decode_png(&asset.bytes)?;
                engine
                    .painter
                    .image(
                        &asset.entry.id,
                        Image::new(
                            Blob::new(Arc::new(pixels)),
                            ImageFormat::Rgba8,
                            width,
                            height,
                        ),
                    )
                    .map_err(|_| AssetError::ResourceLimit)?;
            }
            AssetKind::Font => {
                // Font collections are engine-owned. Replacing a font revision
                // requires a fresh engine; no false per-route unload guarantee.
                if self.installed.contains_key(&asset.entry.id) {
                    return Err(AssetError::Unsupported);
                }
                engine
                    .text
                    .add_font(asset.bytes)
                    .map_err(|_| AssetError::Decode)?;
            }
            AssetKind::Bytes => return Err(AssetError::Unsupported),
        }
        self.installed
            .insert(asset.entry.id, (asset.entry.revision, asset.entry.kind));
        engine.invalidate();
        Ok(())
    }
    pub fn remove_image(&mut self, engine: &mut Engine, id: &str) -> Result<(), AssetError> {
        let Some((_, kind)) = self.installed.get(id) else {
            return Err(AssetError::UnknownId);
        };
        if *kind != AssetKind::PngImage {
            return Err(AssetError::Unsupported);
        }
        self.installed.remove(id);
        engine.painter.remove_image(id);
        engine.invalidate();
        Ok(())
    }
}

pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), AssetError> {
    if bytes.len() > MAX_HTTP_BYTES {
        return Err(AssetError::ResourceLimit);
    }
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_limits(png::Limits {
        bytes: 32 * 1024 * 1024,
    });
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|_| AssetError::Decode)?;
    let (width, height) = (reader.info().width, reader.info().height);
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or(AssetError::ResourceLimit)?;
    if width == 0
        || height == 0
        || width > 4096
        || height > 4096
        || pixels > 8 * 1024 * 1024
        || reader.info().animation_control.is_some()
    {
        return Err(AssetError::ResourceLimit);
    }
    if reader.output_buffer_size() > 32 * 1024 * 1024 {
        return Err(AssetError::ResourceLimit);
    }
    let mut decoded = vec![0; reader.output_buffer_size()];
    let output = reader
        .next_frame(&mut decoded)
        .map_err(|_| AssetError::Decode)?;
    let decoded = &decoded[..output.buffer_size()];
    let mut rgba = Vec::with_capacity(pixels * 4);
    match output.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(decoded),
        png::ColorType::Rgb => {
            for c in decoded.chunks_exact(3) {
                rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for c in decoded.chunks_exact(2) {
                rgba.extend_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for g in decoded {
                rgba.extend_from_slice(&[*g, *g, *g, 255]);
            }
        }
        png::ColorType::Indexed => return Err(AssetError::Decode),
    }
    if rgba.len() != pixels * 4 {
        return Err(AssetError::Decode);
    }
    Ok((width, height, rgba))
}
