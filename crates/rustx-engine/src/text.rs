//! Shared Unicode shaping with bundled fonts and bounded shaped-run caching.
use crate::cache::BudgetCache;
use cosmic_text::{fontdb, Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use rustx_core::{TextAlign, TextDirection};
use std::{collections::HashMap, sync::Arc};
use vello::{
    kurbo::Affine,
    peniko::{Blob, Color, Fill, Font},
    Glyph, Scene,
};
#[derive(Clone)]
pub struct GlyphRun {
    pub font: Font,
    pub size: f32,
    pub glyphs: Vec<Glyph>,
}
#[derive(Clone)]
pub struct Shaped {
    pub clusters: Vec<GlyphCluster>,
    pub width: f32,
    pub height: f32,
    pub runs: Vec<GlyphRun>,
    pub missing: usize,
}
#[derive(Clone, Hash, Eq, PartialEq)]
struct Key {
    text: String,
    size: u32,
    width: u32,
    align: u8,
    direction: u8,
}
pub struct TextEngine {
    system: FontSystem,
    fonts: HashMap<fontdb::ID, Font>,
    cache: BudgetCache<Key, Arc<Shaped>>,
    pub epoch: u64,
    font_bytes: usize,
    pub shape_count: usize,
}
impl Default for TextEngine {
    fn default() -> Self {
        let mut db = fontdb::Database::new();
        db.load_font_data(include_bytes!("../assets/DejaVuSans.ttf").to_vec());
        db.load_font_data(include_bytes!("../assets/NotoEmoji.ttf").to_vec());
        db.set_sans_serif_family("DejaVu Sans");
        Self {
            system: FontSystem::new_with_locale_and_db("en-US".into(), db),
            fonts: HashMap::new(),
            cache: BudgetCache::new(512, 8 * 1024 * 1024),
            epoch: 0,
            font_bytes: include_bytes!("../assets/DejaVuSans.ttf").len()
                + include_bytes!("../assets/NotoEmoji.ttf").len(),
            shape_count: 0,
        }
    }
}
impl TextEngine {
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.cache.bytes())
    }
    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }
    pub fn add_font(&mut self, bytes: Vec<u8>) -> Result<(), &'static str> {
        if bytes.len() > 8 * 1024 * 1024
            || self.font_bytes + bytes.len() > 32 * 1024 * 1024
            || self.system.db().faces().count() >= 16
        {
            return Err("font budget exceeded");
        }
        let before_ids: std::collections::HashSet<_> =
            self.system.db().faces().map(|f| f.id).collect();
        let before = before_ids.len();
        let n = bytes.len();
        self.system.db_mut().load_font_data(bytes);
        if self.system.db().faces().count() == before {
            return Err("invalid font");
        }
        if self.system.db().faces().count() > 16 {
            let added: Vec<_> = self
                .system
                .db()
                .faces()
                .filter(|f| !before_ids.contains(&f.id))
                .map(|f| f.id)
                .collect();
            for id in added {
                self.system.db_mut().remove_face(id);
            }
            return Err("font face budget exceeded");
        }
        self.font_bytes += n;
        self.cache.clear();
        self.epoch += 1;
        Ok(())
    }
    pub fn shape(&mut self, text: &str, size: f32, width: f32) -> Arc<Shaped> {
        self.shape_with(text, size, width, TextAlign::Start, TextDirection::Auto)
    }
    pub fn shape_with(
        &mut self,
        text: &str,
        size: f32,
        width: f32,
        align: TextAlign,
        direction: TextDirection,
    ) -> Arc<Shaped> {
        let size = if size.is_finite() {
            size.clamp(1., 256.)
        } else {
            16.
        };
        let width = if width.is_finite() {
            width.clamp(1., 1_000_000.)
        } else {
            1_000_000.
        };
        let key = Key {
            text: text.into(),
            size: size.to_bits(),
            width: width.to_bits(),
            align: align as u8,
            direction: direction as u8,
        };
        if let Some(run) = self.cache.get(&key) {
            return run.clone();
        }
        let mut buffer = Buffer::new(&mut self.system, Metrics::new(size, size * 1.4));
        buffer.set_size(&mut self.system, Some(width), None);
        let directed = match direction {
            TextDirection::Auto => text.to_owned(),
            _ => text
                .lines()
                .map(|line| {
                    format!(
                        "{}{}",
                        if direction == TextDirection::RightToLeft {
                            "\u{200f}"
                        } else {
                            "\u{200e}"
                        },
                        line
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        };
        buffer.set_text(
            &mut self.system,
            &directed,
            &Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
        );
        for line in &mut buffer.lines {
            line.set_align(match align {
                TextAlign::Start => None,
                TextAlign::Center => Some(cosmic_text::Align::Center),
                TextAlign::End => Some(cosmic_text::Align::End),
            });
        }
        buffer.shape_until_scroll(&mut self.system, false);
        let mut shaped = Shaped {
            clusters: vec![],
            width: 0.,
            height: 0.,
            runs: vec![],
            missing: 0,
        };
        for line in buffer.layout_runs() {
            shaped.width = shaped.width.max(line.line_w);
            shaped.height = shaped.height.max(line.line_top + line.line_height);
            let base = text
                .split_inclusive('\n')
                .take(line.line_i)
                .map(str::len)
                .sum::<usize>();
            let prefix = if direction == TextDirection::Auto {
                0
            } else {
                3
            };
            for g in line.glyphs {
                shaped.clusters.push(GlyphCluster {
                    start: base + g.start.saturating_sub(prefix),
                    end: base + g.end.saturating_sub(prefix),
                    x: g.x,
                    y: line.line_top,
                    width: g.w,
                    height: line.line_height,
                    rtl: g.level.is_rtl(),
                });
                let font = self
                    .fonts
                    .entry(g.font_id)
                    .or_insert_with(|| {
                        self.system
                            .db()
                            .with_face_data(g.font_id, |bytes, index| {
                                Font::new(Blob::new(Arc::new(bytes.to_vec())), index)
                            })
                            .expect("loaded font")
                    })
                    .clone();
                if g.glyph_id == 0 {
                    shaped.missing += 1;
                }
                let glyph = Glyph {
                    id: u32::from(g.glyph_id),
                    x: g.x + g.font_size * g.x_offset,
                    y: line.line_y + g.y - g.font_size * g.y_offset,
                };
                if let Some(run) = shaped
                    .runs
                    .last_mut()
                    .filter(|r| r.font.data.id() == font.data.id() && r.size == g.font_size)
                {
                    run.glyphs.push(glyph);
                } else {
                    shaped.runs.push(GlyphRun {
                        font,
                        size: g.font_size,
                        glyphs: vec![glyph],
                    });
                }
            }
        }
        self.shape_count += 1;
        let bytes = key.text.len()
            + shaped
                .runs
                .iter()
                .map(|r| r.glyphs.len() * std::mem::size_of::<Glyph>())
                .sum::<usize>()
            + shaped.clusters.len() * std::mem::size_of::<GlyphCluster>();
        let result = Arc::new(shaped);
        self.cache.insert(key, result.clone(), bytes);
        result
    }
    pub fn paint(scene: &mut Scene, text: &Shaped, transform: Affine, color: Color) {
        for run in &text.runs {
            scene
                .draw_glyphs(&run.font)
                .font_size(run.size)
                .transform(transform)
                .brush(color)
                .draw(Fill::NonZero, run.glyphs.iter().copied());
        }
    }
}

#[derive(Clone, Debug)]
pub struct GlyphCluster {
    pub start: usize,
    pub end: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rtl: bool,
}
pub fn byte_to_utf16(text: &str, byte: usize) -> Option<usize> {
    text.is_char_boundary(byte)
        .then(|| text[..byte].encode_utf16().count())
}
pub fn utf16_to_byte(text: &str, units: usize) -> Option<usize> {
    let mut used = 0;
    for (i, c) in text.char_indices() {
        if used == units {
            return Some(i);
        }
        used += c.len_utf16();
    }
    if used == units {
        Some(text.len())
    } else {
        None
    }
}
