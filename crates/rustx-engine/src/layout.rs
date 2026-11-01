//! Taffy nodes retain runtime identity; only changed styles/content/children invalidate layout.
use crate::text::TextEngine;
use rustx_core::{Direction, Display, Kind, NodeId, Snapshot};
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
}
#[derive(Clone, PartialEq)]
struct TextContext {
    text: String,
    size: f32,
}
struct Entry {
    id: taffy::NodeId,
    context: Option<TextContext>,
}
pub struct LayoutEngine {
    tree: TaffyTree<TextContext>,
    nodes: HashMap<NodeId, Entry>,
    pub changed: usize,
}
impl Default for LayoutEngine {
    fn default() -> Self {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        Self {
            tree,
            nodes: HashMap::new(),
            changed: 0,
        }
    }
}
fn flatten(n: &Snapshot, out: &mut Vec<Snapshot>) {
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
    fn sync(&mut self, node: &Snapshot, width: f32, seen: &mut HashSet<NodeId>) -> taffy::NodeId {
        seen.insert(node.id);
        let s = &node.style;
        let columns = if s.min_column > 0. {
            (((width - 2. * s.padding + s.gap) / (s.min_column + s.gap)).floor() as u16)
                .clamp(1, s.columns)
        } else {
            s.columns
        };
        let style = taffy::Style {
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
            flex_shrink: 0.,
            size: Size {
                width: s.width.map(length).unwrap_or(auto()),
                height: s.height.map(length).unwrap_or(auto()),
            },
            min_size: Size {
                width: length(0.),
                height: length(0.),
            },
            padding: taffy::Rect {
                left: length(s.padding),
                right: length(s.padding),
                top: length(s.padding),
                bottom: length(s.padding),
            },
            gap: Size {
                width: length(s.gap),
                height: length(s.gap),
            },
            grid_template_columns: vec![fr(1.); usize::from(columns)],
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
            });
        let child_ids: Vec<_> = children(node)
            .iter()
            .map(|c| self.sync(c, s.width.unwrap_or(width) - 2. * s.padding, seen))
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
        fn resolve(n: &mut Snapshot, width: f32) {
            if width < 600. {
                if let Some(v) = n.style.small_font_size {
                    n.style.font_size = v;
                }
            }
            for c in &mut n.children {
                resolve(c, width);
            }
        }
        resolve(&mut responsive, width);
        let snapshot = &responsive;
        let mut roots = vec![];
        flatten(snapshot, &mut roots);
        let mut root = snapshot.clone();
        root.id = 0;
        root.kind = Kind::Div;
        root.text.clear();
        root.children = roots;
        root.style = rustx_core::Style::column().gap(0.).width(width);
        self.changed = 0;
        let mut seen = HashSet::new();
        let id = self.sync(&root, width, &mut seen);
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
        self.tree
            .compute_layout_with_measure(
                id,
                Size {
                    width: AvailableSpace::Definite(width),
                    height: AvailableSpace::Definite(height),
                },
                |known, available, _, context, _| {
                    let Some(context) = context else {
                        return Size::ZERO;
                    };
                    let width = known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(v) => v,
                        AvailableSpace::MinContent => 1.,
                        AvailableSpace::MaxContent => 1_000_000.,
                    });
                    let shaped = text.shape(&context.text, context.size, width);
                    Size {
                        width: known.width.unwrap_or(shaped.width.ceil()),
                        height: known.height.unwrap_or(shaped.height.ceil()),
                    }
                },
            )
            .unwrap();
        self.output(&root, 0., 0.)
    }
    fn output(&self, node: &Snapshot, x: f64, y: f64) -> BoxNode {
        let l = self.tree.layout(self.nodes[&node.id].id).unwrap();
        let x = x + f64::from(l.location.x + node.style.translate.0);
        let y = y + f64::from(l.location.y + node.style.translate.1);
        BoxNode {
            node: node.clone(),
            rect: Rect::new(
                x,
                y,
                x + f64::from(l.size.width),
                y + f64::from(l.size.height),
            ),
            children: children(node)
                .iter()
                .map(|c| self.output(c, x, y))
                .collect(),
        }
    }
}
