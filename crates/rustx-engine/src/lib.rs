//! Platform-shared layout, typography, interaction and GPU scene generation.
#[cfg(not(target_arch = "wasm32"))]
pub mod accessibility;
pub mod assets;
pub mod cache;
pub mod controls;
pub mod cpu;
pub mod diagnostics;
pub mod drawing;
pub mod editing;
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
    pub diagnostics: diagnostics::Diagnostics,
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
    hovered: Option<NodeId>,
    last_pointer: Option<Point>,
    dragged: bool,
    pointer_default_prevented: bool,
    scroll: f64,
    applied_scroll: f64,
    committed_scroll: f64,
    scroll_positions: std::collections::HashMap<NodeId, f64>,
    font_epoch: u64,
    resource_epoch: u64,
    model_epoch: u64,
    modal: Option<NodeId>,
    restore_focus: Option<NodeId>,
    text_drag: Option<(u64, NodeId, usize)>,
    preferred_caret_x: Option<(NodeId, f64, usize)>,
    editor_revision: Option<(NodeId, u64)>,
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
            diagnostics: diagnostics::Diagnostics::default(),
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
            hovered: None,
            last_pointer: None,
            dragged: false,
            pointer_default_prevented: false,
            scroll: 0.,
            applied_scroll: 0.,
            committed_scroll: 0.,
            scroll_positions: Default::default(),
            font_epoch: 0,
            resource_epoch: 0,
            model_epoch: 0,
            modal: None,
            restore_focus: None,
            text_drag: None,
            preferred_caret_x: None,
            editor_revision: None,
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
    /// Browser blur has already dispatched its typed event; clear focus without
    /// emitting a second Blur event or moving focus back into the application.
    pub fn blur_node(&mut self, id: NodeId) {
        if self.focus == Some(id) {
            self.focus = None;
            self.dirty = true;
        }
    }
    pub fn dispatch(
        &mut self,
        id: NodeId,
        event: rustx_core::EventData,
    ) -> rustx_core::DispatchResult {
        fn walk(b: &BoxNode, out: &mut std::collections::HashMap<NodeId, (f64, f64)>) {
            out.insert(b.node.id, (b.rect.x0, b.rect.y0));
            for child in &b.children {
                walk(child, out);
            }
        }
        let mut origins = std::collections::HashMap::new();
        if let Some(boxes) = &self.boxes {
            walk(boxes, &mut origins);
        }
        let captured = self.runtime.captured_pointer(event.pointer_id);
        if matches!(
            event.kind,
            rustx_core::EventKind::Click
                | rustx_core::EventKind::PointerDown
                | rustx_core::EventKind::PointerMove
                | rustx_core::EventKind::PointerUp
                | rustx_core::EventKind::Wheel
                | rustx_core::EventKind::KeyDown
        ) {
            fn modal(s: &rustx_core::Snapshot) -> Option<&rustx_core::Snapshot> {
                if s.control.hidden {
                    return None;
                }
                s.children
                    .iter()
                    .rev()
                    .find_map(modal)
                    .or_else(|| s.control.modal.then_some(s))
            }
            fn contains(s: &rustx_core::Snapshot, id: NodeId) -> bool {
                s.id == id || s.children.iter().any(|c| contains(c, id))
            }
            let effective = if matches!(
                event.kind,
                rustx_core::EventKind::PointerMove | rustx_core::EventKind::PointerUp
            ) {
                captured.unwrap_or(id)
            } else {
                id
            };
            let snapshot = self.runtime.snapshot();
            if modal(&snapshot).is_some_and(|modal| !contains(modal, effective)) {
                let mut result = rustx_core::DispatchResult {
                    default_prevented: true,
                    ..Default::default()
                };
                // A capture acquired before the modal opened must still end.
                if event.kind == rustx_core::EventKind::PointerUp && captured.is_some() {
                    let mut cancel = rustx_core::EventData::pointer(
                        rustx_core::EventKind::PointerCancel,
                        event.pointer_id,
                        event.x,
                        event.y,
                    );
                    cancel.pointer_type = event.pointer_type;
                    let cleanup = self
                        .runtime
                        .dispatch_with_origins(effective, cancel, &origins);
                    result.pointer_released = cleanup.pointer_released;
                }
                return result;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let link_default =
            event.kind == rustx_core::EventKind::Click && !event.ctrl && !event.meta && !event.alt;
        let slider_default = matches!(event.kind, rustx_core::EventKind::PointerDown)
            || event.kind == rustx_core::EventKind::PointerMove && captured.is_some();
        let pointer_down = event.kind == rustx_core::EventKind::PointerDown;
        let pointer_id = event.pointer_id;
        let kind = event.kind;
        let primary = event.button == 0;
        let extend = event.shift;
        let text_pointer = event.pointer_type != rustx_core::PointerType::Touch;
        let point = Point::new(event.x, event.y);
        let mut result = self.runtime.dispatch_with_origins(id, event, &origins);
        let drag = self
            .text_drag
            .filter(|(pointer, _, _)| *pointer == pointer_id);
        if drag.is_some()
            && matches!(
                kind,
                rustx_core::EventKind::PointerUp | rustx_core::EventKind::PointerCancel
            )
        {
            self.text_drag = None;
        }
        if !result.default_prevented && text_pointer {
            if pointer_down && primary && result.pointer_capture.is_none() {
                if let Some((editor, offset)) = self.text_position_at(id, point) {
                    self.preferred_caret_x = None;
                    self.focus_node(id);
                    let anchor = if extend {
                        editor.selection().anchor
                    } else {
                        offset
                    };
                    let _ = editor.select(anchor, offset);
                    self.runtime.invalidate_model();
                    if self.runtime.capture_pointer(pointer_id, id) {
                        self.text_drag = Some((pointer_id, id, anchor));
                        result.pointer_capture = Some(id);
                    }
                    result.handled = true;
                    result.default_prevented = true;
                }
            } else if matches!(
                kind,
                rustx_core::EventKind::PointerMove | rustx_core::EventKind::PointerUp
            ) {
                if let Some((_, node, anchor)) = drag {
                    if let Some((editor, offset)) = self.text_position_at(node, point) {
                        let _ = editor.select(anchor, offset);
                        self.runtime.invalidate_model();
                        result.handled = true;
                        result.default_prevented = true;
                    }
                }
            }
        }
        if slider_default && !result.default_prevented {
            let slider = captured.unwrap_or(id);
            if self.slider_at(slider, point) {
                result.handled = true;
                if pointer_down && self.runtime.capture_pointer(pointer_id, slider) {
                    result.pointer_capture = Some(slider);
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if link_default && !result.default_prevented {
            fn find(s: &rustx_core::Snapshot, id: NodeId) -> Option<&rustx_core::Snapshot> {
                if s.id == id {
                    Some(s)
                } else {
                    s.children.iter().find_map(|c| find(c, id))
                }
            }
            if let Some(node) =
                find(&self.runtime.snapshot(), id).filter(|s| s.role == Some("link"))
            {
                if let (Some(url), Some(scope)) =
                    (node.control.href.clone(), self.runtime.scope_for_node(id))
                {
                    if let Some(services) = scope.context::<rustx_core::ServiceClient>() {
                        result.handled |= services
                            .call(
                                &scope,
                                rustx_core::ServiceOperation::Navigate {
                                    url,
                                    replace: false,
                                    hash: false,
                                },
                                |_| {},
                            )
                            .is_ok();
                    }
                }
            }
        }
        if self.diagnostics.enabled {
            fn path(s: &rustx_core::Snapshot, id: NodeId, out: &mut Vec<NodeId>) -> bool {
                let n = out.len();
                out.push(s.id);
                if s.id == id || s.children.iter().any(|c| path(c, id, out)) {
                    true
                } else {
                    out.truncate(n);
                    false
                }
            }
            let target = if matches!(
                kind,
                rustx_core::EventKind::PointerMove
                    | rustx_core::EventKind::PointerUp
                    | rustx_core::EventKind::PointerCancel
            ) {
                captured.unwrap_or(id)
            } else {
                id
            };
            let mut nodes = vec![];
            path(&self.runtime.snapshot(), target, &mut nodes);
            nodes.truncate(128);
            self.diagnostics.event(kind, target, nodes, &result);
        }
        self.dirty = true;
        result
    }
    pub fn focus_node(&mut self, id: NodeId) {
        let snapshot = self.runtime.snapshot();
        fn find(s: &rustx_core::Snapshot, id: NodeId) -> Option<&rustx_core::Snapshot> {
            if s.id == id {
                Some(s)
            } else {
                s.children.iter().find_map(|c| find(c, id))
            }
        }
        if !find(&snapshot, id).is_some_and(|s| s.focusable()) {
            return;
        }
        if let Some(modal) = self.modal {
            if !find(&snapshot, modal).is_some_and(|s| find(s, id).is_some()) {
                return;
            }
        }
        if self.focus != Some(id) {
            self.text_drag = None;
            if let Some(old) = self.focus {
                self.runtime
                    .dispatch(old, rustx_core::EventData::new(rustx_core::EventKind::Blur));
            }
            self.runtime
                .dispatch(id, rustx_core::EventData::new(rustx_core::EventKind::Focus));
        }
        self.runtime.focus(id);
        self.focus = Some(id);
        self.dirty = true;
    }
    /// Hit-test the same shaped clusters used for Rust painting. The native or
    /// browser text control never measures visible geometry for this default.
    fn text_position_at(
        &mut self,
        id: NodeId,
        point: Point,
    ) -> Option<(rustx_core::Editor, usize)> {
        fn find(b: &BoxNode, id: NodeId) -> Option<&BoxNode> {
            if b.node.id == id {
                Some(b)
            } else {
                b.children.iter().find_map(|child| find(child, id))
            }
        }
        let b = self.boxes.as_ref().and_then(|b| find(b, id))?;
        if b.node.control.disabled || b.node.control.hidden || !b.node.control.options.is_empty() {
            return None;
        }
        let editor = self.runtime.editor(id)?;
        // Password offsets have a separate masked-to-source mapping. Keep the
        // existing platform selection path for those fields.
        if editor.is_password() {
            return None;
        }
        let content = &b.node.text;
        let style = &b.node.style;
        let shaped = self.text.shape_editing(
            content,
            (b.rect.width() as f32 - style.padding_horizontal()).max(1.),
            style,
        );
        let x = point.x - b.rect.x0 - f64::from(style.padding_left());
        let y = point.y - b.rect.y0 - f64::from(style.padding_top())
            + f64::from(b.node.editing.as_ref().map_or(0., |e| e.scroll_y));
        Some((editor, crate::editing::hit_offset(&shaped, content, x, y)))
    }
    fn move_vertical(
        &mut self,
        id: NodeId,
        node: &rustx_core::Snapshot,
        editor: &rustx_core::Editor,
        down: bool,
        extend: bool,
    ) {
        fn width(b: &BoxNode, id: NodeId) -> Option<f32> {
            if b.node.id == id {
                Some(b.rect.width() as f32 - b.node.style.padding_horizontal())
            } else {
                b.children.iter().find_map(|child| width(child, id))
            }
        }
        let width = self
            .boxes
            .as_ref()
            .and_then(|b| width(b, id))
            .unwrap_or(400.)
            .max(1.);
        let style = &node.style;
        let shaped = self.text.shape_editing(&node.text, width, style);
        let selection = editor.selection();
        let caret = crate::editing::caret_rect(&shaped, selection.focus, style.font_size);
        let x = self
            .preferred_caret_x
            .filter(|(owner, _, focus)| *owner == id && *focus == selection.focus)
            .map_or(caret.x0, |(_, x, _)| x);
        let mut lines: Vec<_> = shaped.lines.iter().map(|line| f64::from(line.y)).collect();
        lines.sort_by(f64::total_cmp);
        lines.dedup();
        let target = if down {
            lines.iter().copied().find(|y| *y > caret.y0 + 0.01)
        } else {
            lines.iter().copied().rev().find(|y| *y < caret.y0 - 0.01)
        };
        let offset = target.map_or(if down { node.text.len() } else { 0 }, |y| {
            crate::editing::hit_offset(&shaped, &node.text, x, y + f64::from(style.font_size) * 0.7)
        });
        let _ = editor.select(if extend { selection.anchor } else { offset }, offset);
        self.preferred_caret_x = Some((id, x, editor.selection().focus));
    }
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
    pub fn frame(&mut self, width: f32, height: f32, scale: f64) -> bool {
        let started = if self.diagnostics.enabled {
            diagnostics::now()
        } else {
            0.
        };
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
        let model_changed = self.model_epoch != self.runtime.model_epoch();
        self.model_epoch = self.runtime.model_epoch();
        let snapshot = self.runtime.snapshot();
        let committed = if self.diagnostics.enabled {
            diagnostics::now()
        } else {
            0.
        };
        fn accepts_focus(s: &rustx_core::Snapshot, id: NodeId) -> bool {
            if s.id == id {
                s.focusable()
            } else {
                s.children.iter().any(|child| accepts_focus(child, id))
            }
        }
        if self.focus.is_some_and(|id| !accepts_focus(&snapshot, id)) {
            self.focus = None;
        }
        if self.text_drag.is_some_and(|(_, id, _)| {
            !accepts_focus(&snapshot, id) || self.runtime.editor(id).is_none()
        }) {
            if let Some((pointer, id, _)) = self.text_drag.take() {
                self.runtime.dispatch(
                    id,
                    rustx_core::EventData::pointer(
                        rustx_core::EventKind::PointerCancel,
                        pointer,
                        0.,
                        0.,
                    ),
                );
            }
        }
        fn active_modal(s: &rustx_core::Snapshot) -> Option<&rustx_core::Snapshot> {
            if s.control.hidden {
                return None;
            }
            s.children
                .iter()
                .rev()
                .find_map(active_modal)
                .or_else(|| s.control.modal.then_some(s))
        }
        let modal = active_modal(&snapshot);
        if self.modal != modal.map(|s| s.id) {
            if let Some(modal) = modal {
                self.restore_focus = self.focus;
                self.modal = Some(modal.id);
                fn first(s: &rustx_core::Snapshot) -> Option<NodeId> {
                    if s.focusable() {
                        Some(s.id)
                    } else {
                        s.children.iter().find_map(first)
                    }
                }
                if let Some(id) = first(modal) {
                    self.focus_node(id);
                }
            } else {
                self.modal = None;
                if let Some(id) = self.restore_focus.take() {
                    self.focus_node(id);
                }
            }
            self.dirty = true;
        }
        let layout_changed = self.size != (width, height)
            || self.boxes.is_none()
            || commit.components_rendered > 0
            || model_changed
            || font_changed;
        let mut reasons = Vec::new();
        if self.diagnostics.enabled {
            if self.size != (width, height) {
                reasons.push("viewport");
            }
            if self.scale != scale {
                reasons.push("scale");
            }
            if model_changed {
                reasons.push("model");
            }
            if font_changed {
                reasons.push("fonts");
            }
            if resources_changed {
                reasons.push("assets");
            }
            if commit.components_rendered > 0 {
                reasons.push("components");
            }
            if self.dirty {
                reasons.push("paint");
            }
        }
        if layout_changed {
            self.layout.diagnostics = self.diagnostics.enabled;
            self.boxes = Some(
                self.layout
                    .compute(&snapshot, width, height, &mut self.text),
            );
            self.applied_scroll = 0.;
        }
        if !self.dirty && !layout_changed && !resources_changed && self.scale == scale {
            return false;
        }
        let laid_out = if self.diagnostics.enabled {
            diagnostics::now()
        } else {
            0.
        };
        self.size = (width, height);
        self.scale = scale;
        self.dirty = false;
        let root = self.boxes.as_mut().unwrap();
        self.scroll = self
            .scroll
            .clamp(0., (root.rect.height() - f64::from(height)).max(0.));
        root.scroll_translate(Vec2::new(0., self.applied_scroll - self.scroll));
        root.position_overlays(width as f64, height as f64);
        self.applied_scroll = self.scroll;
        fn viewports(
            boxes: &mut BoxNode,
            text: &mut TextEngine,
            runtime: &Runtime,
            focus: Option<NodeId>,
            revision: &mut Option<(NodeId, u64)>,
        ) {
            if let Some(state) = &mut boxes.node.editing {
                if let Some(editor) = runtime.editor(boxes.node.id) {
                    let s = &boxes.node.style;
                    let shaped = text.shape_editing(
                        &boxes.node.text,
                        (boxes.rect.width() as f32 - s.padding_horizontal()).max(1.),
                        s,
                    );
                    let height = (boxes.rect.height() as f32 - s.padding_vertical()).max(1.);
                    let changed = focus == Some(boxes.node.id)
                        && *revision != Some((boxes.node.id, state.revision));
                    let caret = changed.then(|| {
                        let c =
                            crate::editing::caret_rect(&shaped, state.selection.focus, s.font_size);
                        (c.y0 as f32, c.y1 as f32)
                    });
                    state.scroll_y =
                        editor.viewport((shaped.height - height).max(0.), caret, height);
                    if focus == Some(boxes.node.id) {
                        *revision = Some((boxes.node.id, state.revision));
                    }
                }
            }
            for child in &mut boxes.children {
                viewports(child, text, runtime, focus, revision);
            }
        }
        viewports(
            root,
            &mut self.text,
            &self.runtime,
            self.focus,
            &mut self.editor_revision,
        );
        let logical = self.painter.build(
            root,
            &mut self.text,
            &mut self.world,
            Rect::new(0., 0., f64::from(width), f64::from(height)),
            self.focus,
        );
        self.scene.reset();
        self.scene.append(&logical, Some(Affine::scale(scale)));
        // Notify once per actual committed offset change. A cancelled wheel or
        // a scroll request clamped at a boundary produces no Scroll event.
        fn collect_scroll(b: &BoxNode, offsets: &mut std::collections::HashMap<NodeId, f64>) {
            if let Some(offset) = b
                .node
                .control
                .scroll_offset
                .filter(|offset| offset.is_finite())
            {
                offsets.insert(b.node.id, offset);
            }
            for child in &b.children {
                collect_scroll(child, offsets);
            }
        }
        let mut offsets = std::collections::HashMap::new();
        collect_scroll(self.boxes.as_ref().unwrap(), &mut offsets);
        let mut changes = offsets
            .iter()
            .filter_map(|(id, offset)| {
                self.scroll_positions
                    .get(id)
                    .filter(|old| **old != *offset)
                    .map(|old| (*id, *offset - *old))
            })
            .collect::<Vec<_>>();
        self.scroll_positions = offsets;
        if self.scroll != self.committed_scroll {
            changes.push((self.host_root(), self.scroll - self.committed_scroll));
            self.committed_scroll = self.scroll;
        }
        for (id, delta) in changes {
            let mut event = rustx_core::EventData::new(rustx_core::EventKind::Scroll);
            event.delta_y = delta;
            self.runtime.dispatch(id, event);
        }
        if self.diagnostics.enabled {
            self.diagnostics.frame(
                committed - started,
                laid_out - committed,
                diagnostics::now() - laid_out,
                reasons,
                self.layout.changed,
                if layout_changed {
                    self.layout.measurements.clone()
                } else {
                    vec![]
                },
            );
        }
        true
    }
    pub fn pointer_down(&mut self, p: Point) {
        self.last_pointer = Some(p);
        self.dragged = false;
        self.pointer_default_prevented = false;
        self.pressed = self.painter.hit(p);
        if let Some(id) = self.pressed {
            let result = self.dispatch(
                id,
                rustx_core::EventData::pointer(rustx_core::EventKind::PointerDown, 0, p.x, p.y),
            );
            self.pointer_default_prevented = result.default_prevented;
            if !result.default_prevented {
                self.focus_node(id);
            }
        }
        self.dirty = true;
    }
    pub fn pointer_move(&mut self, p: Point) {
        let hovered = self.painter.hit(p);
        if hovered != self.hovered {
            if let Some(old) = self.hovered {
                self.dispatch(
                    old,
                    rustx_core::EventData::pointer(
                        rustx_core::EventKind::PointerLeave,
                        0,
                        p.x,
                        p.y,
                    ),
                );
            }
            if let Some(new) = hovered {
                self.dispatch(
                    new,
                    rustx_core::EventData::pointer(
                        rustx_core::EventKind::PointerEnter,
                        0,
                        p.x,
                        p.y,
                    ),
                );
            }
            self.hovered = hovered;
        }
        if let Some(id) = self
            .runtime
            .captured_pointer(0)
            .or(self.pressed)
            .or_else(|| self.painter.hit(p))
        {
            if self
                .dispatch(
                    id,
                    rustx_core::EventData::pointer(rustx_core::EventKind::PointerMove, 0, p.x, p.y),
                )
                .default_prevented
            {
                self.last_pointer = Some(p);
                return;
            }
        }
        if let (Some(id), Some(last)) = (self.pressed, self.last_pointer) {
            if self.pointer_default_prevented {
                self.last_pointer = Some(p);
                return;
            }
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
            .runtime
            .captured_pointer(0)
            .or(self.pressed)
            .or_else(|| self.painter.hit(p))
        {
            if self
                .dispatch(
                    id,
                    rustx_core::EventData::pointer(rustx_core::EventKind::PointerUp, 0, p.x, p.y),
                )
                .default_prevented
            {
                self.pressed = None;
                return;
            }
        }
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
                self.dispatch(
                    id,
                    rustx_core::EventData::pointer(rustx_core::EventKind::Click, 0, p.x, p.y),
                );
            }
        }
        self.dirty = true;
    }
    pub fn cancel(&mut self) {
        if let Some(id) = self.runtime.captured_pointer(0).or(self.pressed) {
            self.runtime.dispatch(
                id,
                rustx_core::EventData::new(rustx_core::EventKind::PointerCancel),
            );
        }
        self.pressed = None;
        self.last_pointer = None;
        self.dragged = false;
        self.pointer_default_prevented = false;
        self.text_drag = None;
    }
    fn slider_at(&mut self, id: NodeId, p: Point) -> bool {
        fn find(b: &BoxNode, id: NodeId) -> Option<&BoxNode> {
            if b.node.id == id {
                Some(b)
            } else {
                b.children.iter().find_map(|c| find(c, id))
            }
        }
        let Some(b) = self.boxes.as_ref().and_then(|b| find(b, id)) else {
            return false;
        };
        if b.node.role != Some("slider") || b.node.control.disabled || b.node.control.read_only {
            return false;
        }
        let min = b.node.control.min.unwrap_or(0.);
        let max = b.node.control.max.unwrap_or(100.).max(min);
        let step = b.node.control.step.unwrap_or(1.);
        let mut fraction = ((p.x - b.rect.x0 - 12.) / (b.rect.width() - 24.).max(1.)).clamp(0., 1.);
        if b.node.style.text_direction == rustx_core::TextDirection::RightToLeft {
            fraction = 1. - fraction;
        }
        let value = (min + ((max - min) * fraction / step).round() * step).clamp(min, max);
        self.runtime.input(id, value.to_string(), false);
        self.dirty = true;
        true
    }
    pub fn tab(&mut self, back: bool) {
        let snapshot = self.runtime.snapshot();
        fn find(s: &rustx_core::Snapshot, id: NodeId) -> Option<&rustx_core::Snapshot> {
            if s.id == id {
                Some(s)
            } else {
                s.children.iter().find_map(|c| find(c, id))
            }
        }
        fn collect(s: &rustx_core::Snapshot, out: &mut Vec<NodeId>) {
            if s.control.hidden {
                return;
            }
            if s.focusable() && s.control.tab_index != Some(-1) {
                out.push(s.id);
            }
            for c in &s.children {
                collect(c, out);
            }
        }
        let mut ids = vec![];
        let root = self
            .modal
            .and_then(|id| find(&snapshot, id))
            .unwrap_or(&snapshot);
        collect(root, &mut ids);
        if !ids.is_empty() {
            let i = ids.iter().position(|id| Some(*id) == self.focus);
            let next = if back {
                i.map_or(ids.len() - 1, |i| (i + ids.len() - 1) % ids.len())
            } else {
                i.map_or(0, |i| (i + 1) % ids.len())
            };
            self.focus_node(ids[next]);
            self.dirty = true;
        }
    }
    pub fn activate(&mut self) {
        if let Some(id) = self.focus {
            self.dispatch(id, rustx_core::EventData::new(rustx_core::EventKind::Click));
            self.dirty = true;
        }
    }
    /// Native keyboard entry point; browser bridges dispatch first and then call
    /// `key_target` only when the synchronous event did not cancel its default.
    pub fn key(&mut self, key: &str, shift: bool, ctrl: bool, alt: bool, meta: bool) -> bool {
        if key == "Tab" && self.focus.is_none() {
            self.tab(shift);
            return true;
        }
        let Some(id) = self.focus else {
            return false;
        };
        let mut event = rustx_core::EventData::key(key);
        event.shift = shift;
        event.ctrl = ctrl;
        event.alt = alt;
        event.meta = meta;
        let result = self.runtime.dispatch(id, event);
        if result.default_prevented {
            self.dirty = true;
            return true;
        }
        if self.key_target(id, key, shift, ctrl, alt, meta) {
            return true;
        }
        if !ctrl && !meta && !alt && key.chars().count() == 1 {
            return self.insert_text(key);
        }
        false
    }
    pub fn insert_text(&mut self, value: &str) -> bool {
        let Some(id) = self.focus else {
            return false;
        };
        let Some(editor) = self.runtime.editor(id) else {
            return false;
        };
        if editor.insert(value).is_err() {
            return false;
        }
        self.runtime
            .input(id, editor.value(), editor.is_composing());
        self.runtime.invalidate_model();
        self.dirty = true;
        true
    }
    pub fn key_target(
        &mut self,
        id: NodeId,
        key: &str,
        shift: bool,
        ctrl: bool,
        alt: bool,
        meta: bool,
    ) -> bool {
        if alt {
            return false;
        }
        if ctrl || meta || !matches!(key, "ArrowUp" | "ArrowDown") {
            self.preferred_caret_x = None;
        }
        if key == "Tab" && !ctrl && !meta {
            self.tab(shift);
            return true;
        }
        let snapshot = self.runtime.snapshot();
        fn path<'a>(
            s: &'a rustx_core::Snapshot,
            id: NodeId,
            out: &mut Vec<&'a rustx_core::Snapshot>,
        ) -> bool {
            let n = out.len();
            out.push(s);
            if s.id == id || s.children.iter().any(|c| path(c, id, out)) {
                true
            } else {
                out.truncate(n);
                false
            }
        }
        let mut nodes = vec![];
        if !path(&snapshot, id, &mut nodes) {
            return false;
        }
        let node = *nodes.last().unwrap();
        if node.control.disabled || node.control.hidden {
            return false;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if node.role == Some("link") && key == "Enter" && !ctrl && !meta {
            return self
                .dispatch(id, rustx_core::EventData::new(rustx_core::EventKind::Click))
                .handled;
        }
        if node.kind == rustx_core::Kind::Canvas && !ctrl && !meta {
            let delta = match key {
                "ArrowLeft" => Some((40., 0.)),
                "ArrowRight" => Some((-40., 0.)),
                "ArrowUp" => Some((0., 40.)),
                "ArrowDown" => Some((0., -40.)),
                _ => None,
            };
            if let Some((dx, dy)) = delta {
                self.runtime
                    .canvas(id, rustx_core::drawing::CanvasEvent::Pan { dx, dy });
                self.dirty = true;
                return true;
            }
        }
        if let Some(editor) = self.runtime.editor(id) {
            if editor.is_composing() {
                return false;
            }
            let before = editor.value();
            let handled = if ctrl || meta {
                match key.to_lowercase().as_str() {
                    "a" => {
                        editor.select_all();
                        true
                    }
                    "z" => {
                        if shift {
                            editor.redo();
                        } else {
                            editor.undo();
                        }
                        true
                    }
                    "y" => {
                        editor.redo();
                        true
                    }
                    "arrowleft" | "arrowright" if ctrl && !meta => {
                        editor.move_word(
                            if key == "ArrowLeft" {
                                rustx_core::MoveDirection::Backward
                            } else {
                                rustx_core::MoveDirection::Forward
                            },
                            shift,
                        );
                        true
                    }
                    "home" | "end" if ctrl && !meta => {
                        editor.move_grapheme(
                            if key == "Home" {
                                rustx_core::MoveDirection::Start
                            } else {
                                rustx_core::MoveDirection::End
                            },
                            shift,
                        );
                        true
                    }
                    _ => false,
                }
            } else {
                match key {
                    "Backspace" => {
                        let _ = editor.delete_backward();
                        true
                    }
                    "Delete" => {
                        let _ = editor.delete_forward();
                        true
                    }
                    "ArrowUp" | "ArrowDown" if node.control.multiline && !editor.is_password() => {
                        self.move_vertical(id, node, &editor, key == "ArrowDown", shift);
                        true
                    }
                    "ArrowLeft" | "ArrowRight" => {
                        let direction = if key == "ArrowLeft" {
                            rustx_core::MoveDirection::Backward
                        } else {
                            rustx_core::MoveDirection::Forward
                        };
                        // Visual boundaries come from shared shaped glyph clusters.
                        let width = self
                            .boxes
                            .as_ref()
                            .and_then(|b| {
                                fn find(b: &BoxNode, id: NodeId) -> Option<f32> {
                                    if b.node.id == id {
                                        Some(
                                            b.rect.width() as f32
                                                - b.node.style.padding_horizontal(),
                                        )
                                    } else {
                                        b.children.iter().find_map(|c| find(c, id))
                                    }
                                }
                                find(b, id)
                            })
                            .unwrap_or(400.);
                        let shaped =
                            self.text
                                .shape_editing(&node.text, width.max(1.), &node.style);
                        let mut stops = vec![];
                        for c in &shaped.clusters {
                            stops.push((c.y, c.x, if c.rtl { c.end } else { c.start }));
                            stops.push((c.y, c.x + c.width, if c.rtl { c.start } else { c.end }));
                        }
                        stops.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
                        stops.dedup_by(|a, b| a.2 == b.2 && a.0 == b.0 && a.1 == b.1);
                        let selection = editor.selection();
                        if !node.control.password() {
                            if let Some(at) = stops.iter().position(|s| s.2 == selection.focus) {
                                let next = if key == "ArrowLeft" {
                                    at.saturating_sub(1)
                                } else {
                                    (at + 1).min(stops.len() - 1)
                                };
                                let focus = stops[next].2;
                                let _ = editor
                                    .select(if shift { selection.anchor } else { focus }, focus);
                            } else {
                                editor.move_grapheme(direction, shift);
                            }
                        } else {
                            editor.move_grapheme(direction, shift);
                        }
                        true
                    }
                    "Home" => {
                        editor.move_grapheme(rustx_core::MoveDirection::Start, shift);
                        true
                    }
                    "End" => {
                        editor.move_grapheme(rustx_core::MoveDirection::End, shift);
                        true
                    }
                    "Enter" if node.control.multiline => {
                        let _ = editor.insert("\n");
                        true
                    }
                    "Enter" => {
                        if let Some(form) = nodes.iter().rev().find(|s| s.role == Some("form")) {
                            self.runtime.dispatch(
                                form.id,
                                rustx_core::EventData::new(rustx_core::EventKind::Submit),
                            );
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            };
            if handled {
                if editor.value() != before {
                    self.runtime.input(id, editor.value(), false);
                }
                self.runtime.invalidate_model();
                self.dirty = true;
            }
            return handled;
        }
        if ctrl || meta {
            return false;
        }
        if matches!(
            key,
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Home" | "End"
        ) {
            if let Some(group) = nodes
                .iter()
                .rev()
                .find(|s| matches!(s.role, Some("tablist" | "menu" | "radiogroup")))
            {
                let mut items = vec![];
                fn collect(s: &rustx_core::Snapshot, out: &mut Vec<NodeId>) {
                    if s.control.hidden || s.control.disabled {
                        return;
                    }
                    if s.focusable() {
                        out.push(s.id);
                    }
                    for c in &s.children {
                        collect(c, out);
                    }
                }
                for c in &group.children {
                    collect(c, &mut items);
                }
                if !items.is_empty() {
                    let at = items.iter().position(|x| *x == id).unwrap_or(0);
                    let backward = matches!(key, "ArrowLeft" | "ArrowUp")
                        ^ (matches!(key, "ArrowLeft" | "ArrowRight")
                            && group.style.text_direction
                                == rustx_core::TextDirection::RightToLeft);
                    let next = match key {
                        "Home" => 0,
                        "End" => items.len() - 1,
                        _ if backward => (at + items.len() - 1) % items.len(),
                        _ => (at + 1) % items.len(),
                    };
                    self.focus_node(items[next]);
                    if group.role != Some("menu") {
                        self.runtime.click(items[next], 0., 0.);
                    }
                    return true;
                }
            }
        }
        false
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
        if let Some(id) = self.painter.hit(p) {
            let mut event = rustx_core::EventData::new(rustx_core::EventKind::Wheel);
            event.x = p.x;
            event.y = p.y;
            event.delta_y = dy;
            if self.runtime.dispatch(id, event).default_prevented {
                return;
            }
        }
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

impl Engine {
    fn host_root(&self) -> NodeId {
        let snapshot = self.runtime.snapshot();
        let mut root = &snapshot;
        while root.kind == rustx_core::Kind::Fragment && !root.children.is_empty() {
            root = &root.children[0];
        }
        root.id
    }
    /// Platform notifications share the committed Rust event path. Drops prefer
    /// the hit target, then focus; viewport/resource notifications target root.
    pub fn host_event(&mut self, event: rustx_core::EventData) -> rustx_core::DispatchResult {
        let target = if event.kind == rustx_core::EventKind::FileDrop {
            self.painter
                .hit(Point::new(event.x, event.y))
                .or(self.focus)
        } else {
            None
        }
        .unwrap_or_else(|| self.host_root());
        self.dispatch(target, event)
    }
}
