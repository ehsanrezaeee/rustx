//! Retained per-node Vello fragments and clipping-aware spatial hit testing.
use crate::{color, layout::BoxNode, text::TextEngine, world::World};
use rustx_core::{Kind, NodeId, Style};
use std::collections::{HashMap, HashSet};
use vello::{
    kurbo::{Affine, Point, Rect, RoundedRect, Shape, Stroke},
    peniko::{Fill, Gradient, Image, Mix},
    Scene,
};
#[derive(PartialEq)]
struct PaintKey {
    style: Style,
    text: String,
    size: (f64, f64),
    kind: Kind,
    label_inset: f32,
    text_scroll: f32,
    epoch: u64,
}
struct Cached {
    key: PaintKey,
    scene: Scene,
}
struct Hit {
    id: NodeId,
    shape: RoundedRect,
    clips: Vec<RoundedRect>,
}
#[derive(Default)]
pub struct SceneEngine {
    cache: HashMap<NodeId, Cached>,
    pub(crate) images: HashMap<String, Image>,
    pub(crate) epoch: u64,
    hits: Vec<Hit>,
    grid: HashMap<(i32, i32), Vec<usize>>,
    pub painted: usize,
    pub reused: usize,
    pub drawing_visible: usize,
}
impl SceneEngine {
    pub fn image(&mut self, key: impl Into<String>, image: Image) -> Result<(), &'static str> {
        let key = key.into();
        let bytes = image.data.data().len();
        let used: usize = self
            .images
            .iter()
            .filter(|(k, _)| *k != &key)
            .map(|(_, i)| i.data.data().len())
            .sum();
        if image.width == 0
            || image.height == 0
            || image.width > 4096
            || image.height > 4096
            || bytes != image.width as usize * image.height as usize * 4
            || used + bytes > 32 * 1024 * 1024
            || (!self.images.contains_key(&key) && self.images.len() >= 64)
        {
            return Err("image resource budget exceeded or invalid RGBA");
        }
        self.images.insert(key, image);
        self.epoch += 1;
        self.cache.clear();
        Ok(())
    }
    pub fn remove_image(&mut self, key: &str) {
        self.images.remove(key);
        self.epoch += 1;
        self.cache.clear();
    }
    pub fn resource_stats(&self) -> (usize, usize, usize, usize) {
        (
            self.images.len(),
            self.images.values().map(|i| i.data.data().len()).sum(),
            self.cache.len(),
            self.cache_bytes(),
        )
    }
    fn cache_bytes(&self) -> usize {
        fn bytes<T>(v: &Vec<T>) -> usize {
            v.capacity() * std::mem::size_of::<T>()
        }
        self.cache
            .values()
            .map(|c| {
                let e = c.scene.encoding();
                c.key.text.capacity()
                    + bytes(&e.path_tags)
                    + bytes(&e.path_data)
                    + bytes(&e.draw_tags)
                    + bytes(&e.draw_data)
                    + bytes(&e.transforms)
                    + bytes(&e.styles)
                    + bytes(&e.resources.patches)
                    + bytes(&e.resources.color_stops)
                    + bytes(&e.resources.glyphs)
                    + bytes(&e.resources.glyph_runs)
                    + bytes(&e.resources.normalized_coords)
            })
            .sum()
    }
    pub fn hit(&self, p: Point) -> Option<NodeId> {
        self.grid
            .get(&((p.x / 128.).floor() as i32, (p.y / 128.).floor() as i32))?
            .iter()
            .rev()
            .find_map(|i| {
                let h = &self.hits[*i];
                (h.shape.winding(p) != 0 && h.clips.iter().all(|c| c.winding(p) != 0))
                    .then_some(h.id)
            })
    }
    pub fn buttons(&self) -> Vec<NodeId> {
        self.hits.iter().map(|h| h.id).collect()
    }
    fn local_scene(&self, key: &PaintKey, text: &mut TextEngine) -> Scene {
        let s = &key.style;
        let mut own = Scene::new();
        let rect = Rect::new(0., 0., key.size.0, key.size.1);
        let rounded = RoundedRect::from_rect(rect, f64::from(s.radius));
        if let Some(shadow) = s.shadow {
            own.draw_blurred_rounded_rect(
                Affine::translate((f64::from(shadow.offset.0), f64::from(shadow.offset.1))),
                rect,
                color(shadow.color),
                f64::from(s.radius),
                f64::from(shadow.blur),
            );
        }
        if let Some((a, b)) = s.gradient {
            own.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Gradient::new_linear((0., 0.), (rect.width(), rect.height()))
                    .with_stops([color(a), color(b)]),
                None,
                &rounded,
            );
        } else if let Some(c) = s.background {
            own.fill(Fill::NonZero, Affine::IDENTITY, color(c), None, &rounded);
        }
        if let Some((width, c)) = s.border {
            own.stroke(
                &Stroke::new(f64::from(width)),
                Affine::IDENTITY,
                color(c),
                None,
                &RoundedRect::from_rect(rect.inset(-f64::from(width) * 0.5), f64::from(s.radius)),
            );
        }
        if matches!(key.kind, Kind::Text | Kind::Button | Kind::Input) {
            let shaped = text.shape_with(
                &key.text,
                s.font_size,
                (rect.width() as f32 - 2. * s.padding - key.label_inset).max(1.),
                s.text_align,
                s.text_direction,
            );
            TextEngine::paint(
                &mut own,
                &shaped,
                Affine::translate((
                    f64::from(s.padding + key.label_inset),
                    f64::from(s.padding - key.text_scroll),
                )),
                color(s.color),
            );
        }
        if key.kind == Kind::Image {
            own.push_layer(Mix::Clip, 1., Affine::IDENTITY, &rounded);
            if let Some(image) = self.images.get(&key.text) {
                own.draw_image(
                    image,
                    Affine::scale_non_uniform(
                        rect.width() / f64::from(image.width),
                        rect.height() / f64::from(image.height),
                    ),
                );
            }
            own.pop_layer();
        }
        own
    }
    pub fn build(
        &mut self,
        root: &BoxNode,
        text: &mut TextEngine,
        world: &mut Option<World>,
        viewport: Rect,
        focus: Option<NodeId>,
    ) -> Scene {
        self.painted = 0;
        self.drawing_visible = 0;
        self.reused = 0;
        self.hits.clear();
        self.grid.clear();
        let mut scene = Scene::new();
        let mut seen = HashSet::new();
        self.walk(
            root,
            text,
            world,
            viewport,
            &mut vec![],
            &mut scene,
            &mut seen,
            focus,
        );
        self.cache.retain(|id, _| seen.contains(id));
        if self.cache.len() > 4096 || self.cache_bytes() > 16 * 1024 * 1024 {
            self.cache.clear();
        }
        scene
    }
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &mut self,
        b: &BoxNode,
        text: &mut TextEngine,
        world: &mut Option<World>,
        viewport: Rect,
        clips: &mut Vec<RoundedRect>,
        scene: &mut Scene,
        seen: &mut HashSet<NodeId>,
        focus: Option<NodeId>,
    ) {
        seen.insert(b.node.id);
        let s = &b.node.style;
        let extent = s.shadow.map_or(0., |v| {
            f64::from(v.blur * 2.5 + v.offset.0.abs() + v.offset.1.abs())
        });
        let visible = b.rect.inflate(extent, extent).intersect(viewport).area() > 0.;
        if !visible && s.clip {
            return;
        }
        let shape = RoundedRect::from_rect(b.rect, f64::from(s.radius));
        if s.opacity < 1. {
            scene.push_layer(Mix::Normal, s.opacity, Affine::IDENTITY, &viewport);
        }
        if visible {
            let content = if matches!(b.node.kind, Kind::Text | Kind::Button | Kind::Input) {
                b.node.text_content()
            } else {
                b.node.text.clone()
            };
            let key = PaintKey {
                style: s.clone(),
                text: content,
                size: (b.rect.width(), b.rect.height()),
                kind: b.node.kind,
                label_inset: crate::controls::label_inset(&b.node),
                text_scroll: b.node.editing.as_ref().map_or(0., |e| e.scroll_y),
                epoch: if matches!(b.node.kind, Kind::Text | Kind::Input | Kind::Button) {
                    text.epoch
                } else if b.node.kind == Kind::Image {
                    self.epoch
                } else {
                    0
                },
            };
            if self.cache.get(&b.node.id).is_some_and(|c| c.key == key) {
                self.reused += 1;
            } else {
                let own = self.local_scene(&key, text);
                self.cache.insert(b.node.id, Cached { key, scene: own });
                self.painted += 1;
            }
            scene.append(
                &self.cache[&b.node.id].scene,
                Some(Affine::translate(b.rect.origin().to_vec2())),
            );
            for (rect, ink) in crate::controls::decorations(b) {
                scene.fill(Fill::NonZero, Affine::IDENTITY, color(ink), None, &rect);
            }
            if b.node.interactive {
                let i = self.hits.len();
                self.hits.push(Hit {
                    id: b.node.id,
                    shape,
                    clips: clips.clone(),
                });
                let bounds = b.rect.intersect(viewport);
                for x in (bounds.x0 / 128.).floor() as i32..=(bounds.x1 / 128.).floor() as i32 {
                    for y in (bounds.y0 / 128.).floor() as i32..=(bounds.y1 / 128.).floor() as i32 {
                        self.grid.entry((x, y)).or_default().push(i)
                    }
                }
            }
            if focus == Some(b.node.id) {
                scene.push_layer(Mix::Clip, 1., Affine::IDENTITY, &shape);
                for (rect, ink, alpha) in crate::editing::decorations(b, text) {
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        color(ink).with_alpha(alpha),
                        None,
                        &rect,
                    );
                }
                scene.pop_layer();
                scene.stroke(
                    &Stroke::new(2.),
                    Affine::IDENTITY,
                    color(0x8ef0c5),
                    None,
                    &RoundedRect::from_rect(b.rect.inset(2.), f64::from(s.radius)),
                );
            }
        }
        if s.clip {
            scene.push_layer(Mix::Clip, 1., Affine::IDENTITY, &shape);
            clips.push(shape);
        }
        if b.node.kind == Kind::Canvas && visible {
            scene.push_layer(Mix::Clip, 1., Affine::IDENTITY, &shape);
            if let Some(drawing) = &b.node.drawing {
                let camera = b.node.camera;
                let ids = drawing.visible(camera, b.rect.width(), b.rect.height());
                self.drawing_visible += ids.len();
                for id in ids {
                    let item = drawing.shape(id);
                    let (path, transform) =
                        crate::drawing::geometry(item, camera, (b.rect.x0, b.rect.y0));
                    match &item.primitive {
                        rustx_core::drawing::Primitive::Rect { .. } => {
                            scene.fill(Fill::NonZero, transform, color(item.fill), None, &path)
                        }
                        rustx_core::drawing::Primitive::Path { stroke, .. } => {
                            scene.fill(Fill::NonZero, transform, color(item.fill), None, &path);
                            if let Some((width, c)) = stroke {
                                scene.stroke(
                                    &Stroke::new(*width),
                                    transform,
                                    color(*c),
                                    None,
                                    &path,
                                );
                            }
                        }
                        rustx_core::drawing::Primitive::Text {
                            text: content,
                            size,
                        } => {
                            let shaped = text.shape(content, *size, item.bounds.width() as f32);
                            TextEngine::paint(scene, &shaped, transform, color(item.fill));
                        }
                        rustx_core::drawing::Primitive::Image { key } => {
                            if let Some(image) = self.images.get(key) {
                                scene.draw_image(
                                    image,
                                    transform
                                        * Affine::scale_non_uniform(
                                            item.bounds.width() / image.width as f64,
                                            item.bounds.height() / image.height as f64,
                                        ),
                                );
                            }
                        }
                    }
                }
            }
            if let Some(world) = world {
                world.paint(scene, b.rect)
            }
            scene.pop_layer();
        }
        let viewport = if s.clip {
            viewport.intersect(b.rect)
        } else {
            viewport
        };
        for c in &b.children {
            self.walk(c, text, world, viewport, clips, scene, seen, focus);
        }
        if s.clip {
            clips.pop();
            scene.pop_layer();
        }
        if s.opacity < 1. {
            scene.pop_layer();
        }
    }
}
