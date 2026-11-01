//! Shared Unicode shaping with bundled fonts and bounded shaped-run caching.
use cosmic_text::{fontdb, Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
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
    pub width: f32,
    pub height: f32,
    pub runs: Vec<GlyphRun>,
    pub missing: usize,
}
#[derive(Hash, Eq, PartialEq)]
struct Key {
    text: String,
    size: u32,
    width: u32,
}
pub struct TextEngine {
    system: FontSystem,
    fonts: HashMap<fontdb::ID, Font>,
    cache: HashMap<Key, Arc<Shaped>>,
    pub shape_count: usize,
}
impl Default for TextEngine {
    fn default() -> Self {
        let mut db = fontdb::Database::new();
        db.load_font_data(include_bytes!("../assets/DejaVuSans.ttf").to_vec());
        db.set_sans_serif_family("DejaVu Sans");
        Self {
            system: FontSystem::new_with_locale_and_db("en-US".into(), db),
            fonts: HashMap::new(),
            cache: HashMap::new(),
            shape_count: 0,
        }
    }
}
impl TextEngine {
    pub fn add_font(&mut self, bytes: Vec<u8>) {
        self.system.db_mut().load_font_data(bytes);
        self.cache.clear();
    }
    pub fn shape(&mut self, text: &str, size: f32, width: f32) -> Arc<Shaped> {
        let width = width.max(1.);
        let key = Key {
            text: text.into(),
            size: size.to_bits(),
            width: width.to_bits(),
        };
        if let Some(run) = self.cache.get(&key) {
            return run.clone();
        }
        let mut buffer = Buffer::new(&mut self.system, Metrics::new(size, size * 1.4));
        buffer.set_size(&mut self.system, Some(width), None);
        buffer.set_text(
            &mut self.system,
            text,
            &Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.system, false);
        let mut shaped = Shaped {
            width: 0.,
            height: 0.,
            runs: vec![],
            missing: 0,
        };
        for line in buffer.layout_runs() {
            shaped.width = shaped.width.max(line.line_w);
            shaped.height = shaped.height.max(line.line_top + line.line_height);
            for g in line.glyphs {
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
        if self.cache.len() >= 512 {
            self.cache.clear();
        }
        let result = Arc::new(shaped);
        self.cache.insert(key, result.clone());
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
