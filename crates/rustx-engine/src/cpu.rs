//! Explicit software backend; layout and shaped glyph positions stay in Rust.
use crate::{layout::BoxNode, text::TextEngine};
use rustx_core::Kind;
use std::collections::HashMap;
use tiny_skia::*;
use vello::kurbo::{PathEl, RoundedRect, Shape};
pub struct CpuRenderer {
    pub pixmap: Pixmap,
    font: fontdue::Font,
    glyphs: HashMap<(u16, u32), (fontdue::Metrics, Vec<u8>)>,
    images: HashMap<String, Pixmap>,
}
fn path(rect: vello::kurbo::Rect, radius: f32, scale: f32) -> Path {
    let p = RoundedRect::from_rect(rect, radius as f64).to_path(0.1);
    let mut b = PathBuilder::new();
    for e in p.elements() {
        match e {
            PathEl::MoveTo(p) => b.move_to(p.x as f32 * scale, p.y as f32 * scale),
            PathEl::LineTo(p) => b.line_to(p.x as f32 * scale, p.y as f32 * scale),
            PathEl::QuadTo(p, q) => b.quad_to(
                p.x as f32 * scale,
                p.y as f32 * scale,
                q.x as f32 * scale,
                q.y as f32 * scale,
            ),
            PathEl::CurveTo(p, q, r) => b.cubic_to(
                p.x as f32 * scale,
                p.y as f32 * scale,
                q.x as f32 * scale,
                q.y as f32 * scale,
                r.x as f32 * scale,
                r.y as f32 * scale,
            ),
            PathEl::ClosePath => b.close(),
        }
    }
    b.finish().unwrap()
}
fn color(v: u32, alpha: f32) -> Color {
    Color::from_rgba8(
        (v >> 16) as u8,
        (v >> 8) as u8,
        v as u8,
        (alpha.clamp(0., 1.) * 255.) as u8,
    )
}
impl CpuRenderer {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            pixmap: Pixmap::new(w, h).unwrap(),
            font: fontdue::Font::from_bytes(
                include_bytes!("../assets/DejaVuSans.ttf") as &[u8],
                fontdue::FontSettings::default(),
            )
            .unwrap(),
            glyphs: HashMap::new(),
            images: HashMap::new(),
        }
    }
    pub fn focus(&mut self, root: &BoxNode, id: Option<u64>, scale: f32) {
        fn find(b: &BoxNode, id: u64) -> Option<&BoxNode> {
            if b.node.id == id {
                Some(b)
            } else {
                b.children.iter().find_map(|c| find(c, id))
            }
        }
        if let Some(b) = id.and_then(|id| find(root, id)) {
            let p = path(b.rect, b.node.style.radius, scale);
            let mut paint = Paint::default();
            paint.set_color(color(0xb5f36b, 1.));
            self.pixmap.stroke_path(
                &p,
                &paint,
                &Stroke {
                    width: 2. * scale,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
    pub fn image(&mut self, key: impl Into<String>, w: u32, h: u32, rgba: Vec<u8>) {
        if let Some(p) = IntSize::from_wh(w, h).and_then(|size| Pixmap::from_vec(rgba, size)) {
            self.images.insert(key.into(), p);
        }
    }
    pub fn draw(&mut self, root: &BoxNode, text: &mut TextEngine, w: u32, h: u32, scale: f32) {
        if self.pixmap.width() != w || self.pixmap.height() != h {
            self.pixmap = Pixmap::new(w.max(1), h.max(1)).unwrap();
        }
        self.pixmap.fill(color(0x0b1018, 1.));
        self.walk(root, text, scale, None, 1.);
    }
    fn walk(
        &mut self,
        b: &BoxNode,
        text: &mut TextEngine,
        scale: f32,
        mask: Option<&Mask>,
        alpha: f32,
    ) {
        let s = &b.node.style;
        let alpha = alpha * s.opacity;
        let p = path(b.rect, s.radius, scale);
        let mut paint = Paint::default();
        paint.anti_alias = true;
        if let Some((a, z)) = s.gradient {
            paint.shader = LinearGradient::new(
                Point::from_xy(b.rect.x0 as f32 * scale, b.rect.y0 as f32 * scale),
                Point::from_xy(b.rect.x1 as f32 * scale, b.rect.y1 as f32 * scale),
                vec![
                    GradientStop::new(0., color(a, alpha)),
                    GradientStop::new(1., color(z, alpha)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            )
            .unwrap();
            self.pixmap
                .fill_path(&p, &paint, FillRule::Winding, Transform::identity(), mask);
        } else if let Some(c) = s.background {
            paint.set_color(color(c, alpha));
            self.pixmap
                .fill_path(&p, &paint, FillRule::Winding, Transform::identity(), mask);
        }
        if let Some((w, c)) = s.border {
            paint.set_color(color(c, alpha));
            self.pixmap.stroke_path(
                &p,
                &paint,
                &Stroke {
                    width: w * scale,
                    ..Default::default()
                },
                Transform::identity(),
                mask,
            );
        }
        let clip = if s.clip || b.node.kind == Kind::Image {
            let mut clip = Mask::new(self.pixmap.width(), self.pixmap.height()).unwrap();
            clip.fill_path(&p, FillRule::Winding, true, Transform::identity());
            if let Some(parent) = mask {
                for (a, b) in clip.data_mut().iter_mut().zip(parent.data()) {
                    *a = ((*a as u16 * *b as u16) / 255) as u8;
                }
            }
            Some(clip)
        } else {
            None
        };
        let child_mask = clip.as_ref().or(mask);
        if matches!(b.node.kind, Kind::Text | Kind::Button | Kind::Input) {
            let shaped = text.shape(
                &b.node.text_content(),
                s.font_size,
                (b.rect.width() as f32 - 2. * s.padding).max(1.),
            );
            for run in &shaped.runs {
                for g in &run.glyphs {
                    let size = run.size * scale;
                    let key = (g.id as u16, size.to_bits());
                    if self.glyphs.len() > 2048 {
                        self.glyphs.clear();
                    }
                    let (m, bytes) = self
                        .glyphs
                        .entry(key)
                        .or_insert_with(|| self.font.rasterize_indexed(g.id as u16, size));
                    if m.width == 0 || m.height == 0 {
                        continue;
                    }
                    let mut glyph = Pixmap::new(m.width as u32, m.height as u32).unwrap();
                    for (dst, a) in glyph.data_mut().chunks_exact_mut(4).zip(bytes) {
                        let a = (*a as f32 * alpha) as u16;
                        dst[0] = (((s.color >> 16) & 255) as u16 * a / 255) as u8;
                        dst[1] = (((s.color >> 8) & 255) as u16 * a / 255) as u8;
                        dst[2] = ((s.color & 255) as u16 * a / 255) as u8;
                        dst[3] = a as u8;
                    }
                    let x = ((b.rect.x0 as f32 + s.padding + g.x) * scale).round() as i32 + m.xmin;
                    let y = ((b.rect.y0 as f32 + s.padding + g.y) * scale).round() as i32
                        - m.height as i32
                        - m.ymin;
                    self.pixmap.draw_pixmap(
                        x,
                        y,
                        glyph.as_ref(),
                        &PixmapPaint::default(),
                        Transform::identity(),
                        child_mask,
                    );
                }
            }
        }
        if b.node.kind == Kind::Image {
            if let Some(image) = self.images.get(&b.node.text) {
                let t = Transform::from_scale(
                    b.rect.width() as f32 * scale / image.width() as f32,
                    b.rect.height() as f32 * scale / image.height() as f32,
                )
                .post_translate(b.rect.x0 as f32 * scale, b.rect.y0 as f32 * scale);
                self.pixmap.draw_pixmap(
                    0,
                    0,
                    image.as_ref(),
                    &PixmapPaint {
                        opacity: alpha,
                        ..Default::default()
                    },
                    t,
                    child_mask,
                );
            }
        }
        for c in &b.children {
            self.walk(c, text, scale, child_mask, alpha)
        }
    }
}
