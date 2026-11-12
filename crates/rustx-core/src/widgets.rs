//! The finite portable widget vocabulary. Geometry and text stay in Rust.
use crate::{dynamic, Kind, Node, Scope, Signal, Style, View};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub disabled: bool,
}
impl Choice {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlState {
    /// Committed Rust scroll position used for normalized Scroll notifications.
    pub scroll_offset: Option<f64>,
    pub disabled: bool,
    pub read_only: bool,
    pub checked: Option<bool>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    pub modal: bool,
    pub hidden: bool,
    pub stack: bool,
    pub overlay: bool,
    pub anchor: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub options: Vec<Choice>,
    pub described_by: Option<String>,
    pub labelled_by: Option<String>,
    pub controls: Option<String>,
    pub tab_index: Option<i32>,
    pub href: Option<String>,
    pub input_type: Option<String>,
    pub lang: Option<String>,
    pub direction: Option<String>,
    pub multiline: bool,
    pub required: bool,
    pub invalid: bool,
    pub live: Option<String>,
    pub level: Option<u8>,
    pub sort: Option<String>,
    pub editable: bool,
    /// Ordinary Rust text using a read-only platform selection/copy bridge.
    pub selectable: bool,
    pub value_text: Option<String>,
    pub value_now: Option<f64>,
}
impl ControlState {
    pub fn password(&self) -> bool {
        self.input_type.as_deref() == Some("password")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widget {
    Box,
    Row,
    Column,
    Grid,
    Stack,
    Spacer,
    Divider,
    ScrollView,
    Text,
    Heading,
    Paragraph,
    Image,
    Icon,
    Button,
    IconButton,
    Link,
    Tabs,
    Menu,
    MenuItem,
    Breadcrumbs,
    Form,
    Label,
    TextInput,
    PasswordInput,
    TextArea,
    Checkbox,
    RadioGroup,
    Switch,
    Select,
    Combobox,
    Slider,
    Dialog,
    Popover,
    Tooltip,
    Alert,
    Toast,
    Progress,
    Spinner,
    List,
    VirtualList,
    Table,
    TableRow,
    TableCell,
    Canvas,
    DrawingSurface,
}
impl Node {
    pub fn widget(widget: Widget) -> Self {
        use Widget::*;
        let kind = match widget {
            Text | Heading | Paragraph | Icon | Label | Divider | Spacer | Alert | Toast
            | Tooltip | Progress | Spinner => Kind::Text,
            Button | IconButton | Link | MenuItem | Checkbox | Switch | Slider => Kind::Button,
            TextInput | PasswordInput | TextArea | Combobox | Select => Kind::Input,
            Image => Kind::Image,
            Canvas | DrawingSurface => Kind::Canvas,
            _ => Kind::Div,
        };
        let role = match widget {
            Heading => "heading",
            Paragraph => "paragraph",
            Image | Icon => "img",
            Button | IconButton => "button",
            Link => "link",
            Tabs => "tablist",
            Menu => "menu",
            MenuItem => "menuitem",
            Breadcrumbs => "navigation",
            Form => "form",
            Label => "label",
            TextInput | PasswordInput | TextArea => "textbox",
            Checkbox => "checkbox",
            RadioGroup => "radiogroup",
            Switch => "switch",
            Select | Combobox => "combobox",
            Slider => "slider",
            Dialog => "dialog",
            Popover => "group",
            Tooltip => "tooltip",
            Alert => "alert",
            Toast => "status",
            Progress | Spinner => "progressbar",
            List | VirtualList => "list",
            Table => "table",
            TableRow => "row",
            TableCell => "cell",
            Divider => "separator",
            Canvas | DrawingSurface => "img",
            Text => "text",
            _ => "group",
        };
        let mut n = Node::new(kind).role(role);
        match widget {
            Row | Tabs | Menu | Breadcrumbs | TableRow => n.style = Style::row(),
            Grid => n.style = Style::grid(2),
            Stack => {
                n.style = Style::grid(1).gap(0.);
                n.control.stack = true;
            }
            ScrollView | VirtualList => n.style = Style::column().height(320.).clip(),
            Spacer => n.style = Style::column().grow(1.),
            Divider => {
                n.style = Style::column()
                    .height(1.)
                    .background_token(crate::ColorToken::Border)
            }
            Button | IconButton | Link | MenuItem | Checkbox | Switch | Slider => {
                n.style = Style::column()
                    .padding(10.)
                    .radius(6.)
                    .background_token(crate::ColorToken::Surface)
                    .color_token(crate::ColorToken::Text)
            }
            TextInput | PasswordInput | TextArea | Select | Combobox => {
                n.style = Style::column()
                    .padding(10.)
                    .height(if widget == TextArea { 100. } else { 42. })
                    .background_token(crate::ColorToken::Surface)
                    .border(1., 0x52647b)
                    .radius(6.)
            }
            Dialog | Popover => {
                n.style = Style::column()
                    .padding(24.)
                    .background_token(crate::ColorToken::Surface)
                    .border(2., 0x95a4bd)
                    .radius(12.)
            }
            _ => {}
        }
        match widget {
            TextInput => n.control.input_type = Some("text".into()),
            PasswordInput => n.control.input_type = Some("password".into()),
            TextArea => {
                n.control.input_type = Some("text".into());
                n.control.multiline = true;
            }
            Checkbox | Switch => n.control.checked = Some(false),
            Select | Combobox => {
                n.control.expanded = Some(false);
                n.control.input_type = Some("text".into());
            }
            Slider => {
                n.control.min = Some(0.);
                n.control.max = Some(100.);
                n.control.step = Some(1.);
                n.text = "0".into();
            }
            Dialog => {
                n.control.modal = true;
                n.control.overlay = true;
            }
            Popover | Tooltip => n.control.overlay = true,
            Heading => {
                n.control.level = Some(2);
                n.style.font_size = 24.;
            }
            Alert => n.control.live = Some("assertive".into()),
            Toast => n.control.live = Some("polite".into()),
            _ => {}
        }
        n
    }
    pub fn control(mut self, state: ControlState) -> Self {
        self.control = state;
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.control.disabled = value;
        if value {
            self.style.opacity = 0.5;
        }
        self
    }
    pub fn read_only(mut self, value: bool) -> Self {
        self.control.read_only = value;
        self
    }
    /// Keep normal Rust text layout/painting while allowing selection and copy.
    /// The platform projection is one read-only multiline text control.
    pub fn selectable(mut self, value: bool) -> Self {
        assert_eq!(
            self.kind,
            Kind::Text,
            "selectable requires Text or Paragraph"
        );
        self.control.selectable = value;
        self.control.editable = value;
        self.control.read_only = value;
        self.control.multiline = value;
        self
    }
    pub fn checked(mut self, value: bool) -> Self {
        self.control.checked = Some(value);
        self
    }
    pub fn selected(mut self, value: bool) -> Self {
        self.control.selected = Some(value);
        self
    }
    pub fn expanded(mut self, value: bool) -> Self {
        self.control.expanded = Some(value);
        self
    }
    pub fn modal(mut self, value: bool) -> Self {
        self.control.modal = value;
        self
    }
    pub fn overlay(mut self, value: bool) -> Self {
        self.control.overlay = value;
        self
    }
    pub fn anchor(mut self, value: impl ToString) -> Self {
        self.control.anchor = Some(value.to_string());
        self
    }
    pub fn hidden(mut self, value: bool) -> Self {
        self.control.hidden = value;
        self
    }
    pub fn min(mut self, value: f64) -> Self {
        self.control.min = value.is_finite().then_some(value);
        self
    }
    pub fn max(mut self, value: f64) -> Self {
        self.control.max = value.is_finite().then_some(value);
        self
    }
    pub fn step(mut self, value: f64) -> Self {
        self.control.step = (value.is_finite() && value > 0.).then_some(value);
        self
    }
    pub fn options(mut self, value: Vec<Choice>) -> Self {
        assert!(value.len() <= 4096, "option limit exceeded");
        self.control.options = value;
        self
    }
    pub fn described_by(mut self, value: impl ToString) -> Self {
        self.control.described_by = Some(value.to_string());
        self
    }
    pub fn labelled_by(mut self, value: impl ToString) -> Self {
        self.control.labelled_by = Some(value.to_string());
        self
    }
    pub fn controls(mut self, value: impl ToString) -> Self {
        self.control.controls = Some(value.to_string());
        self
    }
    pub fn tab_index(mut self, value: i32) -> Self {
        self.control.tab_index = Some(value);
        self
    }
    pub fn href(mut self, value: impl ToString) -> Self {
        self.control.href = Some(value.to_string());
        self
    }
    pub fn input_type(mut self, value: impl ToString) -> Self {
        self.control.input_type = Some(value.to_string());
        self
    }
    pub fn lang(mut self, value: impl ToString) -> Self {
        self.control.lang = Some(value.to_string());
        self
    }
    pub fn required(mut self, value: bool) -> Self {
        self.control.required = value;
        self
    }
    pub fn invalid(mut self, value: bool) -> Self {
        self.control.invalid = value;
        self
    }
    pub fn live(mut self, value: impl ToString) -> Self {
        self.control.live = Some(value.to_string());
        self
    }
    pub fn level(mut self, value: u8) -> Self {
        self.control.level = Some(value.clamp(1, 6));
        self
    }
    pub fn sort(mut self, value: impl ToString) -> Self {
        self.control.sort = Some(value.to_string());
        self
    }
    pub fn value_text(mut self, value: impl ToString) -> Self {
        self.control.value_text = Some(value.to_string());
        self
    }
    pub fn value_now(mut self, value: f64) -> Self {
        self.control.value_now = value.is_finite().then_some(value);
        self
    }
}

/// Controlled tab panels keep the component mounted while switching their view.
#[derive(Clone)]
pub struct TabPanel {
    pub id: String,
    pub label: String,
    pub content: View,
    pub disabled: bool,
}
pub fn tabs(active: Signal<usize>, panels: Vec<TabPanel>) -> View {
    assert!(!panels.is_empty() && panels.len() <= 256);
    dynamic(move || {
        let index = active.get().min(panels.len() - 1);
        let buttons = panels
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let selected = active.clone();
                Node::widget(Widget::Button)
                    .role("tab")
                    .id(format!("{}-tab", p.id))
                    .controls(&p.id)
                    .label(&p.label)
                    .value(&p.label)
                    .selected(i == index)
                    .disabled(p.disabled)
                    .tab_index(if i == index { 0 } else { -1 })
                    .on_click(move |_| {
                        let _ = selected.set(i);
                    })
            })
            .collect();
        Node::widget(Widget::Column).children(vec![
            Node::widget(Widget::Tabs).children(buttons),
            panels[index]
                .content
                .clone()
                .id(&panels[index].id)
                .role("tabpanel")
                .labelled_by(format!("{}-tab", panels[index].id)),
        ])
    })
}

pub fn radio_group(value: Signal<String>, label: impl Into<String>, choices: Vec<Choice>) -> View {
    let label = label.into();
    dynamic(move || {
        let current = value.get();
        let tab_value = choices
            .iter()
            .find(|c| !c.disabled && c.value == current)
            .or_else(|| choices.iter().find(|c| !c.disabled))
            .map(|c| c.value.as_str());
        Node::widget(Widget::RadioGroup).label(&label).children(
            choices
                .iter()
                .map(|choice| {
                    let value = value.clone();
                    let selected = choice.value.clone();
                    Node::widget(Widget::Button)
                        .role("radio")
                        .checked(current == choice.value)
                        .value(&choice.label)
                        .label(&choice.label)
                        .disabled(choice.disabled)
                        .tab_index(if tab_value == Some(choice.value.as_str()) {
                            0
                        } else {
                            -1
                        })
                        .on_click(move |_| {
                            let _ = value.set(selected.clone());
                        })
                })
                .collect(),
        )
    })
}

#[derive(Clone)]
pub struct OverlayState {
    pub open: Signal<bool>,
}
impl OverlayState {
    pub fn new(cx: &Scope) -> Self {
        Self {
            open: cx.signal(false),
        }
    }
    pub fn show(&self) {
        let _ = self.open.set(true);
    }
    pub fn close(&self) {
        let _ = self.open.set(false);
    }
}
pub fn dialog(state: OverlayState, label: impl Into<String>, content: View) -> View {
    overlay(state, label.into(), content, true)
}
pub fn popover(state: OverlayState, label: impl Into<String>, content: View) -> View {
    overlay(state, label.into(), content, false)
}
fn overlay(state: OverlayState, label: String, content: View, modal: bool) -> View {
    dynamic(move || {
        if !state.open.get() {
            return Node::fragment(vec![]);
        }
        let escape = state.clone();
        let close = state.clone();
        Node::widget(if modal {
            Widget::Dialog
        } else {
            Widget::Popover
        })
        .label(&label)
        .modal(modal)
        .on_key_down(move |e| {
            if e.data.key == "Escape" {
                escape.close();
                e.prevent_default();
            }
        })
        .children(vec![
            content.clone(),
            Node::widget(Widget::Button)
                .value("Close")
                .label("Close")
                .on_click(move |_| close.close()),
        ])
    })
}

#[derive(Clone, Debug)]
pub struct TableColumn {
    pub label: String,
}
#[derive(Clone, Debug)]
pub struct TableRecord {
    pub key: String,
    pub cells: Vec<String>,
}
#[derive(Clone)]
pub struct TableState {
    pub rows: Signal<Vec<TableRecord>>,
    pub selected: Signal<Option<String>>,
    pub sort_column: Signal<Option<usize>>,
    pub descending: Signal<bool>,
}
impl TableState {
    pub fn new(cx: &Scope, rows: Vec<TableRecord>) -> Self {
        assert!(rows.len() <= 10_000);
        Self {
            rows: cx.signal(rows),
            selected: cx.signal(None),
            sort_column: cx.signal(None),
            descending: cx.signal(false),
        }
    }
    pub fn sort(&self, column: usize) {
        if self.sort_column.get() == Some(column) {
            let _ = self.descending.update(|v| *v = !*v);
        } else {
            let _ = self.sort_column.set(Some(column));
            let _ = self.descending.set(false);
        }
    }
}
pub fn data_table(state: TableState, columns: Vec<TableColumn>, label: impl Into<String>) -> View {
    assert!(!columns.is_empty() && columns.len() <= 64);
    let label = label.into();
    dynamic(move || {
        let selected = state.selected.get();
        let sort = state.sort_column.get();
        let descending = state.descending.get();
        let mut rows = state.rows.get();
        if let Some(column) = sort {
            rows.sort_by(|a, b| {
                let order = a
                    .cells
                    .get(column)
                    .cmp(&b.cells.get(column))
                    .then(a.key.cmp(&b.key));
                if descending {
                    order.reverse()
                } else {
                    order
                }
            });
        }
        let headers = columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let state = state.clone();
                Node::widget(Widget::Button)
                    .role("columnheader")
                    .value(&column.label)
                    .sort(if sort == Some(index) {
                        if descending {
                            "descending"
                        } else {
                            "ascending"
                        }
                    } else {
                        "none"
                    })
                    .on_click(move |_| state.sort(index))
            })
            .collect();
        let mut children = vec![Node::widget(Widget::TableRow).children(headers)];
        for row in rows {
            let select = state.selected.clone();
            let key = row.key.clone();
            let keyboard = state.selected.clone();
            let keyboard_key = key.clone();
            let cells = row
                .cells
                .iter()
                .map(|cell| Node::widget(Widget::TableCell).children(vec![Node::text(cell)]))
                .collect();
            children.push(
                Node::widget(Widget::TableRow)
                    .key(&row.key)
                    .selected(selected.as_ref() == Some(&row.key))
                    .tab_index(0)
                    .on_click(move |_| {
                        let _ = select.set(Some(key.clone()));
                    })
                    .on_key_down(move |e| {
                        if e.data.key == " " || e.data.key == "Enter" {
                            let _ = keyboard.set(Some(keyboard_key.clone()));
                            e.prevent_default();
                        }
                    })
                    .children(cells),
            );
        }
        Node::widget(Widget::Table).label(&label).children(children)
    })
}

pub fn progress(value: Option<f64>, label: impl Into<String>) -> View {
    let mut node = Node::widget(Widget::Progress)
        .label(label.into())
        .min(0.)
        .max(100.);
    node.text = value
        .filter(|v| v.is_finite())
        .map(|v| format!("{:.0}%", v.clamp(0., 100.)))
        .unwrap_or_else(|| "Working…".into());
    node
}
pub fn spinner(label: impl Into<String>) -> View {
    let label = label.into();
    Node::component("RustX.Spinner", move |cx| {
        let frame = cx.signal(0usize);
        if let (Some(services), Some(tasks)) = (
            cx.context::<crate::ServiceClient>(),
            cx.context::<crate::TaskQueue>(),
        ) {
            let out = frame.clone();
            let owner = cx.clone();
            let _ = tasks.spawn(&cx, async move {
                loop {
                    if services
                        .sleep(&owner, std::time::Duration::from_millis(150))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    if out.update(|v| *v = (*v + 1) % 4).is_err() {
                        break;
                    }
                }
            });
        }
        let label = label.clone();
        dynamic(move || {
            Node::widget(Widget::Spinner)
                .label(&label)
                .value(["|", "/", "—", "\\"][frame.get()])
        })
    })
}

/// Bounded nested scrolling with a declared logical content extent. The inner
/// scroll owner consumes a wheel only while it can move; boundaries chain out.
#[derive(Clone)]
pub struct ScrollState {
    pub offset: Signal<f64>,
    pub viewport: f64,
    pub extent: f64,
}
impl ScrollState {
    pub fn new(cx: &Scope, viewport: f64, extent: f64) -> Self {
        assert!(viewport.is_finite() && viewport > 0. && extent.is_finite() && extent >= 0.);
        Self {
            offset: cx.signal(0.),
            viewport,
            extent,
        }
    }
    pub fn scroll_by(&self, delta: f64) -> bool {
        if !delta.is_finite() {
            return false;
        }
        let old = self.offset.get();
        let next = (old + delta).clamp(0., (self.extent - self.viewport).max(0.));
        if next != old {
            let _ = self.offset.set(next);
            true
        } else {
            false
        }
    }
    pub fn scroll_to(&self, offset: f64) {
        if offset.is_finite() {
            let _ = self
                .offset
                .set(offset.clamp(0., (self.extent - self.viewport).max(0.)));
        }
    }
}
pub fn scroll_view(state: ScrollState, content: View) -> View {
    dynamic(move || {
        let offset = state.offset.get();
        let offset = if offset.is_finite() {
            offset.clamp(0., (state.extent - state.viewport).max(0.))
        } else {
            0.
        };
        let wheel = state.clone();
        let keyboard = state.clone();
        let mut node = Node::widget(Widget::ScrollView)
            .role("region")
            .tab_index(0)
            .style(Style::column().height(state.viewport as f32).clip())
            .on_wheel(move |e| {
                if !e.default_prevented() && wheel.scroll_by(e.data.delta_y) {
                    e.prevent_default();
                }
            })
            .on_key_down(move |e| {
                if e.default_prevented() {
                    return;
                }
                let delta = match e.data.key.as_str() {
                    "ArrowDown" => 40.,
                    "ArrowUp" => -40.,
                    "PageDown" => keyboard.viewport,
                    "PageUp" => -keyboard.viewport,
                    "Home" => -keyboard.extent,
                    "End" => keyboard.extent,
                    _ => return,
                };
                if keyboard.scroll_by(delta) {
                    e.prevent_default();
                }
            })
            .children(vec![Node::widget(Widget::Column)
                .style(Style::column().translate(0., -offset as f32))
                .children(vec![content.clone()])]);
        node.control.scroll_offset = Some(offset);
        node
    })
}

pub fn select(value: Signal<String>, choices: Vec<Choice>, label: impl Into<String>) -> View {
    let label = label.into();
    let id = format!(
        "select-{}",
        label
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
    );
    select_with_id(id, value, choices, label)
}
/// A Rust retained listbox popup. Selection is controlled; opening/search state
/// belongs to the component scope and is discarded on unmount.
pub fn select_with_id(
    id: impl Into<String>,
    value: Signal<String>,
    choices: Vec<Choice>,
    label: impl Into<String>,
) -> View {
    let id = id.into();
    let label = label.into();
    assert!(choices.len() <= 4096);
    Node::component("RustX.Select", move |cx| {
        let open = cx.signal(false);
        let value = value.clone();
        let choices = choices.clone();
        let label = label.clone();
        let id = id.clone();
        dynamic(move || {
            let selected = value.get();
            let is_open = open.get();
            let display = choices
                .iter()
                .find(|c| c.value == selected)
                .map_or(selected.as_str(), |c| c.label.as_str());
            let toggle = open.clone();
            let key_close = open.clone();
            let blur = open.clone();
            let update = value.clone();
            let mut children = vec![Node::widget(Widget::Button)
                .id(&id)
                .role("combobox")
                .label(&label)
                .value(&selected)
                .value_text(format!("{display} ▾"))
                .options(choices.clone())
                .expanded(is_open)
                .controls(format!("{id}-options"))
                .on_click(move |_| {
                    let _ = toggle.update(|v| *v = !*v);
                })
                .on_input(move |e| {
                    let _ = update.set(e.value);
                })
                .on_blur(move |_| {
                    let _ = blur.set(false);
                })
                .on_key_down(move |e| {
                    if e.data.key == "Escape" {
                        let _ = key_close.set(false);
                        e.prevent_default();
                    }
                })];
            if is_open {
                let options = choices
                    .iter()
                    .map(|choice| {
                        let update = value.clone();
                        let close = open.clone();
                        let choice_value = choice.value.clone();
                        Node::widget(Widget::Button)
                            .role("option")
                            .selected(choice.value == selected)
                            .disabled(choice.disabled)
                            .tab_index(-1)
                            .value(&choice.label)
                            .on_pointer_down(|e| e.prevent_default())
                            .on_click(move |_| {
                                let _ = update.set(choice_value.clone());
                                let _ = close.set(false);
                            })
                    })
                    .collect();
                children.push(
                    Node::widget(Widget::Column)
                        .role("listbox")
                        .id(format!("{id}-options"))
                        .overlay(true)
                        .anchor(&id)
                        .label(&label)
                        .style(
                            Style::column()
                                .padding(4.)
                                .background_token(crate::ColorToken::Surface)
                                .border(1., 0x95a4bd),
                        )
                        .children(options),
                );
            }
            Node::widget(Widget::Column).children(children)
        })
    })
}

pub fn combobox(value: Signal<String>, choices: Vec<Choice>, label: impl Into<String>) -> View {
    let label = label.into();
    let id = format!(
        "combo-{}",
        label
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
    );
    combobox_with_id(id, value, choices, label)
}
pub fn combobox_with_id(
    id: impl Into<String>,
    value: Signal<String>,
    choices: Vec<Choice>,
    label: impl Into<String>,
) -> View {
    let id = id.into();
    let label = label.into();
    assert!(choices.len() <= 4096);
    Node::component("RustX.Combobox", move |cx| {
        let initial = value.get();
        let query = cx.signal(
            choices
                .iter()
                .find(|c| c.value == initial)
                .map_or_else(|| initial.clone(), |c| c.label.clone()),
        );
        let open = cx.signal(false);
        let active = cx.signal(0usize);
        let controlled = value.clone();
        let controlled_query = query.clone();
        let controlled_open = open.clone();
        let controlled_active = active.clone();
        let controlled_choices = choices.clone();
        let previous = std::cell::RefCell::new(initial);
        cx.effect(move || {
            let selected = controlled.get();
            if *previous.borrow() != selected {
                *previous.borrow_mut() = selected.clone();
                let label = controlled_choices
                    .iter()
                    .find(|choice| choice.value == selected)
                    .map_or_else(|| selected.clone(), |choice| choice.label.clone());
                let _ = controlled_query.set(label);
                let _ = controlled_open.set(false);
                let _ = controlled_active.set(0);
            }
            Box::new(|| {})
        });
        let value = value.clone();
        let choices = choices.clone();
        let id = id.clone();
        let label = label.clone();
        dynamic(move || {
            let search = query.get();
            let is_open = open.get();
            let selected = value.get();
            let matches: Vec<_> = choices
                .iter()
                .filter(|c| c.label.to_lowercase().contains(&search.to_lowercase()))
                .cloned()
                .collect();
            let chosen = active.get().min(matches.len().saturating_sub(1));
            let chosen = matches
                .get(chosen)
                .filter(|choice| !choice.disabled)
                .map(|_| chosen)
                .or_else(|| matches.iter().position(|choice| !choice.disabled))
                .unwrap_or(chosen);
            let input = query.clone();
            let show = open.clone();
            let at = active.clone();
            let keys = matches.clone();
            let keyboard_active = active.clone();
            let keyboard_value = value.clone();
            let keyboard_query = query.clone();
            let keyboard_open = open.clone();
            let blur = open.clone();
            let mut control = Node::widget(Widget::TextInput)
                .id(&id)
                .role("combobox")
                .label(&label)
                .value(&search)
                .expanded(is_open)
                .controls(format!("{id}-options"))
                .on_input(move |e| {
                    let _ = input.set(e.value);
                    let _ = show.set(true);
                    let _ = at.set(0);
                })
                .on_blur(move |_| {
                    let _ = blur.set(false);
                })
                .on_key_down(move |e| match e.data.key.as_str() {
                    "ArrowDown" | "ArrowUp" if !keys.is_empty() => {
                        let next = (1..=keys.len())
                            .map(|distance| {
                                if e.data.key == "ArrowDown" {
                                    (chosen + distance) % keys.len()
                                } else {
                                    (chosen + keys.len() - distance) % keys.len()
                                }
                            })
                            .find(|&index| !keys[index].disabled)
                            .unwrap_or(chosen);
                        let _ = keyboard_active.set(next);
                        let _ = keyboard_open.set(true);
                        e.prevent_default();
                    }
                    "Enter" if !keys.is_empty() => {
                        if !keys[chosen].disabled {
                            let _ = keyboard_value.set(keys[chosen].value.clone());
                            let _ = keyboard_query.set(keys[chosen].label.clone());
                            let _ = keyboard_open.set(false);
                        }
                        e.prevent_default();
                    }
                    "Escape" => {
                        let _ = keyboard_open.set(false);
                        e.prevent_default();
                    }
                    _ => {}
                });
            control.control.editable = true;
            let mut children = vec![control];
            if is_open {
                let options = matches
                    .iter()
                    .enumerate()
                    .map(|(index, choice)| {
                        let update = value.clone();
                        let text = query.clone();
                        let close = open.clone();
                        let choice = choice.clone();
                        Node::widget(Widget::Button)
                            .role("option")
                            .id(format!("{id}-option-{index}"))
                            .selected(choice.value == selected || index == chosen)
                            .disabled(choice.disabled)
                            .tab_index(-1)
                            .value(&choice.label)
                            .on_pointer_down(|e| e.prevent_default())
                            .on_click(move |_| {
                                let _ = update.set(choice.value.clone());
                                let _ = text.set(choice.label.clone());
                                let _ = close.set(false);
                            })
                    })
                    .collect();
                children.push(
                    Node::widget(Widget::Column)
                        .role("listbox")
                        .id(format!("{id}-options"))
                        .overlay(true)
                        .anchor(&id)
                        .label(&label)
                        .style(
                            Style::column()
                                .padding(4.)
                                .background_token(crate::ColorToken::Surface),
                        )
                        .children(options),
                );
            }
            Node::widget(Widget::Column).children(children)
        })
    })
}

pub fn tooltip(id: impl Into<String>, trigger: View, text: impl Into<String>) -> View {
    let id = id.into();
    let text = text.into();
    let trigger_id = trigger
        .id
        .clone()
        .unwrap_or_else(|| format!("{id}-trigger"));
    let trigger = trigger.id(&trigger_id);
    Node::component("RustX.Tooltip", move |cx| {
        let shown = cx.signal(false);
        let id = id.clone();
        let text = text.clone();
        let trigger = trigger.clone();
        let trigger_id = trigger_id.clone();
        dynamic(move || {
            let enter = shown.clone();
            let leave = shown.clone();
            let focus = shown.clone();
            let blur = shown.clone();
            let escape = shown.clone();
            let trigger = trigger
                .clone()
                .described_by(&id)
                .on_pointer_enter(move |_| {
                    let _ = enter.set(true);
                })
                .on_pointer_leave(move |_| {
                    let _ = leave.set(false);
                })
                .on_event(crate::EventKind::Focus, move |_| {
                    let _ = focus.set(true);
                })
                .on_blur(move |_| {
                    let _ = blur.set(false);
                })
                .on_key_down(move |e| {
                    if e.data.key == "Escape" {
                        let _ = escape.set(false);
                        e.prevent_default();
                    }
                });
            let mut children = vec![trigger];
            if shown.get() {
                children.push(
                    Node::widget(Widget::Tooltip)
                        .id(&id)
                        .anchor(&trigger_id)
                        .value(&text)
                        .style(
                            Style::column()
                                .padding(8.)
                                .background_token(crate::ColorToken::Surface),
                        ),
                );
            }
            Node::widget(Widget::Column).children(children)
        })
    })
}

#[derive(Clone, Debug)]
pub struct ToastMessage {
    pub id: u64,
    pub text: String,
}
#[derive(Clone)]
pub struct ToastQueue {
    pub messages: Signal<Vec<ToastMessage>>,
    next: std::rc::Rc<std::cell::Cell<u64>>,
}
impl ToastQueue {
    pub fn new(cx: &Scope) -> Self {
        Self {
            messages: cx.signal(vec![]),
            next: std::rc::Rc::new(std::cell::Cell::new(1)),
        }
    }
    pub fn push(&self, text: impl Into<String>) -> u64 {
        let id = self.next.get();
        self.next
            .set(id.checked_add(1).expect("toast id exhausted"));
        let mut messages = self.messages.get();
        if messages.len() == 8 {
            messages.remove(0);
        }
        let text = text.into();
        messages.push(ToastMessage {
            id,
            text: text.chars().take(4096).collect(),
        });
        let _ = self.messages.set(messages);
        id
    }
    pub fn dismiss(&self, id: u64) {
        let _ = self
            .messages
            .update(|messages| messages.retain(|m| m.id != id));
    }
}
pub fn toast_region(queue: ToastQueue) -> View {
    dynamic(move || {
        Node::widget(Widget::Column)
            .role("region")
            .label("Notifications")
            .children(
                queue
                    .messages
                    .get()
                    .into_iter()
                    .map(|message| {
                        let queue = queue.clone();
                        let id = message.id;
                        Node::widget(Widget::Column)
                            .role("status")
                            .live("polite")
                            .key(id)
                            .children(vec![
                                Node::text(message.text),
                                Node::widget(Widget::Button)
                                    .value("Dismiss")
                                    .on_click(move |_| queue.dismiss(id)),
                            ])
                    })
                    .collect(),
            )
    })
}
