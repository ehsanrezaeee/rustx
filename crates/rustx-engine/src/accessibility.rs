//! Native semantic projection through AccessKit (Windows UI Automation and
//! macOS NSAccessibility). Actual assistive-technology tasks remain platform
//! acceptance gates; portable tree tests cannot certify a screen reader.
use crate::{layout::BoxNode, Engine};
use accesskit::{
    Action, ActionData, ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler,
    Invalid, Live, Node, NodeId, Rect, Role, TextDirection, TextPosition, TextSelection, Toggled,
    TreeId, TreeInfo, TreeUpdate,
};
use rustx_core::{Kind, SemanticNode};
use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc, Arc, Mutex},
};
use unicode_segmentation::UnicodeSegmentation;
use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::Window,
};

const ROOT: NodeId = NodeId(0);
const MAX_QUEUED_ACTIONS: usize = 128;
const TEXT_NODE_FLAG: u64 = 1 << 63;
fn text_id(owner: u64, line: usize) -> NodeId {
    NodeId(TEXT_NODE_FLAG | (owner << 10) | line as u64)
}
fn text_owner(id: NodeId) -> Option<u64> {
    (id.0 & TEXT_NODE_FLAG != 0).then_some((id.0 & !TEXT_NODE_FLAG) >> 10)
}
fn text_lines(value: &str) -> Vec<&str> {
    let mut lines: Vec<_> = value.split_inclusive('\n').collect();
    if value.is_empty() || value.ends_with('\n') {
        lines.push("");
    }
    lines
}

fn role(node: &SemanticNode) -> Role {
    if node.control.password() {
        return Role::PasswordInput;
    }
    match node.role {
        "button" => Role::Button,
        "link" => Role::Link,
        "textbox" if node.control.multiline => Role::MultilineTextInput,
        "textbox" => Role::TextInput,
        "checkbox" => Role::CheckBox,
        "radio" => Role::RadioButton,
        "radiogroup" => Role::RadioGroup,
        "switch" => Role::Switch,
        "slider" => Role::Slider,
        "combobox" if node.control.options.is_empty() => Role::EditableComboBox,
        "combobox" => Role::ComboBox,
        "option" => Role::ListBoxOption,
        "listbox" => Role::ListBox,
        "list" => Role::List,
        "listitem" => Role::ListItem,
        "heading" => Role::Heading,
        "paragraph" => Role::Paragraph,
        "img" => Role::Image,
        "text" | "label" => Role::Label,
        "table" => Role::Table,
        "row" => Role::Row,
        "cell" => Role::Cell,
        "columnheader" => Role::ColumnHeader,
        "rowheader" => Role::RowHeader,
        "dialog" => Role::Dialog,
        "alertdialog" => Role::AlertDialog,
        "alert" => Role::Alert,
        "status" => Role::Status,
        "tooltip" => Role::Tooltip,
        "progressbar" => Role::ProgressIndicator,
        "tablist" => Role::TabList,
        "tab" => Role::Tab,
        "tabpanel" => Role::TabPanel,
        "menu" => Role::Menu,
        "menuitem" => Role::MenuItem,
        "navigation" => Role::Navigation,
        "form" => Role::Form,
        "separator" => Role::Splitter,
        "main" => Role::Main,
        "banner" => Role::Banner,
        "region" => Role::Region,
        "group" => Role::Group,
        _ => Role::GenericContainer,
    }
}

fn bounds(node: &BoxNode, scale: f64, out: &mut HashMap<u64, Rect>) {
    out.insert(
        node.node.id,
        Rect {
            x0: node.rect.x0 * scale,
            y0: node.rect.y0 * scale,
            x1: node.rect.x1 * scale,
            y1: node.rect.y1 * scale,
        },
    );
    for child in &node.children {
        bounds(child, scale, out);
    }
}

/// Convert one committed semantic tree and Rust layout into an atomic AccessKit
/// update. All geometry is in physical pixels relative to the native client area.
pub fn project(engine: &Engine, scale: f64) -> TreeUpdate {
    let semantic = engine.runtime.semantics();
    let mut rects = HashMap::new();
    if let Some(boxes) = &engine.boxes {
        bounds(boxes, scale, &mut rects);
    }
    let mut ids = HashMap::new();
    fn index(node: &SemanticNode, ids: &mut HashMap<String, NodeId>) {
        if node.control.hidden {
            return;
        }
        if let Some(id) = &node.app_id {
            ids.insert(id.clone(), NodeId(node.id));
        }
        for child in &node.children {
            index(child, ids);
        }
    }
    index(&semantic, &mut ids);
    let mut nodes = vec![];
    fn walk(
        node: &SemanticNode,
        engine: &Engine,
        rects: &HashMap<u64, Rect>,
        ids: &HashMap<String, NodeId>,
        nodes: &mut Vec<(NodeId, Node)>,
    ) {
        if node.control.hidden {
            return;
        }
        let mut accessible = Node::new(role(node));
        if !node.label.is_empty() {
            accessible.set_label(node.label.clone());
        }
        // Password text is never published to platform diagnostics or AT.
        if !node.control.password() && !node.value.is_empty() {
            accessible.set_value(node.value.clone());
        }
        if let Some(rect) = rects.get(&node.id) {
            accessible.set_bounds(*rect);
        }
        if node.control.disabled {
            accessible.set_disabled();
        }
        if node.control.read_only {
            accessible.set_read_only();
        }
        if node.control.required {
            accessible.set_required();
        }
        if node.control.invalid {
            accessible.set_invalid(Invalid::True);
        }
        if node.control.modal {
            accessible.set_modal();
        }
        if let Some(selected) = node.control.selected {
            accessible.set_selected(selected);
        }
        if let Some(checked) = node.control.checked {
            accessible.set_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            });
        }
        if let Some(expanded) = node.control.expanded {
            accessible.set_expanded(expanded);
        }
        if let Some(level) = node.control.level {
            accessible.set_level(level.saturating_sub(1) as usize);
        }
        if let Some(language) = &node.control.lang {
            accessible.set_language(language.clone());
        }
        if let Some(direction) = &node.control.direction {
            accessible.set_text_direction(if direction == "rtl" {
                TextDirection::RightToLeft
            } else {
                TextDirection::LeftToRight
            });
        }
        if let Some(live) = &node.control.live {
            accessible.set_live(if live == "assertive" {
                Live::Assertive
            } else if live == "polite" {
                Live::Polite
            } else {
                Live::Off
            });
        }
        if let Some(min) = node.control.min {
            accessible.set_min_numeric_value(min);
        }
        if let Some(max) = node.control.max {
            accessible.set_max_numeric_value(max);
        }
        if let Some(step) = node.control.step {
            accessible.set_numeric_value_step(step);
        }
        if let Some(value) = node.control.value_now {
            accessible.set_numeric_value(value);
        }
        if let Some(url) = &node.control.href {
            accessible.set_url(url.clone());
        }
        if let Some(app_id) = &node.app_id {
            accessible.set_html_id(app_id.clone());
        }
        for (relation, value) in [
            (0, &node.control.labelled_by),
            (1, &node.control.described_by),
            (2, &node.control.controls),
        ] {
            if let Some(value) = value {
                for id in value.split_whitespace().filter_map(|id| ids.get(id)) {
                    match relation {
                        0 => accessible.push_labelled_by(*id),
                        1 => accessible.push_described_by(*id),
                        _ => accessible.push_controlled(*id),
                    }
                }
            }
        }
        if let Some((position, _)) = node.position {
            accessible.set_position_in_set(position.saturating_sub(1));
        }
        if let Some(total) = node
            .children
            .iter()
            .filter_map(|c| c.position.map(|(_, total)| total))
            .max()
        {
            accessible.set_size_of_set(total);
        }
        if !node.control.disabled {
            if node.focusable {
                accessible.add_action(Action::Focus);
                accessible.add_action(Action::ScrollIntoView);
            }
            if matches!(
                node.role,
                "button" | "link" | "checkbox" | "switch" | "radio" | "tab" | "menuitem" | "option"
            ) {
                accessible.add_action(Action::Click);
            }
            if matches!(node.role, "textbox" | "combobox" | "slider") && !node.control.read_only {
                accessible.add_action(Action::SetValue);
            }
            if node.role == "slider" {
                accessible.add_action(Action::Increment);
                accessible.add_action(Action::Decrement);
            }
            if node.control.expanded.is_some() {
                accessible.add_action(Action::Expand);
                accessible.add_action(Action::Collapse);
            }
        }
        let mut children = node
            .children
            .iter()
            .filter(|c| !c.control.hidden)
            .map(|c| NodeId(c.id))
            .collect::<Vec<_>>();
        if let Some(editor) = engine
            .runtime
            .editor(node.id)
            .filter(|editor| !editor.is_password())
        {
            let value = editor.value();
            let lines = text_lines(&value);
            // Stable synthetic IDs reserve ten bits for line identities. Very
            // long documents keep ValuePattern without claiming TextPattern.
            if lines.len() <= 1024
                && lines
                    .iter()
                    .all(|line| line.graphemes(true).all(|g| g.len() <= u8::MAX as usize))
            {
                let mut offset = 0;
                let selection = editor.selection();
                let mut anchor = None;
                let mut focus = None;
                for (index, line) in lines.iter().enumerate() {
                    let run_id = text_id(node.id, index);
                    let mut run = Node::new(Role::TextRun);
                    run.set_value(line.to_string());
                    run.set_character_lengths(
                        line.graphemes(true)
                            .map(|g| g.len() as u8)
                            .collect::<Vec<_>>(),
                    );
                    let position = |byte: usize| TextPosition {
                        node: run_id,
                        character_index: line[..byte - offset].graphemes(true).count(),
                    };
                    let within = |byte: usize| {
                        byte >= offset
                            && (byte < offset + line.len()
                                || index + 1 == lines.len() && byte == offset + line.len())
                    };
                    if anchor.is_none() && within(selection.anchor) {
                        anchor = Some(position(selection.anchor));
                    }
                    if focus.is_none() && within(selection.focus) {
                        focus = Some(position(selection.focus));
                    }
                    offset += line.len();
                    children.push(run_id);
                    nodes.push((run_id, run));
                }
                if let (Some(anchor), Some(focus)) = (anchor, focus) {
                    accessible.set_text_selection(TextSelection { anchor, focus });
                }
                accessible.add_action(Action::SetTextSelection);
                if !node.control.read_only && !node.control.disabled {
                    accessible.add_action(Action::ReplaceSelectedText);
                }
            }
        }
        accessible.set_children(children);
        nodes.push((NodeId(node.id), accessible));
        for child in &node.children {
            walk(child, engine, rects, ids, nodes);
        }
    }
    walk(&semantic, engine, &rects, &ids, &mut nodes);
    let mut window = Node::new(Role::Window);
    window.set_label("RustX");
    if !semantic.control.hidden {
        window.set_children(vec![NodeId(semantic.id)]);
    }
    nodes.push((ROOT, window));
    let valid: HashSet<_> = nodes.iter().map(|(id, _)| *id).collect();
    let focus = engine
        .focused_node()
        .map(NodeId)
        .filter(|id| valid.contains(id))
        .unwrap_or(ROOT);
    let mut tree = TreeInfo::new(ROOT);
    tree.toolkit_name = Some("RustX".into());
    tree.toolkit_version = Some(env!("CARGO_PKG_VERSION").into());
    TreeUpdate {
        nodes,
        tree: Some(tree),
        tree_id: TreeId::ROOT,
        focus,
    }
}

/// Revalidate every platform request against the current committed tree. Stale,
/// hidden, disabled and read-only nodes cannot be activated by a queued action.
pub fn dispatch_action(engine: &mut Engine, request: ActionRequest) -> bool {
    if request.target_tree != TreeId::ROOT {
        return false;
    }
    let snapshot = engine.runtime.snapshot();
    fn find(n: &rustx_core::Snapshot, id: u64) -> Option<&rustx_core::Snapshot> {
        if n.control.hidden || n.control.disabled {
            return None;
        }
        if n.id == id {
            Some(n)
        } else {
            n.children.iter().find_map(|c| find(c, id))
        }
    }
    let id = request.target_node.0;
    let Some(node) = find(&snapshot, id) else {
        return false;
    };
    fn modal(node: &rustx_core::Snapshot) -> Option<&rustx_core::Snapshot> {
        if node.control.hidden {
            return None;
        }
        node.children
            .iter()
            .rev()
            .find_map(modal)
            .or_else(|| node.control.modal.then_some(node))
    }
    if modal(&snapshot).is_some_and(|modal| find(modal, id).is_none()) {
        return false;
    }
    match request.action {
        Action::Focus | Action::ScrollIntoView if node.focusable() => {
            engine.focus_node(id);
            engine.focused_node() == Some(id)
        }
        Action::Click
            if matches!(node.kind, Kind::Button)
                || matches!(node.role, Some("radio" | "tab" | "menuitem" | "option")) =>
        {
            engine.runtime.click(id, 0., 0.)
        }
        Action::SetValue
            if !node.control.read_only
                && (node.kind == Kind::Input || node.role == Some("slider")) =>
        {
            let value = match request.data {
                Some(ActionData::Value(value)) => value.to_string(),
                Some(ActionData::NumericValue(value)) if value.is_finite() => value.to_string(),
                _ => return false,
            };
            engine.runtime.input(id, value, false)
        }
        Action::ReplaceSelectedText if node.kind == Kind::Input && !node.control.read_only => {
            let (Some(editor), Some(ActionData::Value(value))) =
                (engine.runtime.editor(id), request.data)
            else {
                return false;
            };
            if editor.insert(&value).is_err() {
                return false;
            }
            engine.runtime.invalidate_model();
            engine.runtime.input(id, editor.value(), false)
        }
        Action::SetTextSelection if node.kind == Kind::Input || node.control.selectable => {
            let (Some(editor), Some(ActionData::SetTextSelection(selection))) =
                (engine.runtime.editor(id), request.data)
            else {
                return false;
            };
            if editor.is_password() {
                return false;
            }
            let value = editor.value();
            let lines = text_lines(&value);
            let byte = |position: TextPosition| -> Option<usize> {
                if text_owner(position.node) != Some(id) {
                    return None;
                }
                let line_index = (position.node.0 & 1023) as usize;
                let line = lines.get(line_index)?;
                let local = if position.character_index == line.graphemes(true).count() {
                    line.len()
                } else {
                    line.grapheme_indices(true).nth(position.character_index)?.0
                };
                Some(
                    lines[..line_index]
                        .iter()
                        .map(|line| line.len())
                        .sum::<usize>()
                        + local,
                )
            };
            let (Some(anchor), Some(focus)) = (byte(selection.anchor), byte(selection.focus))
            else {
                return false;
            };
            if editor.select(anchor, focus).is_err() {
                return false;
            }
            engine.runtime.invalidate_model();
            true
        }
        Action::Increment | Action::Decrement
            if node.role == Some("slider") && !node.control.read_only =>
        {
            let step = node.control.step.unwrap_or(1.);
            let current = node.text.parse::<f64>().unwrap_or(0.);
            let value = (current
                + if request.action == Action::Increment {
                    step
                } else {
                    -step
                })
            .clamp(
                node.control.min.unwrap_or(0.),
                node.control.max.unwrap_or(100.),
            );
            engine.runtime.input(id, value.to_string(), false)
        }
        Action::Expand | Action::Collapse if node.control.expanded.is_some() => {
            let expand = request.action == Action::Expand;
            if node.control.expanded == Some(expand) {
                true
            } else {
                engine.runtime.click(id, 0., 0.)
            }
        }
        _ => false,
    }
}

struct Activation {
    latest: Arc<Mutex<Option<TreeUpdate>>>,
    proxy: EventLoopProxy<()>,
}
impl ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        let _ = self.proxy.send_event(());
        self.latest.lock().ok().and_then(|tree| tree.clone())
    }
}
struct Actions {
    sender: mpsc::SyncSender<ActionRequest>,
    proxy: EventLoopProxy<()>,
}
impl ActionHandler for Actions {
    fn do_action(&mut self, request: ActionRequest) {
        // The platform callback may run off-thread. Never block UI Automation
        // on the Rust UI owner and never accumulate an unbounded action queue.
        if self.sender.try_send(request).is_ok() {
            let _ = self.proxy.send_event(());
        }
    }
}
struct Deactivation;
impl DeactivationHandler for Deactivation {
    fn deactivate_accessibility(&mut self) {}
}

pub struct NativeAccessibility {
    adapter: accesskit_winit::Adapter,
    latest: Arc<Mutex<Option<TreeUpdate>>>,
    receiver: mpsc::Receiver<ActionRequest>,
    scale: f64,
}
impl NativeAccessibility {
    /// The window must still be invisible, as required by AccessKit/winit.
    pub fn new(events: &ActiveEventLoop, window: &Window, proxy: EventLoopProxy<()>) -> Self {
        let latest = Arc::new(Mutex::new(None));
        let (sender, receiver) = mpsc::sync_channel(MAX_QUEUED_ACTIONS);
        let adapter = accesskit_winit::Adapter::with_direct_handlers(
            events,
            window,
            Activation {
                latest: latest.clone(),
                proxy: proxy.clone(),
            },
            Actions { sender, proxy },
            Deactivation,
        );
        Self {
            adapter,
            latest,
            receiver,
            scale: window.scale_factor(),
        }
    }
    pub fn process_event(&mut self, window: &Window, event: &WindowEvent) {
        self.scale = window.scale_factor();
        self.adapter.process_event(window, event);
    }
    pub fn update(&mut self, engine: &Engine) {
        // Host layout is in logical pixels; AccessKit native coordinates follow
        // the platform's client-space coordinates (physical pixels on Windows).
        self.update_at_scale(engine, self.scale);
    }
    pub fn update_at_scale(&mut self, engine: &Engine, scale: f64) {
        let update = project(engine, scale);
        if let Ok(mut latest) = self.latest.lock() {
            *latest = Some(update.clone());
        }
        self.adapter.update_if_active(|| update);
    }
    pub fn dispatch_actions(&mut self, engine: &mut Engine) -> bool {
        let mut changed = false;
        for request in self.receiver.try_iter() {
            changed |= dispatch_action(engine, request);
        }
        if changed {
            engine.invalidate();
        }
        changed
    }
}
