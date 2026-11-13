//! Taffy nodes retain runtime identity; only changed styles/content/children invalidate layout.
use crate::text::TextEngine;
use rustx_core::{Direction, Display, Kind, NodeId, Snapshot, Theme};
use std::collections::{HashMap, HashSet};
use taffy::prelude::*;
use vello::kurbo::{Rect, Vec2};
#[derive(Clone, Debug)]
pub struct BoxNode {
    pub node: Snapshot,
    pub rect: Rect,
    pub children: Vec<BoxNode>,
}
impl BoxNode {
    pub fn find(&self, id: &str) -> Option<&Self> {
        if self.node.app_id.as_deref() == Some(id) {
            Some(self)
        } else {
            self.children.iter().find_map(|c| c.find(id))
        }
    }
    pub fn translate(&mut self, offset: Vec2) {
        self.rect = self.rect + offset;
        for c in &mut self.children {
            c.translate(offset);
        }
    }
    pub fn scroll_translate(&mut self, offset: Vec2) {
        if self.node.control.overlay {
            return;
        }
        self.rect = self.rect + offset;
        for child in &mut self.children {
            child.scroll_translate(offset);
        }
    }
    pub fn position_overlays(&mut self, width: f64, height: f64) {
        fn index(b: &BoxNode, anchors: &mut HashMap<String, Rect>) {
            if let Some(id) = &b.node.app_id {
                anchors.insert(id.clone(), b.rect);
            }
            for child in &b.children {
                index(child, anchors);
            }
        }
        let mut anchors = HashMap::new();
        index(self, &mut anchors);
        for layer in &mut self.children {
            if !layer.node.control.overlay {
                continue;
            }
            let target = if layer.node.control.modal {
                Some((
                    ((width - layer.rect.width()) * 0.5).max(8.),
                    ((height - layer.rect.height()) * 0.5).max(8.),
                ))
            } else {
                layer
                    .node
                    .control
                    .anchor
                    .as_ref()
                    .and_then(|id| anchors.get(id))
                    .map(|anchor| {
                        (
                            anchor
                                .x0
                                .clamp(8., (width - layer.rect.width() - 8.).max(8.)),
                            (if anchor.y1 + layer.rect.height() + 4. <= height {
                                anchor.y1 + 4.
                            } else {
                                (anchor.y0 - layer.rect.height() - 4.).max(8.)
                            })
                            .clamp(8., (height - layer.rect.height() - 8.).max(8.)),
                        )
                    })
                    .or_else(|| {
                        Some((
                            layer
                                .rect
                                .x0
                                .clamp(8., (width - layer.rect.width() - 8.).max(8.)),
                            layer
                                .rect
                                .y0
                                .clamp(8., (height - layer.rect.height() - 8.).max(8.)),
                        ))
                    })
            };
            if let Some((x, y)) = target {
                layer.translate(Vec2::new(x - layer.rect.x0, y - layer.rect.y0));
                index(layer, &mut anchors);
            }
        }
    }
}
#[derive(Clone, PartialEq)]
struct TextContext {
    text: String,
    size: f32,
    align: rustx_core::TextAlign,
    direction: rustx_core::TextDirection,
    typography: rustx_core::Typography,
    epoch: u64,
}
struct Entry {
    id: taffy::NodeId,
    context: Option<TextContext>,
}
pub struct LayoutEngine {
    tree: TaffyTree<TextContext>,
    nodes: HashMap<NodeId, Entry>,
    pub changed: usize,
    pub theme: Theme,
    pub font_epoch: u64,
    pub diagnostics: bool,
    pub measurements: Vec<serde_json::Value>,
}
impl Default for LayoutEngine {
    fn default() -> Self {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        Self {
            tree,
            nodes: HashMap::new(),
            changed: 0,
            theme: Theme::DARK,
            font_epoch: 0,
            diagnostics: false,
            measurements: vec![],
        }
    }
}
fn flatten(n: &Snapshot, out: &mut Vec<Snapshot>) {
    if n.control.hidden {
        return;
    }
    if n.kind == Kind::Fragment {
        for c in &n.children {
            flatten(c, out)
        }
    } else {
        out.push(n.clone())
    }
}
fn children(n: &Snapshot) -> Vec<Snapshot> {
    let mut out = vec![];
    if !matches!(
        n.kind,
        Kind::Text | Kind::Button | Kind::Input | Kind::Image | Kind::Canvas
    ) {
        for c in &n.children {
            flatten(c, &mut out)
        }
    }
    out
}
impl LayoutEngine {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    fn sync(
        &mut self,
        node: &Snapshot,
        width: f32,
        seen: &mut HashSet<NodeId>,
        stack_cell: bool,
    ) -> taffy::NodeId {
        seen.insert(node.id);
        let s = &node.style;
        let padding = s.padding_insets();
        let margin = s.margin_insets();
        let dimension = |legacy: Option<f32>, value: rustx_core::Length| match legacy {
            Some(value) => length(value),
            None => match value {
                rustx_core::Length::Auto => auto(),
                rustx_core::Length::Px(value) => length(value.max(0.)),
                rustx_core::Length::Percent(value) => percent(value.max(0.) / 100.),
            },
        };
        let offset = |value: rustx_core::Length| -> LengthPercentageAuto {
            match value {
                rustx_core::Length::Auto => auto(),
                rustx_core::Length::Px(value) => length(value),
                rustx_core::Length::Percent(value) => percent(value / 100.),
            }
        };
        let label_inset = crate::controls::label_inset(node);
        let columns = if s.min_column > 0. {
            (((width - s.padding_horizontal() + s.gap) / (s.min_column + s.gap)).floor() as u16)
                .clamp(1, s.columns)
        } else {
            s.columns
        };
        let style = taffy::Style {
            position: if node.control.overlay || s.position == rustx_core::Position::Absolute {
                Position::Absolute
            } else {
                Position::Relative
            },
            inset: taffy::Rect {
                top: offset(s.inset[0]),
                right: offset(s.inset[1]),
                bottom: offset(s.inset[2]),
                left: offset(s.inset[3]),
            },
            display: match s.display {
                Display::Flex => taffy::Display::Flex,
                Display::Grid => taffy::Display::Grid,
                Display::Block => taffy::Display::Block,
            },
            flex_direction: if s.direction == Direction::Row {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
            flex_wrap: if s.wrap {
                FlexWrap::Wrap
            } else {
                FlexWrap::NoWrap
            },
            flex_grow: s.grow,
            flex_shrink: s.shrink,
            align_items: Some(match s.align {
                rustx_core::Align::Start => AlignItems::Start,
                rustx_core::Align::Center => AlignItems::Center,
                rustx_core::Align::End => AlignItems::End,
                rustx_core::Align::Stretch => AlignItems::Stretch,
            }),
            justify_content: Some(match s.justify {
                rustx_core::Justify::Start => JustifyContent::Start,
                rustx_core::Justify::Center => JustifyContent::Center,
                rustx_core::Justify::End => JustifyContent::End,
                rustx_core::Justify::SpaceBetween => JustifyContent::SpaceBetween,
            }),
            size: Size {
                width: dimension(s.width, s.width_length),
                height: dimension(s.height, s.height_length),
            },
            min_size: Size {
                width: length(s.min_width.unwrap_or(0.).max(if label_inset > 0. {
                    s.padding_horizontal() + label_inset
                } else {
                    0.
                })),
                height: length(s.min_height.unwrap_or(0.).max(if label_inset > 0. {
                    s.padding_vertical() + 18.
                } else {
                    0.
                })),
            },
            max_size: Size {
                width: s
                    .max_width
                    .or_else(|| node.control.overlay.then_some((width - 16.).max(1.)))
                    .map(length)
                    .unwrap_or(auto()),
                height: s.max_height.map(length).unwrap_or(auto()),
            },
            padding: taffy::Rect {
                left: length(padding.left + label_inset),
                right: length(padding.right),
                top: length(padding.top),
                bottom: length(padding.bottom),
            },
            margin: taffy::Rect {
                left: length(margin.left),
                right: length(margin.right),
                top: length(margin.top),
                bottom: length(margin.bottom),
            },
            gap: Size {
                width: length(s.gap),
                height: length(s.gap),
            },
            grid_template_columns: vec![fr(1.); usize::from(columns)],
            grid_row: if stack_cell { line(1) } else { auto() },
            grid_column: if stack_cell { line(1) } else { auto() },
            overflow: taffy::Point {
                x: if s.clip {
                    taffy::Overflow::Hidden
                } else {
                    taffy::Overflow::Visible
                },
                y: if s.clip {
                    taffy::Overflow::Hidden
                } else {
                    taffy::Overflow::Visible
                },
            },
            ..Default::default()
        };
        let context =
            matches!(node.kind, Kind::Text | Kind::Button | Kind::Input).then(|| TextContext {
                text: node.text_content(),
                size: s.font_size,
                align: s.text_align,
                direction: s.text_direction,
                typography: TextEngine::node_typography(node),
                epoch: self.font_epoch,
            });
        let child_ids: Vec<_> = children(node)
            .iter()
            .map(|c| {
                self.sync(
                    c,
                    s.width
                        .or_else(|| s.width_length.resolve(width))
                        .unwrap_or(width)
                        - s.padding_horizontal(),
                    seen,
                    node.control.stack,
                )
            })
            .collect();
        if let Some(e) = self.nodes.get_mut(&node.id) {
            if self.tree.style(e.id).unwrap() != &style {
                self.tree.set_style(e.id, style).unwrap();
                self.changed += 1;
            }
            if e.context != context {
                self.tree.set_node_context(e.id, context.clone()).unwrap();
                e.context = context;
                self.changed += 1;
            }
            if self.tree.children(e.id).unwrap() != child_ids {
                self.tree.set_children(e.id, &child_ids).unwrap();
                self.changed += 1;
            }
            e.id
        } else {
            let id = self.tree.new_with_children(style, &child_ids).unwrap();
            self.tree.set_node_context(id, context.clone()).unwrap();
            self.nodes.insert(node.id, Entry { id, context });
            self.changed += 1;
            id
        }
    }
    pub fn compute(
        &mut self,
        snapshot: &Snapshot,
        width: f32,
        height: f32,
        text: &mut TextEngine,
    ) -> BoxNode {
        let mut responsive = snapshot.clone();
        fn resolve(n: &mut Snapshot, width: f32, theme: Theme) {
            n.style = n.style.resolve(width, theme);
            for c in &mut n.children {
                resolve(c, width, theme);
            }
        }
        resolve(&mut responsive, width, self.theme);
        let snapshot = &responsive;
        let mut roots = vec![];
        flatten(snapshot, &mut roots);
        let mut root = snapshot.clone();
        root.id = 0;
        root.kind = Kind::Div;
        root.text.clear();
        root.children = roots;
        root.style = rustx_core::Style::column().gap(0.).width(width);
        // Overlay geometry is measured against the viewport, not a clipped or
        // narrow logical parent. Runtime/semantic ownership stays untouched.
        fn detach(node: &mut Snapshot, layers: &mut Vec<Snapshot>) {
            let mut retained = vec![];
            for mut child in std::mem::take(&mut node.children) {
                let mut nested = vec![];
                detach(&mut child, &mut nested);
                if child.control.overlay {
                    layers.push(child);
                } else {
                    retained.push(child);
                }
                layers.extend(nested);
            }
            node.children = retained;
        }
        let mut layers = vec![];
        detach(&mut root, &mut layers);
        root.children.extend(layers);
        self.changed = 0;
        let mut seen = HashSet::new();
        let id = self.sync(&root, width, &mut seen, false);
        let stale: Vec<_> = self
            .nodes
            .keys()
            .filter(|id| !seen.contains(id))
            .copied()
            .collect();
        for id in stale {
            let e = self.nodes.remove(&id).unwrap();
            let _ = self.tree.remove(e.id);
        }
        self.measurements.clear();
        let measured_nodes = if self.diagnostics {
            self.nodes
                .iter()
                .map(|(node, entry)| (entry.id, *node))
                .collect::<HashMap<_, _>>()
        } else {
            HashMap::new()
        };
        let mut timings = HashMap::<NodeId, (f64, usize)>::new();
        self.tree
            .compute_layout_with_measure(
                id,
                Size {
                    width: AvailableSpace::Definite(width),
                    height: AvailableSpace::Definite(height),
                },
                |known, available, id, context, _| {
                    let Some(context) = context else {
                        return Size::ZERO;
                    };
                    let width = known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(v) => v,
                        AvailableSpace::MinContent => 1.,
                        AvailableSpace::MaxContent => 1_000_000.,
                    });
                    let started = self.diagnostics.then(crate::diagnostics::now);
                    let shaped = text.shape_with_typography(
                        &context.text,
                        context.size,
                        width,
                        context.align,
                        context.direction,
                        &context.typography,
                    );
                    if let Some(started) = started {
                        if let Some(node) = measured_nodes.get(&id) {
                            let entry = timings.entry(*node).or_default();
                            entry.0 += (crate::diagnostics::now() - started).max(0.);
                            entry.1 += 1;
                        }
                    }
                    Size {
                        width: known.width.unwrap_or(shaped.width.ceil()),
                        height: known.height.unwrap_or(shaped.height.ceil()),
                    }
                },
            )
            .unwrap();
        let mut timings = timings.into_iter().collect::<Vec<_>>();
        timings.sort_by(|a, b| b.1 .0.total_cmp(&a.1 .0));
        self.measurements=timings.into_iter().take(32).map(|(node,(ms,calls))|serde_json::json!({"node":node,"text_measure_ms":ms,"calls":calls})).collect();
        let mut output = self.output(&root, 0., 0.);
        output.position_overlays(width as f64, height as f64);
        output
    }
    fn output(&self, node: &Snapshot, x: f64, y: f64) -> BoxNode {
        let l = self.tree.layout(self.nodes[&node.id].id).unwrap();
        let x = x + f64::from(l.location.x + node.style.translate.0);
        let y = y + f64::from(l.location.y + node.style.translate.1);
        let mut children = children(node)
            .iter()
            .map(|child| self.output(child, x, y))
            .collect::<Vec<_>>();
        // Paint and hit testing share this order; runtime ownership and Tab order
        // retain their original construction order. Overlay layers remain on top.
        if children.iter().any(|child| child.node.style.z_index != 0) {
            children.sort_by_key(|child| (child.node.control.overlay, child.node.style.z_index));
        }
        BoxNode {
            node: node.clone(),
            rect: Rect::new(
                x,
                y,
                x + f64::from(l.size.width),
                y + f64::from(l.size.height),
            ),
            children,
        }
    }
}
