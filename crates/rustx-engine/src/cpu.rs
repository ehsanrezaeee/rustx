//! Explicit software backend; layout and shaped glyph positions stay in Rust.
use crate::{layout::BoxNode, text::TextEngine};
use rustx_core::Kind;
use std::collections::HashMap;
use tiny_skia::*;
use vello::kurbo::{PathEl, RoundedRect, Shape};
pub struct CpuRenderer {
    pub pixmap: Pixmap,
    fonts: HashMap<u64, fontdue::Font>,
    glyphs: crate::cache::BudgetCache<(u64, u16, u32), (fontdue::Metrics, Vec<u8>)>,
    pub visited: usize,
    pub culled: usize,
    pub drawing_visible: usize,
    pub clear_color: u32,
    images: HashMap<String, Pixmap>,
    image_epoch: u64,
}
fn path(rect: vello::kurbo::Rect, radius: f32, scale: f32) -> Path {
    let p = RoundedRect::from_rect(rect, radius as f64).to_path(0.1);
    geometry_path(&p, scale)
}
fn geometry_path(p: &vello::kurbo::BezPath, scale: f32) -> Path {
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
            fonts: HashMap::new(),
            glyphs: crate::cache::BudgetCache::new(2048, 4 * 1024 * 1024),
            visited: 0,
            culled: 0,
            drawing_visible: 0,
            clear_color: 0x0b1018,
            images: HashMap::new(),
            image_epoch: u64::MAX,
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
    pub fn editing(&mut self, root: &BoxNode, id: Option<u64>, text: &mut TextEngine, scale: f32) {
        fn find(b: &BoxNode, id: u64) -> Option<&BoxNode> {
            if b.node.id == id {
                Some(b)
            } else {
                b.children.iter().find_map(|c| find(c, id))
            }
        }
        if let Some(b) = id.and_then(|id| find(root, id)) {
            let mut clip = Mask::new(self.pixmap.width(), self.pixmap.height()).unwrap();
            clip.fill_path(
                &path(b.rect, b.node.style.radius, scale),
                FillRule::Winding,
                true,
                Transform::identity(),
            );
            for (rect, ink, alpha) in crate::editing::decorations(b, text) {
                let mut paint = Paint::default();
                paint.set_color(color(ink, alpha));
                self.pixmap.fill_path(
                    &path(rect, 0., scale),
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    Some(&clip),
                );
            }
        }
    }
    pub fn sync_images(&mut self, scene: &crate::scene::SceneEngine) {
        if self.image_epoch != scene.epoch {
            self.images.clear();
            for (key, image) in &scene.images {
                self.image(key.clone(), image.width, image.height, {
                    let mut bytes = image.data.data().to_vec();
                    for p in bytes.chunks_exact_mut(4) {
                        p[3] = (p[3] as f32 * image.alpha.clamp(0., 1.)).round() as u8;
                    }
                    bytes
                });
            }
            self.image_epoch = scene.epoch;
        }
    }
    pub fn image(&mut self, key: impl Into<String>, w: u32, h: u32, mut rgba: Vec<u8>) {
        for p in rgba.chunks_exact_mut(4) {
            for c in 0..3 {
                p[c] = ((p[c] as u16 * p[3] as u16 + 127) / 255) as u8;
            }
        }
        if let Some(p) = IntSize::from_wh(w, h).and_then(|size| Pixmap::from_vec(rgba, size)) {
            self.images.insert(key.into(), p);
        }
    }
    pub fn draw(&mut self, root: &BoxNode, text: &mut TextEngine, w: u32, h: u32, scale: f32) {
        if self.pixmap.width() != w || self.pixmap.height() != h {
            self.pixmap = Pixmap::new(w.max(1), h.max(1)).unwrap();
        }
        self.visited = 0;
        self.culled = 0;
        self.drawing_visible = 0;
        self.pixmap.fill(color(self.clear_color, 1.));
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
        self.visited += 1;
        let viewport = vello::kurbo::Rect::new(
            0.,
            0.,
            self.pixmap.width() as f64 / scale as f64,
            self.pixmap.height() as f64 / scale as f64,
        );
        let extent = b.node.style.shadow.map_or(b.rect, |s| {
            b.rect.union(
                (b.rect + vello::kurbo::Vec2::new(s.offset.0 as f64, s.offset.1 as f64))
                    .inflate(s.blur as f64 * 3., s.blur as f64 * 3.),
            )
        });
        if extent.intersect(viewport).area() <= 0. && b.node.style.clip {
            self.culled += 1;
            return;
        }
        if b.node.style.opacity < 1. {
            fn extent(b: &BoxNode) -> vello::kurbo::Rect {
                let mut r = b.rect;
                if let Some(s) = b.node.style.shadow {
                    r = r.union(
                        (b.rect + vello::kurbo::Vec2::new(s.offset.0 as f64, s.offset.1 as f64))
                            .inflate(s.blur as f64 * 3., s.blur as f64 * 3.),
                    );
                }
                if !b.node.style.clip {
                    for c in &b.children {
                        r = r.union(extent(c));
                    }
                }
                r
            }
            let r = extent(b).intersect(viewport);
            if r.area() <= 0. {
                return;
            }
            let left = (r.x0 * scale as f64).floor().max(0.) as u32;
            let top = (r.y0 * scale as f64).floor().max(0.) as u32;
            let width = ((r.x1 * scale as f64).ceil() as u32 - left).max(1);
            let height = ((r.y1 * scale as f64).ceil() as u32 - top).max(1);
            let mut layer = Pixmap::new(width, height).unwrap();
            let mut shifted = b.clone();
            shifted.translate(vello::kurbo::Vec2::new(
                -(left as f64) / scale as f64,
                -(top as f64) / scale as f64,
            ));
            let cropped = mask.map(|mask| {
                let mut m = Mask::new(width, height).unwrap();
                for y in 0..height {
                    let from = ((top + y) * mask.width() + left) as usize;
                    let to = (y * width) as usize;
                    m.data_mut()[to..to + width as usize]
                        .copy_from_slice(&mask.data()[from..from + width as usize]);
                }
                m
            });
            std::mem::swap(&mut self.pixmap, &mut layer);
            self.paint_node(&shifted, text, scale, cropped.as_ref(), 1.);
            std::mem::swap(&mut self.pixmap, &mut layer);
            self.pixmap.draw_pixmap(
                left as i32,
                top as i32,
                layer.as_ref(),
                &PixmapPaint {
                    opacity: (alpha * b.node.style.opacity).clamp(0., 1.),
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        } else {
            self.paint_node(b, text, scale, mask, alpha);
        }
    }
    fn paint_node(
        &mut self,
        b: &BoxNode,
        text: &mut TextEngine,
        scale: f32,
        mask: Option<&Mask>,
        alpha: f32,
    ) {
        let s = &b.node.style;
        let p = path(b.rect, s.radius, scale);
        let mut paint = Paint::default();
        paint.anti_alias = true;
        if let Some(shadow) = s.shadow {
            self.paint_shadow(b, shadow, scale, mask, alpha);
        }
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
            let border = path(b.rect.inset(-f64::from(w) * 0.5), s.radius, scale);
            self.pixmap.stroke_path(
                &border,
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
            let label_inset = crate::controls::label_inset(&b.node);
            let shaped = text.shape_node(
                &b.node,
                (b.rect.width() as f32 - s.padding_horizontal() - label_inset).max(1.),
            );
            self.paint_text(
                &shaped,
                vello::kurbo::Affine::translate((
                    b.rect.x0 + f64::from(s.padding_left() + label_inset),
                    b.rect.y0 + s.padding_top() as f64
                        - f64::from(b.node.editing.as_ref().map_or(0., |e| e.scroll_y)),
                )),
                s.color,
                child_mask,
                alpha,
                scale,
            );
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
                        quality: FilterQuality::Bilinear,
                        ..Default::default()
                    },
                    t,
                    child_mask,
                );
            }
        }
        for (rect, ink) in crate::controls::decorations(b) {
            let mut control_paint = Paint::default();
            control_paint.set_color(color(ink, alpha));
            self.pixmap.fill_path(
                &geometry_path(&rect, scale),
                &control_paint,
                FillRule::Winding,
                Transform::identity(),
                child_mask,
            );
        }
        if b.node.kind == Kind::Canvas {
            let mut canvas_clip = Mask::new(self.pixmap.width(), self.pixmap.height()).unwrap();
            canvas_clip.fill_path(&p, FillRule::Winding, true, Transform::identity());
            if let Some(parent) = child_mask {
                for (a, b) in canvas_clip.data_mut().iter_mut().zip(parent.data()) {
                    *a = ((*a as u16 * *b as u16) / 255) as u8;
                }
            }
            self.paint_canvas(b, text, scale, Some(&canvas_clip), alpha);
        }
        for c in &b.children {
            self.walk(c, text, scale, child_mask, alpha)
        }
    }
    fn paint_shadow(
        &mut self,
        b: &BoxNode,
        shadow: rustx_core::Shadow,
        scale: f32,
        mask: Option<&Mask>,
        alpha: f32,
    ) {
        let shifted =
            b.rect + vello::kurbo::Vec2::new(shadow.offset.0 as f64, shadow.offset.1 as f64);
        let extent = shifted.inflate(shadow.blur as f64 * 3. + 1., shadow.blur as f64 * 3. + 1.);
        let left = (extent.x0 * scale as f64).floor() as i32;
        let top = (extent.y0 * scale as f64).floor() as i32;
        let w = (extent.width() * scale as f64).ceil().max(1.) as u32;
        let h = (extent.height() * scale as f64).ceil().max(1.) as u32;
        if w as u64 * h as u64 > 16_777_216 {
            return;
        }
        let local = shifted
            + vello::kurbo::Vec2::new(-(left as f64) / scale as f64, -(top as f64) / scale as f64);
        let mut m = Mask::new(w, h).unwrap();
        m.fill_path(
            &path(local, b.node.style.radius, scale),
            FillRule::Winding,
            true,
            Transform::identity(),
        );
        let sigma = (shadow.blur * scale).max(0.01);
        let radius = (sigma * 3.).ceil() as i32;
        let weights: Vec<f32> = (-radius..=radius)
            .map(|x| (-(x * x) as f32 / (2. * sigma * sigma)).exp())
            .collect();
        let sum: f32 = weights.iter().sum();
        let mut temp = vec![0f32; w as usize * h as usize];
        let mut blurred = vec![0f32; temp.len()];
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let mut n = 0.;
                for (k, weight) in weights.iter().enumerate() {
                    let q = x + k as i32 - radius;
                    if q >= 0 && q < w as i32 {
                        n += m.data()[(y * w as i32 + q) as usize] as f32 * weight;
                    }
                }
                temp[(y * w as i32 + x) as usize] = n / sum;
            }
        }
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let mut n = 0.;
                for (k, weight) in weights.iter().enumerate() {
                    let q = y + k as i32 - radius;
                    if q >= 0 && q < h as i32 {
                        n += temp[(q * w as i32 + x) as usize] * weight;
                    }
                }
                blurred[(y * w as i32 + x) as usize] = n / sum;
            }
        }
        let mut image = Pixmap::new(w, h).unwrap();
        for (pixel, a) in image.data_mut().chunks_exact_mut(4).zip(blurred) {
            let a = (a * alpha).clamp(0., 255.) as u16;
            pixel[0] = (((shadow.color >> 16) & 255) as u16 * a / 255) as u8;
            pixel[1] = (((shadow.color >> 8) & 255) as u16 * a / 255) as u8;
            pixel[2] = ((shadow.color & 255) as u16 * a / 255) as u8;
            pixel[3] = a as u8;
        }
        self.pixmap.draw_pixmap(
            left,
            top,
            image.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            mask,
        );
    }
    pub fn glyph_stats(&self) -> (usize, usize) {
        (self.glyphs.len(), self.glyphs.bytes())
    }
    fn paint_text(
        &mut self,
        shaped: &crate::text::Shaped,
        transform: vello::kurbo::Affine,
        ink: u32,
        mask: Option<&Mask>,
        alpha: f32,
        scale: f32,
    ) {
        for run in &shaped.runs {
            let font_id = run.font.data.id();
            if !self.fonts.contains_key(&font_id) {
                if self.fonts.len() >= 16 {
                    self.fonts.clear();
                }
                if let Ok(font) = fontdue::Font::from_bytes(
                    run.font.data.data(),
                    fontdue::FontSettings {
                        collection_index: run.font.index,
                        ..Default::default()
                    },
                ) {
                    self.fonts.insert(font_id, font);
                } else {
                    continue;
                }
            }
            for g in &run.glyphs {
                let size = run.size * scale;
                let key = (font_id, g.id as u16, size.to_bits());
                if self.glyphs.get(&key).is_none() {
                    let data = self.fonts[&font_id].rasterize_indexed(g.id as u16, size);
                    let n = data.1.len() + std::mem::size_of::<fontdue::Metrics>();
                    self.glyphs.insert(key, data, n);
                }
                let Some((m, bytes)) = self.glyphs.get(&key) else {
                    continue;
                };
                if m.width == 0 || m.height == 0 {
                    continue;
                }
                let mut glyph = Pixmap::new(m.width as u32, m.height as u32).unwrap();
                for (dst, a) in glyph.data_mut().chunks_exact_mut(4).zip(bytes) {
                    let a = (*a as f32 * alpha) as u16;
                    dst[0] = (((ink >> 16) & 255) as u16 * a / 255) as u8;
                    dst[1] = (((ink >> 8) & 255) as u16 * a / 255) as u8;
                    dst[2] = ((ink & 255) as u16 * a / 255) as u8;
                    dst[3] = a as u8;
                }
                let local = vello::kurbo::Affine::translate((
                    g.x as f64 + m.xmin as f64 / scale as f64,
                    g.y as f64 - (m.height as f64 + m.ymin as f64) / scale as f64,
                )) * vello::kurbo::Affine::scale(1. / scale as f64);
                let t = (vello::kurbo::Affine::scale(scale as f64) * transform * local).as_coeffs();
                self.pixmap.draw_pixmap(
                    0,
                    0,
                    glyph.as_ref(),
                    &PixmapPaint {
                        quality: FilterQuality::Bilinear,
                        ..Default::default()
                    },
                    Transform::from_row(
                        t[0] as f32,
                        t[1] as f32,
                        t[2] as f32,
                        t[3] as f32,
                        t[4] as f32,
                        t[5] as f32,
                    ),
                    mask,
                );
            }
        }
    }
    fn paint_canvas(
        &mut self,
        b: &BoxNode,
        text: &mut TextEngine,
        scale: f32,
        mask: Option<&Mask>,
        alpha: f32,
    ) {
        let Some(scene) = &b.node.drawing else {
            return;
        };
        let ids = scene.visible(b.node.camera, b.rect.width(), b.rect.height());
        self.drawing_visible += ids.len();
        for id in ids {
            let shape = scene.shape(id);
            let (geometry, t) =
                crate::drawing::geometry(shape, b.node.camera, (b.rect.x0, b.rect.y0));
            let mut builder = PathBuilder::new();
            let map = |p: vello::kurbo::Point| {
                let p = t * p;
                (p.x as f32 * scale, p.y as f32 * scale)
            };
            for el in geometry.elements() {
                match *el {
                    PathEl::MoveTo(p) => {
                        let (x, y) = map(p);
                        builder.move_to(x, y);
                    }
                    PathEl::LineTo(p) => {
                        let (x, y) = map(p);
                        builder.line_to(x, y);
                    }
                    PathEl::QuadTo(p, q) => {
                        let (x, y) = map(p);
                        let (a, b) = map(q);
                        builder.quad_to(x, y, a, b);
                    }
                    PathEl::CurveTo(p, q, r) => {
                        let (x, y) = map(p);
                        let (a, b) = map(q);
                        let (c, d) = map(r);
                        builder.cubic_to(x, y, a, b, c, d);
                    }
                    PathEl::ClosePath => builder.close(),
                }
            }
            let Some(path) = builder.finish() else {
                continue;
            };
            let mut paint = Paint::default();
            paint.set_color(color(shape.fill, alpha));
            match &shape.primitive {
                rustx_core::drawing::Primitive::Rect { .. } => self.pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    mask,
                ),
                rustx_core::drawing::Primitive::Path { stroke, .. } => {
                    self.pixmap.fill_path(
                        &path,
                        &paint,
                        FillRule::Winding,
                        Transform::identity(),
                        mask,
                    );
                    if let Some((width, c)) = stroke {
                        paint.set_color(color(*c, alpha));
                        let outline = vello::kurbo::stroke(
                            geometry.iter(),
                            &vello::kurbo::Stroke::new(*width),
                            &vello::kurbo::StrokeOpts::default(),
                            0.01,
                        );
                        let mut outline_builder = PathBuilder::new();
                        for el in outline.elements() {
                            match *el {
                                PathEl::MoveTo(p) => {
                                    let (x, y) = map(p);
                                    outline_builder.move_to(x, y);
                                }
                                PathEl::LineTo(p) => {
                                    let (x, y) = map(p);
                                    outline_builder.line_to(x, y);
                                }
                                PathEl::QuadTo(p, q) => {
                                    let (x, y) = map(p);
                                    let (a, b) = map(q);
                                    outline_builder.quad_to(x, y, a, b);
                                }
                                PathEl::CurveTo(p, q, r) => {
                                    let (x, y) = map(p);
                                    let (a, b) = map(q);
                                    let (c, d) = map(r);
                                    outline_builder.cubic_to(x, y, a, b, c, d);
                                }
                                PathEl::ClosePath => outline_builder.close(),
                            }
                        }
                        if let Some(path) = outline_builder.finish() {
                            self.pixmap.fill_path(
                                &path,
                                &paint,
                                FillRule::Winding,
                                Transform::identity(),
                                mask,
                            );
                        }
                    }
                }
                rustx_core::drawing::Primitive::Text {
                    text: content,
                    size,
                } => {
                    let shaped = text.shape(content, *size, shape.bounds.width() as f32);
                    self.paint_text(&shaped, t, shape.fill, mask, alpha, scale);
                }
                rustx_core::drawing::Primitive::Image { key } => {
                    if let Some(image) = self.images.get(key) {
                        let t = (vello::kurbo::Affine::scale(scale as f64)
                            * t
                            * vello::kurbo::Affine::scale_non_uniform(
                                shape.bounds.width() / image.width() as f64,
                                shape.bounds.height() / image.height() as f64,
                            ))
                        .as_coeffs();
                        self.pixmap.draw_pixmap(
                            0,
                            0,
                            image.as_ref(),
                            &PixmapPaint {
                                opacity: alpha,
                                quality: FilterQuality::Bilinear,
                                ..Default::default()
                            },
                            Transform::from_row(
                                t[0] as f32,
                                t[1] as f32,
                                t[2] as f32,
                                t[3] as f32,
                                t[4] as f32,
                                t[5] as f32,
                            ),
                            mask,
                        );
                    }
                }
            }
        }
    }
}
