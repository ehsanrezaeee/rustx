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
    images: HashMap<String, Image>,
    epoch: u64,
    hits: Vec<Hit>,
    grid: HashMap<(i32, i32), Vec<usize>>,
    pub painted: usize,
    pub reused: usize,
}
impl SceneEngine {
    pub fn image(&mut self, key: impl Into<String>, image: Image) {
        self.images.insert(key.into(), image);
        self.epoch += 1;
    }
    pub fn remove_image(&mut self, key: &str) {
        self.images.remove(key);
        self.epoch += 1;
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
                &RoundedRect::from_rect(rect.inset(f64::from(width) * 0.5), f64::from(s.radius)),
            );
        }
        if matches!(key.kind, Kind::Text | Kind::Button | Kind::Input) {
            let shaped = text.shape(
                &key.text,
                s.font_size,
                (rect.width() as f32 - 2. * s.padding).max(1.),
            );
            TextEngine::paint(
                &mut own,
                &shaped,
                Affine::translate((f64::from(s.padding), f64::from(s.padding))),
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
                epoch: if b.node.kind == Kind::Image {
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
