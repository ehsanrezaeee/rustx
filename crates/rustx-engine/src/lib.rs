//! Platform-shared layout, typography, interaction and GPU scene generation.
pub mod cache;
pub mod cpu;
pub mod drawing;
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
    last_pointer: Option<Point>,
    dragged: bool,
    scroll: f64,
    applied_scroll: f64,
    font_epoch: u64,
    resource_epoch: u64,
    theme: rustx_core::Signal<rustx_core::Theme>,
}
impl Engine {
    pub fn new(root: impl Fn(rustx_core::Scope) -> View + 'static) -> Self {
        let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
        let out = saved.clone();
        let runtime = Runtime::new(move |cx| {
            let theme = cx.signal(rustx_core::Theme::DARK);
            cx.provide(rustx_core::ThemeContext(theme.clone()));
            *out.borrow_mut() = Some(theme);
            root(cx)
        });
        let theme = saved.borrow().clone().unwrap();
        Self {
            runtime,
            theme,
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
            last_pointer: None,
            dragged: false,
            scroll: 0.,
            applied_scroll: 0.,
            font_epoch: 0,
            resource_epoch: 0,
        }
    }
    pub fn canvas_event(&mut self, id: NodeId, event: rustx_core::drawing::CanvasEvent) -> bool {
        self.runtime.canvas(id, event)
    }
    pub fn canvas_at(&self, id: NodeId) -> Option<Rect> {
        fn find(b: &BoxNode, id: NodeId) -> Option<Rect> {
            if b.node.id == id && b.node.kind == rustx_core::Kind::Canvas {
                Some(b.rect)
            } else {
                b.children.iter().find_map(|c| find(c, id))
            }
        }
        self.boxes.as_ref().and_then(|b| find(b, id))
    }
    pub fn viewport(&self) -> (f32, f32) {
        self.size
    }
    pub fn set_theme(&mut self, theme: rustx_core::Theme) {
        let _ = self.theme.set(theme);
        if self.layout.theme != theme {
            self.layout.theme = theme;
            self.boxes = None;
            self.invalidate();
        }
    }
    pub fn focused_node(&self) -> Option<NodeId> {
        self.focus
    }
    pub fn focus_node(&mut self, id: NodeId) {
        self.runtime.focus(id);
        self.focus = Some(id);
        self.dirty = true;
    }
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
    pub fn frame(&mut self, width: f32, height: f32, scale: f64) -> bool {
        if !width.is_finite()
            || !height.is_finite()
            || !scale.is_finite()
            || width <= 0.
            || height <= 0.
            || scale <= 0.
            || width as f64 * scale > 8192.
            || height as f64 * scale > 8192.
            || width as f64 * height as f64 * scale * scale > 16_777_216.
        {
            return false;
        }
        if let Ok(theme) = self.theme.peek() {
            if theme != self.layout.theme {
                self.layout.theme = theme;
                self.boxes = None;
            }
        }
        let resources_changed = self.resource_epoch != self.painter.epoch;
        self.resource_epoch = self.painter.epoch;
        let font_changed = self.font_epoch != self.text.epoch;
        self.font_epoch = self.text.epoch;
        self.layout.font_epoch = self.text.epoch;
        let commit = self.runtime.flush();
        let layout_changed = self.size != (width, height)
            || self.boxes.is_none()
            || commit.components_rendered > 0
            || font_changed;
        if layout_changed {
            self.boxes =
                Some(
                    self.layout
                        .compute(&self.runtime.snapshot(), width, height, &mut self.text),
                );
            self.applied_scroll = 0.;
        }
        if !self.dirty && !layout_changed && !resources_changed && self.scale == scale {
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
        self.last_pointer = Some(p);
        self.dragged = false;
        self.pressed = self.painter.hit(p);
        self.focus = self.pressed;
        if let Some(id) = self.focus {
            self.runtime.focus(id);
        }
        self.dirty = true;
    }
    pub fn pointer_move(&mut self, p: Point) {
        if let (Some(id), Some(last)) = (self.pressed, self.last_pointer) {
            if self.canvas_at(id).is_some() {
                let (dx, dy) = (p.x - last.x, p.y - last.y);
                if dx != 0. || dy != 0. {
                    self.runtime
                        .canvas(id, rustx_core::drawing::CanvasEvent::Pan { dx, dy });
                    self.dragged = true;
                }
            }
        }
        self.last_pointer = Some(p);
    }
    pub fn pointer_up(&mut self, p: Point) {
        if let Some(id) = self
            .pressed
            .take()
            .filter(|id| Some(*id) == self.painter.hit(p))
        {
            if let Some(rect) = self.canvas_at(id) {
                if !self.dragged {
                    self.runtime.canvas(
                        id,
                        rustx_core::drawing::CanvasEvent::Pick {
                            x: p.x - rect.x0,
                            y: p.y - rect.y0,
                        },
                    );
                }
            } else {
                self.runtime.click(id, p.x as f32, p.y as f32);
            }
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
            self.runtime.focus(ids[next]);
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
    pub fn scroll_at(&mut self, p: Point, dy: f64) {
        if !self
            .painter
            .hit(p)
            .is_some_and(|id| self.runtime.scroll(id, dy))
        {
            self.scroll(dy);
        }
    }
    pub fn scroll(&mut self, dy: f64) {
        self.scroll = (self.scroll + dy).max(0.);
        self.dirty = true;
    }
}
