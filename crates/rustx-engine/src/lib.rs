//! Platform-shared layout, typography, interaction and GPU scene generation.
pub mod cpu;
pub mod gpu;
#[cfg(not(target_arch = "wasm32"))]
pub mod host;
pub mod layout;
pub mod scene;
pub mod text;
pub mod world;
use layout::{BoxNode, LayoutEngine};
use rustx_core::{NodeId, Runtime, View};
use scene::SceneEngine;
use text::TextEngine;
pub use vello;
use vello::{
    kurbo::{Affine, Point, Rect, Vec2},
    peniko::Color,
    Scene,
};
use world::World;
pub fn color(rgb: u32) -> Color {
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}
pub struct Engine {
    pub runtime: Runtime,
    pub layout: LayoutEngine,
    pub text: TextEngine,
    pub painter: SceneEngine,
    pub world: Option<World>,
    pub boxes: Option<BoxNode>,
    pub scene: Scene,
    size: (f32, f32),
    scale: f64,
    dirty: bool,
    focus: Option<NodeId>,
    pressed: Option<NodeId>,
    scroll: f64,
    applied_scroll: f64,
}
impl Engine {
    pub fn new(root: impl Fn(rustx_core::Scope) -> View + 'static) -> Self {
        Self {
            runtime: Runtime::new(root),
            layout: LayoutEngine::default(),
            text: TextEngine::default(),
            painter: SceneEngine::default(),
            world: None,
            boxes: None,
            scene: Scene::new(),
            size: (0., 0.),
            scale: 1.,
            dirty: true,
            focus: None,
            pressed: None,
            scroll: 0.,
            applied_scroll: 0.,
        }
    }
    pub fn focused_node(&self) -> Option<NodeId> {
        self.focus
    }
    pub fn focus_node(&mut self, id: NodeId) {
        self.focus = Some(id);
        self.dirty = true;
    }
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
    pub fn frame(&mut self, width: f32, height: f32, scale: f64) -> bool {
        let commit = self.runtime.flush();
        let layout_changed =
            self.size != (width, height) || self.boxes.is_none() || commit.components_rendered > 0;
        if layout_changed {
            self.boxes =
                Some(
                    self.layout
                        .compute(&self.runtime.snapshot(), width, height, &mut self.text),
                );
            self.applied_scroll = 0.;
        }
        if !self.dirty && !layout_changed && self.scale == scale {
            return false;
        }
        self.size = (width, height);
        self.scale = scale;
        self.dirty = false;
        let root = self.boxes.as_mut().unwrap();
        self.scroll = self
            .scroll
            .clamp(0., (root.rect.height() - f64::from(height)).max(0.));
        root.translate(Vec2::new(0., self.applied_scroll - self.scroll));
        self.applied_scroll = self.scroll;
        let logical = self.painter.build(
            root,
            &mut self.text,
            &mut self.world,
            Rect::new(0., 0., f64::from(width), f64::from(height)),
            self.focus,
        );
        self.scene.reset();
        self.scene.append(&logical, Some(Affine::scale(scale)));
        true
    }
    pub fn pointer_down(&mut self, p: Point) {
        self.pressed = self.painter.hit(p);
        self.focus = self.pressed;
        self.dirty = true;
    }
    pub fn pointer_up(&mut self, p: Point) {
        if let Some(id) = self
            .pressed
            .take()
            .filter(|id| Some(*id) == self.painter.hit(p))
        {
            self.runtime.click(id, p.x as f32, p.y as f32);
        }
        self.dirty = true;
    }
    pub fn cancel(&mut self) {
        self.pressed = None;
    }
    pub fn tab(&mut self, back: bool) {
        let ids = self.painter.buttons();
        if !ids.is_empty() {
            let i = ids.iter().position(|id| Some(*id) == self.focus);
            let next = if back {
                i.map_or(ids.len() - 1, |i| (i + ids.len() - 1) % ids.len())
            } else {
                i.map_or(0, |i| (i + 1) % ids.len())
            };
            self.focus = Some(ids[next]);
            self.dirty = true;
        }
    }
    pub fn activate(&mut self) {
        if let Some(id) = self.focus {
            self.runtime.click(id, 0., 0.);
            self.dirty = true;
        }
    }
    pub fn canvas(&self) -> Option<Rect> {
        fn find(b: &BoxNode) -> Option<Rect> {
            if b.node.kind == rustx_core::Kind::Canvas {
                Some(b.rect)
            } else {
                b.children.iter().find_map(find)
            }
        }
        self.boxes.as_ref().and_then(find)
    }
    pub fn zoom(&mut self, p: Point, factor: f64) {
        if let Some(rect) = self.canvas().filter(|r| r.contains(p)) {
            if let Some(w) = &mut self.world {
                w.camera
                    .zoom_at(Point::new(p.x - rect.x0, p.y - rect.y0), factor);
                self.dirty = true;
            }
        }
    }
    pub fn pan(&mut self, dx: f64, dy: f64) {
        if let Some(w) = &mut self.world {
            w.camera.pan(dx, dy);
            self.dirty = true;
        }
    }
    pub fn scroll(&mut self, dy: f64) {
        self.scroll = (self.scroll + dy).max(0.);
        self.dirty = true;
    }
}
