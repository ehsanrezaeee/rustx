//! Fixed-height virtualization with bounded scopes and one retained focused row.
use crate::*;
#[derive(Clone)]
pub struct VirtualListState {
    pub offset: Signal<f64>,
    pub focused: Signal<Option<usize>>,
    pub count: usize,
    pub row_height: f32,
    pub height: f32,
}
impl VirtualListState {
    pub fn new(cx: &Scope, count: usize, row_height: f32, height: f32) -> Self {
        assert!(
            count <= 1_000_000
                && row_height.is_finite()
                && row_height >= 1.
                && height.is_finite()
                && height >= 1.
        );
        Self {
            offset: cx.signal(0.),
            focused: cx.signal(None),
            count,
            row_height,
            height,
        }
    }
    pub fn scroll_to(&self, offset: f64) {
        if offset.is_finite() {
            let _ = self.offset.set(offset.clamp(
                0.,
                (self.count as f64 * self.row_height as f64 - self.height as f64).max(0.),
            ));
        }
    }
    pub fn scroll_by(&self, delta: f64) {
        self.scroll_to(self.offset.get() + delta);
    }
    pub fn effective_offset(&self) -> f64 {
        let offset = self.offset.get();
        if !offset.is_finite() {
            0.
        } else {
            offset.clamp(
                0.,
                (self.count as f64 * self.row_height as f64 - self.height as f64).max(0.),
            )
        }
    }
    pub fn range(&self) -> std::ops::Range<usize> {
        let offset = self.effective_offset();
        let visible = (self.height / self.row_height).ceil() as usize;
        let first = (offset / self.row_height as f64).floor() as usize;
        first.saturating_sub(visible * 2).min(self.count)..(first + visible * 3 + 1).min(self.count)
    }
}
pub fn virtual_list(state: VirtualListState, render: impl Fn(usize) -> View + 'static) -> View {
    let render = std::rc::Rc::new(render);
    dynamic(move || {
        let mut indices: Vec<_> = state.range().collect();
        if let Some(focus) = state.focused.get().filter(|i| *i < state.count) {
            indices.push(focus);
        }
        indices.sort_unstable();
        indices.dedup();
        let mut children = vec![];
        let mut cursor = 0;
        for index in indices {
            if index > cursor {
                children.push(
                    Node::new(Kind::Div).key(format!("gap-{cursor}")).style(
                        Style::column()
                            .gap(0.)
                            .height((index - cursor) as f32 * state.row_height),
                    ),
                );
            }
            let draw = render.clone();
            let height = state.row_height;
            let total = state.count;
            let focus = state.focused.clone();
            let list = state.clone();
            children.push(
                Node::component("VirtualRow", move |_| {
                    Node::new(Kind::Div)
                        .role("listitem")
                        .position(index + 1, total)
                        .style(Style::column().height(height).gap(0.).clip())
                        .on_focus({
                            let focus = focus.clone();
                            let list = list.clone();
                            move || {
                                let _ = focus.set(Some(index));
                                let row = index as f64 * height as f64;
                                let offset = list.effective_offset();
                                if row < offset {
                                    list.scroll_to(row);
                                } else if row + height as f64 > offset + list.height as f64 {
                                    list.scroll_to(row + height as f64 - list.height as f64);
                                }
                            }
                        })
                        .children(vec![draw(index)])
                })
                .key(format!("row-{index}")),
            );
            cursor = index + 1;
        }
        if cursor < state.count {
            children.push(
                Node::new(Kind::Div).key("tail").style(
                    Style::column().height((state.count - cursor) as f32 * state.row_height),
                ),
            );
        }
        let mut node = Node::new(Kind::Div)
            .role("list")
            .label(format!("{} records", state.count))
            .style(Style::column().gap(0.).height(state.height).clip())
            .on_scroll({
                let state = state.clone();
                move |delta| state.scroll_by(delta)
            })
            .children(vec![Node::new(Kind::Div)
                .style(
                    Style::column()
                        .gap(0.)
                        .height((state.count as f32 * state.row_height).max(1.))
                        .translate(0., -state.effective_offset() as f32),
                )
                .children(children)]);
        node.control.scroll_offset = Some(state.effective_offset());
        node
    })
}
