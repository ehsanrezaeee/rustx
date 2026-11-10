//! Mount-once components; owned signals and automatically tracked reactive bindings.
mod assets;
pub use assets::*;
pub mod drawing;
mod errors;
pub mod events;
pub mod widgets;
pub use errors::*;
pub mod editing;
pub mod forms;
mod resources;
mod router;
mod services;
pub use editing::*;
pub use events::*;
pub use forms::*;
pub use resources::*;
pub use router::*;
pub use services::*;
pub use widgets::*;
mod host;
mod style;
mod virtual_list;
pub use host::*;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashMap, HashSet},
    rc::{Rc, Weak},
};
pub use style::*;
pub use virtual_list::*;
pub type NodeId = u64;
pub const MAX_HOST_ID: u64 = 9_007_199_254_740_991;
#[derive(Clone, Default)]
pub struct NodeRef(Rc<Cell<Option<NodeId>>>);
impl NodeRef {
    pub fn get(&self) -> Option<NodeId> {
        self.0.get()
    }
}

pub type View = Node;
type Cleanup = Box<dyn FnOnce()>;
type Mount = Rc<RefCell<Mounted>>;
type Factory = Rc<dyn Fn(Scope) -> View>;
#[derive(Clone, Copy, Debug, Default)]
pub struct ClickEvent {
    pub target: u64,
    pub x: f32,
    pub y: f32,
}
#[derive(Clone)]
pub struct InputEvent {
    pub target: u64,
    pub value: String,
    pub composing: bool,
}
impl std::fmt::Debug for InputEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputEvent")
            .field("target", &self.target)
            .field("value", &"[redacted]")
            .field("composing", &self.composing)
            .finish()
    }
}
#[derive(Clone)]
pub struct Node {
    pub control: ControlState,
    pub editing: Option<Editor>,
    pub listeners: Vec<EventListener>,
    pub role: Option<&'static str>,
    pub position: Option<(usize, usize)>,
    pub drawing: Option<Rc<drawing::CanvasScene>>,
    pub camera: drawing::Camera,
    pub on_scroll: Option<Rc<dyn Fn(f64)>>,
    pub on_focus: Option<Rc<dyn Fn()>>,
    pub on_canvas: Option<Rc<dyn Fn(drawing::CanvasEvent)>>,
    pub node_ref: Option<NodeRef>,
    pub kind: Kind,
    pub key: Option<String>,
    pub text: String,
    pub style: Style,
    pub id: Option<String>,
    pub label: Option<String>,
    pub on_click: Option<Rc<dyn Fn(ClickEvent)>>,
    pub on_input: Option<Rc<dyn Fn(InputEvent)>>,
    pub children: Vec<Node>,
    pub component: Option<(&'static str, Factory)>,
    pub binding: Option<Rc<dyn Fn() -> View>>,
}
impl Node {
    pub fn new(kind: Kind) -> Self {
        Self {
            control: ControlState::default(),
            editing: None,
            listeners: vec![],
            role: None,
            position: None,
            drawing: None,
            camera: Default::default(),
            on_scroll: None,
            on_focus: None,
            on_canvas: None,
            node_ref: None,
            kind,
            key: None,
            text: String::new(),
            style: Style::default(),
            id: None,
            label: None,
            on_click: None,
            on_input: None,
            children: vec![],
            component: None,
            binding: None,
        }
    }
    pub fn text(v: impl ToString) -> Self {
        Self {
            text: v.to_string(),
            ..Self::new(Kind::Text)
        }
    }
    pub fn editing(mut self, editor: Editor) -> Self {
        self.control.read_only = editor.snapshot().read_only;
        if editor.snapshot().password {
            self.control.input_type = Some("password".into());
        }
        self.editing = Some(editor);
        self
    }
    pub fn on_event(mut self, kind: EventKind, f: impl Fn(Event) + 'static) -> Self {
        self.listeners.push(EventListener {
            kind,
            capture: false,
            callback: Rc::new(f),
        });
        self
    }
    pub fn on_event_capture(mut self, kind: EventKind, f: impl Fn(Event) + 'static) -> Self {
        self.listeners.push(EventListener {
            kind,
            capture: true,
            callback: Rc::new(f),
        });
        self
    }
    pub fn fragment(children: Vec<View>) -> Self {
        Self {
            children,
            ..Self::new(Kind::Fragment)
        }
    }
    pub fn component(name: &'static str, f: impl Fn(Scope) -> View + 'static) -> Self {
        Self {
            component: Some((name, Rc::new(f))),
            ..Self::new(Kind::Fragment)
        }
    }
    pub fn role(mut self, role: &'static str) -> Self {
        self.role = Some(role);
        self
    }
    pub fn position(mut self, index: usize, total: usize) -> Self {
        self.position = Some((index, total));
        self
    }
    pub fn drawing(mut self, scene: Rc<drawing::CanvasScene>) -> Self {
        self.drawing = Some(scene);
        self
    }
    pub fn camera(mut self, camera: drawing::Camera) -> Self {
        self.camera = camera;
        self
    }
    pub fn on_canvas(mut self, f: impl Fn(drawing::CanvasEvent) + 'static) -> Self {
        self.on_canvas = Some(Rc::new(f));
        self
    }
    pub fn on_focus(mut self, f: impl Fn() + 'static) -> Self {
        self.on_focus = Some(Rc::new(f));
        self
    }
    pub fn on_scroll(mut self, f: impl Fn(f64) + 'static) -> Self {
        self.on_scroll = Some(Rc::new(f));
        self
    }
    pub fn node_ref(mut self, r: NodeRef) -> Self {
        self.node_ref = Some(r);
        self
    }
    pub fn style(mut self, v: Style) -> Self {
        self.style = v;
        self
    }
    pub fn id(mut self, v: impl ToString) -> Self {
        self.id = Some(v.to_string());
        self
    }
    pub fn key(mut self, v: impl ToString) -> Self {
        self.key = Some(v.to_string());
        self
    }
    pub fn label(mut self, v: impl ToString) -> Self {
        self.label = Some(v.to_string());
        self
    }
    pub fn value(mut self, v: impl ToString) -> Self {
        self.text = v.to_string();
        self
    }
    pub fn src(self, v: impl ToString) -> Self {
        self.value(v)
    }
    pub fn on_click(mut self, f: impl Fn(ClickEvent) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
    pub fn on_input(mut self, f: impl Fn(InputEvent) + 'static) -> Self {
        self.on_input = Some(Rc::new(f));
        self
    }
    pub fn children(mut self, v: Vec<View>) -> Self {
        self.children = v;
        self
    }
}
pub trait IntoView {
    fn into_view(self) -> View;
}
impl IntoView for View {
    fn into_view(self) -> View {
        self
    }
}
impl IntoView for String {
    fn into_view(self) -> View {
        Node::text(self)
    }
}
impl IntoView for &str {
    fn into_view(self) -> View {
        Node::text(self)
    }
}
impl IntoView for Vec<View> {
    fn into_view(self) -> View {
        Node::fragment(self)
    }
}
impl IntoView for Option<View> {
    fn into_view(self) -> View {
        self.unwrap_or_else(|| Node::fragment(vec![]))
    }
}
pub fn into_view(v: impl IntoView) -> View {
    v.into_view()
}
pub fn dynamic<T: IntoView>(f: impl Fn() -> T + 'static) -> View {
    Node {
        binding: Some(Rc::new(move || f().into_view())),
        ..Node::new(Kind::Fragment)
    }
}
pub fn keyed<T, K: ToString>(
    items: impl IntoIterator<Item = T>,
    key: impl Fn(&T) -> K,
    render: impl Fn(T) -> View,
) -> View {
    Node::fragment(
        items
            .into_iter()
            .map(|item| {
                let k = key(&item).to_string();
                render(item).key(k)
            })
            .collect(),
    )
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub control: ControlState,
    pub editing: Option<EditingState>,
    pub role: Option<&'static str>,
    pub position: Option<(usize, usize)>,
    pub drawing: Option<Rc<drawing::CanvasScene>>,
    pub camera: drawing::Camera,
    pub id: u64,
    pub kind: Kind,
    pub text: String,
    pub style: Style,
    pub app_id: Option<String>,
    pub label: Option<String>,
    pub interactive: bool,
    pub children: Vec<Snapshot>,
}
impl Snapshot {
    pub fn focusable(&self) -> bool {
        !self.control.disabled
            && !self.control.hidden
            && (matches!(self.kind, Kind::Button | Kind::Input | Kind::Canvas)
                || self.control.editable
                || self.control.tab_index.is_some()
                || matches!(self.role, Some("tab" | "radio" | "option" | "menuitem")))
    }
    pub fn text_content(&self) -> String {
        let mut s = self
            .control
            .value_text
            .clone()
            .unwrap_or_else(|| self.text.clone());
        for c in &self.children {
            s.push_str(&c.text_content());
        }
        s
    }
    pub fn find(&self, id: &str) -> Option<&Self> {
        if self.app_id.as_deref() == Some(id) {
            Some(self)
        } else {
            self.children.iter().find_map(|c| c.find(id))
        }
    }
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct SemanticNode {
    pub control: ControlState,
    pub focusable: bool,
    pub position: Option<(usize, usize)>,
    pub id: u64,
    pub role: &'static str,
    pub label: String,
    pub value: String,
    pub app_id: Option<String>,
    pub children: Vec<SemanticNode>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Commit {
    pub components_rendered: usize,
    pub nodes_changed: usize,
    pub scopes_unmounted: usize,
}
struct Owner {
    parent: Option<u64>,
    signals: Vec<u64>,
    observers: Vec<u64>,
    cleanups: HashMap<u64, Cleanup>,
    context: HashMap<TypeId, Box<dyn Any>>,
}
struct Value {
    value: Box<dyn Any>,
    subscribers: HashSet<u64>,
}
#[derive(Clone)]
enum Job {
    View(Rc<dyn Fn() -> View>, Weak<RefCell<Mounted>>),
    Memo(Rc<dyn Fn()>),
    Effect(Rc<dyn Fn() -> Cleanup>),
}
struct Observer {
    job: Job,
    cleanup: Option<Cleanup>,
}
struct Inner {
    next: u64,
    owners: HashMap<u64, Owner>,
    values: HashMap<u64, Value>,
    observers: HashMap<u64, Observer>,
    dirty: BTreeSet<u64>,
    wake: Option<Rc<dyn Fn()>>,
    stats: Commit,
}
thread_local! {static TRACK:RefCell<Option<(Weak<RefCell<Inner>>,u64)>>=const{RefCell::new(None)};}
struct TrackGuard(Option<(Weak<RefCell<Inner>>, u64)>);
impl Drop for TrackGuard {
    fn drop(&mut self) {
        TRACK.with(|t| *t.borrow_mut() = self.0.take());
    }
}
fn track(inner: &Rc<RefCell<Inner>>, id: u64) -> TrackGuard {
    TrackGuard(TRACK.with(|t| t.replace(Some((Rc::downgrade(inner), id)))))
}
fn untracked() -> TrackGuard {
    TrackGuard(TRACK.with(|t| t.replace(None)))
}
fn next(i: &Rc<RefCell<Inner>>) -> u64 {
    let mut b = i.borrow_mut();
    let id = b.next;
    assert!(id <= MAX_HOST_ID, "host ID space exhausted");
    b.next = b.next.checked_add(1).expect("ID space exhausted");
    id
}
fn owner(i: &Rc<RefCell<Inner>>, parent: Option<u64>) -> Scope {
    let id = next(i);
    i.borrow_mut().owners.insert(
        id,
        Owner {
            parent,
            signals: vec![],
            observers: vec![],
            cleanups: HashMap::new(),
            context: HashMap::new(),
        },
    );
    Scope {
        inner: Rc::downgrade(i),
        id,
    }
}
#[derive(Clone)]
pub struct Scope {
    inner: Weak<RefCell<Inner>>,
    id: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disposed;
pub struct Signal<T> {
    inner: Weak<RefCell<Inner>>,
    id: u64,
    _type: std::marker::PhantomData<T>,
}
impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            id: self.id,
            _type: std::marker::PhantomData,
        }
    }
}
impl Scope {
    pub fn node_ref(&self) -> NodeRef {
        NodeRef::default()
    }
    pub fn is_alive(&self) -> bool {
        self.inner
            .upgrade()
            .is_some_and(|i| i.borrow().owners.contains_key(&self.id))
    }
    pub fn signal<T: 'static>(&self, value: T) -> Signal<T> {
        let i = self.inner.upgrade().expect("disposed scope");
        assert!(self.is_alive(), "disposed scope");
        let id = next(&i);
        let mut b = i.borrow_mut();
        b.values.insert(
            id,
            Value {
                value: Box::new(value),
                subscribers: HashSet::new(),
            },
        );
        b.owners.get_mut(&self.id).unwrap().signals.push(id);
        Signal {
            inner: self.inner.clone(),
            id,
            _type: std::marker::PhantomData,
        }
    }
    fn observer(&self, job: Job) -> u64 {
        let i = self.inner.upgrade().expect("disposed scope");
        assert!(self.is_alive(), "disposed scope");
        let id = next(&i);
        let mut b = i.borrow_mut();
        b.observers.insert(id, Observer { job, cleanup: None });
        b.owners.get_mut(&self.id).unwrap().observers.push(id);
        b.dirty.insert(id);
        id
    }
    pub fn memo<T: Clone + PartialEq + 'static>(&self, f: impl Fn() -> T + 'static) -> Signal<T> {
        let v = {
            let _g = untracked();
            f()
        };
        let signal = self.signal(v);
        let out = signal.clone();
        let id = self.observer(Job::Memo(Rc::new(move || {
            let v = f();
            if out.peek().ok().as_ref() != Some(&v) {
                let _ = out.set(v);
            }
        })));
        let i = self.inner.upgrade().unwrap();
        run_job(&i, id);
        signal
    }
    pub fn effect(&self, f: impl Fn() -> Cleanup + 'static) {
        self.observer(Job::Effect(Rc::new(f)));
    }
    pub fn on_cleanup(&self, f: impl FnOnce() + 'static) -> u64 {
        let i = self.inner.upgrade().expect("disposed scope");
        let id = next(&i);
        i.borrow_mut()
            .owners
            .get_mut(&self.id)
            .expect("disposed scope")
            .cleanups
            .insert(id, Box::new(f));
        id
    }
    fn remove_cleanup(&self, id: u64) {
        if let Some(i) = self.inner.upgrade() {
            if let Some(o) = i.borrow_mut().owners.get_mut(&self.id) {
                o.cleanups.remove(&id);
            }
        }
    }
    pub fn cleanup_count(&self) -> usize {
        self.inner
            .upgrade()
            .and_then(|i| i.borrow().owners.get(&self.id).map(|o| o.cleanups.len()))
            .unwrap_or(0)
    }
    pub fn provide<T: 'static>(&self, value: T) {
        let i = self.inner.upgrade().expect("disposed scope");
        i.borrow_mut()
            .owners
            .get_mut(&self.id)
            .unwrap()
            .context
            .insert(TypeId::of::<T>(), Box::new(value));
    }
    pub fn context<T: Clone + 'static>(&self) -> Option<T> {
        let i = self.inner.upgrade()?;
        let b = i.borrow();
        let mut id = self.id;
        loop {
            let o = b.owners.get(&id)?;
            if let Some(v) = o.context.get(&TypeId::of::<T>()) {
                return v.downcast_ref::<T>().cloned();
            }
            id = o.parent?;
        }
    }
}
impl<T: Clone + 'static> Signal<T> {
    pub fn try_get(&self) -> Result<T, Disposed> {
        let i = self.inner.upgrade().ok_or(Disposed)?;
        let tracker = TRACK.with(|t| t.borrow().clone());
        let mut b = i.borrow_mut();
        let v = b.values.get_mut(&self.id).ok_or(Disposed)?;
        if let Some((other, id)) = tracker {
            if Weak::ptr_eq(&other, &self.inner) {
                v.subscribers.insert(id);
            }
        }
        v.value.downcast_ref::<T>().cloned().ok_or(Disposed)
    }
    pub fn get(&self) -> T {
        self.try_get().expect("signal read after scope disposal")
    }
    pub fn peek(&self) -> Result<T, Disposed> {
        let _g = untracked();
        self.try_get()
    }
    pub fn set(&self, value: T) -> Result<(), Disposed> {
        let i = self.inner.upgrade().ok_or(Disposed)?;
        let wake = {
            let mut b = i.borrow_mut();
            let v = b.values.get_mut(&self.id).ok_or(Disposed)?;
            v.value = Box::new(value);
            let subscribers = v.subscribers.clone();
            b.dirty.extend(subscribers);
            b.wake.clone()
        };
        if let Some(f) = wake {
            f()
        }
        Ok(())
    }
    pub fn update(&self, f: impl FnOnce(&mut T)) -> Result<(), Disposed> {
        let mut v = self.peek()?;
        f(&mut v);
        self.set(v)
    }
}
struct Mounted {
    id: u64,
    node: Node,
    scope: Scope,
    children: Vec<Mount>,
}
fn dispose(i: &Rc<RefCell<Inner>>, m: &Mount) {
    let (children, id) = {
        let b = m.borrow();
        if let Some(r) = &b.node.node_ref {
            r.0.set(None);
        }
        (b.children.clone(), b.scope.id)
    };
    for c in children {
        dispose(i, &c)
    }
    let cleanups = {
        let mut b = i.borrow_mut();
        let Some(mut o) = b.owners.remove(&id) else {
            return;
        };
        for id in o.signals {
            b.values.remove(&id);
        }
        for id in o.observers {
            b.dirty.remove(&id);
            if let Some(mut obs) = b.observers.remove(&id) {
                if let Some(c) = obs.cleanup.take() {
                    o.cleanups.insert(id, c);
                }
            }
            for v in b.values.values_mut() {
                v.subscribers.remove(&id);
            }
        }
        b.stats.scopes_unmounted += 1;
        o.cleanups
    };
    for (_, cleanup) in cleanups {
        cleanup()
    }
}
fn compatible(a: &Node, b: &Node) -> bool {
    a.key == b.key
        && match (&a.component, &b.component, &a.binding, &b.binding) {
            (Some((a, _)), Some((b, _)), _, _) => a == b,
            (None, None, Some(_), Some(_)) => true,
            (None, None, None, None) => a.kind == b.kind,
            _ => false,
        }
}
fn reconcile(
    i: &Rc<RefCell<Inner>>,
    old: Option<Mount>,
    mut node: Node,
    parent: Option<u64>,
) -> Mount {
    if node.control.selectable {
        node.control.editable = true;
        node.control.read_only = true;
        node.control.multiline = true;
    }
    if let Some(editor) = &node.editing {
        editor.set_read_only(node.control.read_only);
        let _ = editor.clone().with_password(node.control.password());
    }
    let old = old.filter(|m| {
        let same = compatible(&m.borrow().node, &node);
        if !same {
            dispose(i, m)
        }
        same
    });
    if let Some(m) = old {
        if node.component.is_some() {
            return m;
        }
        if (node.kind == Kind::Input || node.control.editable) && node.editing.is_none() {
            let old = m.borrow();
            if let Some(editor) = &old.node.editing {
                if node.text != old.node.text && editor.value() != node.text {
                    let _ = editor.set_controlled_value(node.text.clone());
                }
                node.editing = Some(editor.clone().with_password(node.control.password()));
                editor.set_read_only(node.control.read_only);
            }
        }
        {
            let b = m.borrow();
            if let Some(r) = &b.node.node_ref {
                r.0.set(None);
            }
            if let Some(r) = &node.node_ref {
                r.0.set(Some(b.id));
            }
        }
        m.borrow_mut().node = node.clone();
        if let Some(f) = node.binding {
            let scope = m.borrow().scope.clone();
            let ids = i.borrow().owners[&scope.id].observers.clone();
            for id in ids {
                let mut b = i.borrow_mut();
                if let Some(o) = b.observers.get_mut(&id) {
                    if matches!(o.job, Job::View(..)) {
                        o.job = Job::View(f.clone(), Rc::downgrade(&m));
                        b.dirty.insert(id);
                    }
                }
            }
        } else {
            reconcile_children(i, &m, node.children)
        }
        i.borrow_mut().stats.nodes_changed += 1;
        return m;
    }
    let scope = owner(i, parent);
    if (node.kind == Kind::Input || node.control.editable) && node.editing.is_none() {
        node.editing = Some(
            Editor::new(node.text.clone())
                .with_password(node.control.password())
                .with_read_only(node.control.read_only),
        );
    }
    let m = Rc::new(RefCell::new(Mounted {
        id: next(i),
        node: node.clone(),
        scope: scope.clone(),
        children: vec![],
    }));
    if let Some(r) = &node.node_ref {
        r.0.set(Some(m.borrow().id));
    }
    if let Some((_, factory)) = node.component {
        let view = {
            let _g = untracked();
            factory(scope)
        };
        reconcile_children(i, &m, vec![view]);
        i.borrow_mut().stats.components_rendered += 1;
    } else if let Some(f) = node.binding {
        let id = scope.observer(Job::View(f, Rc::downgrade(&m)));
        run_job(i, id);
    } else {
        reconcile_children(i, &m, node.children)
    }
    i.borrow_mut().stats.nodes_changed += 1;
    m
}
fn reconcile_children(i: &Rc<RefCell<Inner>>, parent: &Mount, nodes: Vec<Node>) {
    let mut seen = HashSet::new();
    for n in &nodes {
        if let Some(k) = &n.key {
            assert!(seen.insert(k.clone()), "duplicate sibling key: {k}");
        }
    }
    let old = std::mem::take(&mut parent.borrow_mut().children);
    let mut slots: Vec<Option<Mount>> = old.into_iter().map(Some).collect();
    let mut result = vec![];
    let scope = parent.borrow().scope.id;
    for (index, node) in nodes.into_iter().enumerate() {
        let pos = if let Some(k) = &node.key {
            slots.iter().position(|m| {
                m.as_ref()
                    .is_some_and(|m| m.borrow().node.key.as_ref() == Some(k))
            })
        } else {
            Some(index).filter(|x| {
                slots
                    .get(*x)
                    .and_then(Option::as_ref)
                    .is_some_and(|m| m.borrow().node.key.is_none())
            })
        };
        result.push(reconcile(
            i,
            pos.and_then(|x| slots[x].take()),
            node,
            Some(scope),
        ));
    }
    for old in slots.into_iter().flatten() {
        dispose(i, &old)
    }
    parent.borrow_mut().children = result;
}
fn run_job(i: &Rc<RefCell<Inner>>, id: u64) {
    let (job, cleanup) = {
        let mut b = i.borrow_mut();
        b.dirty.remove(&id);
        for v in b.values.values_mut() {
            v.subscribers.remove(&id);
        }
        let Some(o) = b.observers.get_mut(&id) else {
            return;
        };
        (o.job.clone(), o.cleanup.take())
    };
    if let Some(c) = cleanup {
        let _g = untracked();
        c()
    }
    let _g = track(i, id);
    match job {
        Job::View(f, m) => {
            if let Some(m) = m.upgrade() {
                let node = f();
                reconcile_children(i, &m, vec![node]);
                i.borrow_mut().stats.components_rendered += 1;
            }
        }
        Job::Memo(f) => f(),
        Job::Effect(f) => {
            let c = f();
            let mut b = i.borrow_mut();
            if let Some(o) = b.observers.get_mut(&id) {
                o.cleanup = Some(c)
            } else {
                drop(b);
                c()
            }
        }
    }
}
pub struct Runtime {
    inner: Rc<RefCell<Inner>>,
    root: Mount,
    effect_cascade: usize,
    pointer_captures: HashMap<u64, NodeId>,
    model_epoch: u64,
}
impl Runtime {
    pub fn new(f: impl Fn(Scope) -> View + 'static) -> Self {
        let i = Rc::new(RefCell::new(Inner {
            next: 1,
            owners: HashMap::new(),
            values: HashMap::new(),
            observers: HashMap::new(),
            dirty: BTreeSet::new(),
            wake: None,
            stats: Commit::default(),
        }));
        let root = reconcile(&i, None, Node::component("$root", f), None);
        let mut rt = Self {
            inner: i,
            root,
            effect_cascade: 0,
            pointer_captures: HashMap::new(),
            model_epoch: 0,
        };
        rt.flush();
        rt
    }
    pub fn set_waker(&mut self, f: impl Fn() + 'static) {
        self.inner.borrow_mut().wake = Some(Rc::new(f));
    }
    pub fn flush(&mut self) -> Commit {
        self.inner.borrow_mut().stats = Commit::default();
        for _ in 0..1000 {
            let next = {
                let b = self.inner.borrow();
                b.dirty
                    .iter()
                    .find(|id| matches!(b.observers.get(id).map(|o| &o.job), Some(Job::Memo(_))))
                    .or_else(|| {
                        b.dirty.iter().find(|id| {
                            matches!(b.observers.get(id).map(|o| &o.job), Some(Job::View(..)))
                        })
                    })
                    .copied()
            };
            if let Some(id) = next {
                run_job(&self.inner, id)
            } else {
                break;
            }
        }
        assert!(
            !{
                let b = self.inner.borrow();
                b.dirty.iter().any(|id| {
                    b.observers
                        .get(id)
                        .is_some_and(|o| !matches!(o.job, Job::Effect(_)))
                })
            },
            "reactive cycle exceeded 1000 jobs"
        );
        let effects = {
            let b = self.inner.borrow();
            b.dirty
                .iter()
                .filter(|id| {
                    b.observers
                        .get(id)
                        .is_some_and(|o| matches!(o.job, Job::Effect(_)))
                })
                .copied()
                .collect::<Vec<_>>()
        };
        for id in effects {
            run_job(&self.inner, id)
        }
        if self.needs_update() {
            self.effect_cascade += 1;
            assert!(
                self.effect_cascade <= 100,
                "effect update cycle exceeded 100 commits"
            );
        } else {
            self.effect_cascade = 0;
        }
        if !self.pointer_captures.is_empty() {
            fn ids(m: &Mount, out: &mut HashSet<NodeId>) {
                let b = m.borrow();
                out.insert(b.id);
                for c in &b.children {
                    ids(c, out);
                }
            }
            let mut alive = HashSet::new();
            ids(&self.root, &mut alive);
            self.pointer_captures.retain(|_, id| alive.contains(id));
        }
        self.inner.borrow().stats
    }
    pub fn needs_update(&self) -> bool {
        !self.inner.borrow().dirty.is_empty()
    }
    pub fn snapshot(&self) -> Snapshot {
        fn snap(m: &Mount) -> Snapshot {
            let b = m.borrow();
            let mut children: Vec<_> = b.children.iter().map(snap).collect();
            if b.node.control.selectable {
                // Text/Paragraph children are text content, not a second
                // accessible editing representation. Retain their scopes but
                // expose a single selected string and Rust layout box.
                let mut content = b.node.text.clone();
                for child in &children {
                    content.push_str(&child.text_content());
                }
                if let Some(editor) = &b.node.editing {
                    if editor.value() != content {
                        let _ = editor.set_controlled_value(content);
                    }
                }
                children.clear();
            }
            Snapshot {
                control: b.node.control.clone(),
                editing: b.node.editing.as_ref().map(Editor::snapshot),
                id: b.id,
                role: b.node.role,
                position: b.node.position,
                drawing: b.node.drawing.clone(),
                camera: b.node.camera,
                kind: b.node.kind,
                text: if let Some(e) = &b.node.editing {
                    e.snapshot().text
                } else if b.node.control.password() {
                    "•".repeat(b.node.text.chars().count())
                } else {
                    b.node.text.clone()
                },
                style: b.node.style.clone(),
                app_id: b.node.id.clone(),
                label: b.node.label.clone(),
                interactive: b.node.on_click.is_some()
                    || b.node.on_input.is_some()
                    || b.node.drawing.is_some()
                    || b.node.on_scroll.is_some()
                    || b.node.control.editable
                    || !b.node.listeners.is_empty()
                    || matches!(b.node.kind, Kind::Button | Kind::Input),
                children,
            }
        }
        snap(&self.root)
    }
    pub fn semantics(&self) -> SemanticNode {
        fn sem(s: Snapshot) -> SemanticNode {
            let role = if s.control.selectable {
                "textbox"
            } else {
                s.role.unwrap_or(match s.kind {
                    Kind::Button => "button",
                    Kind::Input => "textbox",
                    Kind::Text => "text",
                    Kind::Image | Kind::Canvas => "img",
                    _ => "group",
                })
            };
            let mut control = s.control.clone();
            if matches!(role, "slider" | "progressbar") && control.value_now.is_none() {
                control.value_now = s
                    .text
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite());
            }
            if control.direction.is_none() {
                control.direction = match s.style.text_direction {
                    TextDirection::Auto => None,
                    TextDirection::LeftToRight => Some("ltr".into()),
                    TextDirection::RightToLeft => Some("rtl".into()),
                };
            }
            SemanticNode {
                control,
                focusable: s.focusable(),
                position: s.position,
                id: s.id,
                role,
                label: s.label.clone().unwrap_or_else(|| {
                    if matches!(s.kind, Kind::Button | Kind::Text) {
                        s.text_content()
                    } else {
                        s.text.clone()
                    }
                }),
                value: if s.control.password() {
                    String::new()
                } else if s.kind == Kind::Input || s.control.editable || s.role == Some("slider") {
                    s.text.clone()
                } else {
                    String::new()
                },
                app_id: s.app_id,
                children: s.children.into_iter().map(sem).collect(),
            }
        }
        sem(self.snapshot())
    }
    fn find(&self, id: u64) -> Option<Node> {
        fn find(m: &Mount, id: u64) -> Option<Node> {
            let b = m.borrow();
            if b.id == id {
                Some(b.node.clone())
            } else {
                b.children.iter().find_map(|c| find(c, id))
            }
        }
        find(&self.root, id)
    }
    pub fn scope_for_node(&self, id: NodeId) -> Option<Scope> {
        fn find(m: &Mount, id: NodeId) -> Option<Scope> {
            let b = m.borrow();
            if b.id == id {
                Some(b.scope.clone())
            } else {
                b.children.iter().find_map(|c| find(c, id))
            }
        }
        find(&self.root, id)
    }
    pub fn click(&mut self, id: u64, x: f32, y: f32) -> bool {
        let mut event = EventData::new(EventKind::Click);
        event.x = x as f64;
        event.y = y as f64;
        self.dispatch(id, event).handled
    }
    pub fn input(&mut self, id: u64, value: String, composing: bool) -> bool {
        if !self
            .find(id)
            .is_some_and(|n| !n.control.disabled && !n.control.read_only)
        {
            return false;
        }
        if let Some(editor) = self.editor(id) {
            if editor.value() != value {
                let end = value.encode_utf16().count();
                if editor.sync(value.clone(), end, end, composing).is_err() {
                    return false;
                }
                self.invalidate_model();
            }
        }
        let mut event = EventData::new(EventKind::Input);
        event.value = value;
        event.composing = composing;
        self.dispatch(id, event).handled
    }
    /// The input bridge is the only diagnostic-free path to a password's actual value.
    pub fn edit_value(&self, id: NodeId) -> Option<String> {
        let node = self.find(id)?;
        Some(node.editing.map_or(node.text, |editor| editor.value()))
    }
    pub fn editor(&self, id: NodeId) -> Option<Editor> {
        self.find(id)?.editing
    }
    pub fn model_epoch(&self) -> u64 {
        self.model_epoch
    }
    pub fn invalidate_model(&mut self) {
        self.model_epoch = self.model_epoch.wrapping_add(1);
    }
    pub fn edit_sync(
        &mut self,
        id: NodeId,
        value: String,
        anchor_utf16: usize,
        focus_utf16: usize,
        composing: bool,
    ) -> bool {
        let Some(node) = self.find(id) else {
            return false;
        };
        if node.control.disabled {
            return false;
        }
        let old = node
            .editing
            .as_ref()
            .map_or_else(|| node.text.clone(), Editor::value);
        if node.control.read_only && old != value {
            return false;
        }
        if let Some(editor) = node.editing {
            if editor
                .sync(value.clone(), anchor_utf16, focus_utf16, composing)
                .is_err()
            {
                return false;
            }
        }
        self.invalidate_model();
        if old != value {
            self.input(id, value, composing);
        }
        true
    }
    pub fn captured_pointer(&self, pointer_id: u64) -> Option<NodeId> {
        self.pointer_captures
            .get(&pointer_id)
            .copied()
            .filter(|id| self.find(*id).is_some())
    }
    pub fn capture_pointer(&mut self, pointer_id: u64, id: NodeId) -> bool {
        if self.find(id).is_none()
            || self.pointer_captures.len() >= 64 && !self.pointer_captures.contains_key(&pointer_id)
        {
            return false;
        }
        self.pointer_captures.insert(pointer_id, id);
        true
    }
    /// Dispatch against a cloned committed path. A handler may update signals without
    /// changing this event's path; the next commit observes those updates.
    pub fn dispatch(&mut self, target: NodeId, data: EventData) -> DispatchResult {
        self.dispatch_with_origins(target, data, &HashMap::new())
    }
    pub fn dispatch_with_origins(
        &mut self,
        target: NodeId,
        data: EventData,
        origins: &HashMap<NodeId, (f64, f64)>,
    ) -> DispatchResult {
        if !data.is_valid() {
            return DispatchResult {
                default_prevented: true,
                ..Default::default()
            };
        }
        fn path(m: &Mount, id: NodeId, out: &mut Vec<(NodeId, Node)>) -> bool {
            let b = m.borrow();
            let before = out.len();
            out.push((b.id, b.node.clone()));
            if b.id == id || b.children.iter().any(|c| path(c, id, out)) {
                true
            } else {
                out.truncate(before);
                false
            }
        }
        let pointer = matches!(
            data.kind,
            EventKind::PointerMove | EventKind::PointerUp | EventKind::PointerCancel
        );
        let target = if pointer {
            self.captured_pointer(data.pointer_id).unwrap_or(target)
        } else {
            target
        };
        // Terminal events end capture even if the committed target can no
        // longer receive input (for example after disabling it or opening a
        // modal). Preserve the captured target for this dispatch first.
        let pointer_released = matches!(data.kind, EventKind::PointerUp | EventKind::PointerCancel)
            && self.pointer_captures.remove(&data.pointer_id).is_some();
        let mut nodes = vec![];
        if !path(&self.root, target, &mut nodes) {
            self.pointer_captures.remove(&data.pointer_id);
            return DispatchResult {
                pointer_released,
                ..Default::default()
            };
        }
        fn modal(m: &Mount) -> Option<NodeId> {
            let b = m.borrow();
            if b.node.control.hidden {
                return None;
            }
            b.children
                .iter()
                .rev()
                .find_map(modal)
                .or_else(|| b.node.control.modal.then_some(b.id))
        }
        if let Some(modal) = modal(&self.root) {
            if !nodes.iter().any(|(id, _)| *id == modal)
                && !matches!(
                    data.kind,
                    EventKind::Blur
                        | EventKind::ResourceError
                        | EventKind::Resize
                        | EventKind::PointerCancel
                )
            {
                return DispatchResult {
                    default_prevented: true,
                    pointer_released,
                    ..Default::default()
                };
            }
        }
        if nodes
            .iter()
            .any(|(_, n)| n.control.disabled || n.control.hidden)
        {
            return DispatchResult {
                default_prevented: true,
                pointer_released,
                ..Default::default()
            };
        }
        let state = Rc::new(RefCell::new(events::DispatchState::default()));
        let invoke = |id, node: &Node, capture, phase| {
            state.borrow_mut().immediate = false;
            for listener in &node.listeners {
                if listener.kind == data.kind && listener.capture == capture {
                    state.borrow_mut().result.handled = true;
                    let origin = origins.get(&id).copied().unwrap_or((0., 0.));
                    (listener.callback)(Event {
                        target,
                        current_target: id,
                        phase,
                        local_x: data.x - origin.0,
                        local_y: data.y - origin.1,
                        data: data.clone(),
                        state: state.clone(),
                    });
                    if state.borrow().immediate {
                        break;
                    }
                }
            }
        };
        for (id, node) in nodes.iter().take(nodes.len().saturating_sub(1)) {
            invoke(*id, node, true, EventPhase::Capture);
            if state.borrow().stopped {
                break;
            }
        }
        if !state.borrow().stopped {
            let (id, node) = nodes.last().unwrap();
            invoke(*id, node, true, EventPhase::Target);
            if !state.borrow().immediate {
                invoke(*id, node, false, EventPhase::Target);
            }
        }
        if !state.borrow().stopped {
            for (id, node) in nodes.iter().rev().skip(1) {
                invoke(*id, node, false, EventPhase::Bubble);
                if state.borrow().stopped {
                    break;
                }
            }
        }
        let node = &nodes.last().unwrap().1;
        if node.control.disabled || node.control.hidden {
            state.borrow_mut().result.default_prevented = true;
        }
        if !state.borrow().result.default_prevented {
            match data.kind {
                EventKind::Click => {
                    if let Some(f) = &node.on_click {
                        f(ClickEvent {
                            target,
                            x: data.x as f32,
                            y: data.y as f32,
                        });
                        state.borrow_mut().result.handled = true;
                    }
                    if !node.control.read_only
                        && matches!(node.role, Some("checkbox" | "switch" | "radio"))
                    {
                        if let Some(f) = &node.on_input {
                            f(InputEvent {
                                target,
                                value: if node.role == Some("radio") {
                                    "true".into()
                                } else {
                                    (!node.control.checked.unwrap_or(false)).to_string()
                                },
                                composing: false,
                            });
                            state.borrow_mut().result.handled = true;
                        }
                    }
                }
                EventKind::Input if !node.control.read_only => {
                    if let Some(f) = &node.on_input {
                        f(InputEvent {
                            target,
                            value: data.value.clone(),
                            composing: data.composing,
                        });
                        state.borrow_mut().result.handled = true;
                    }
                }
                EventKind::KeyDown if !data.ctrl && !data.meta && !data.alt && !data.composing => {
                    let role = node.role.unwrap_or("");
                    if (data.key == "Enter" || data.key == " ")
                        && node.kind == Kind::Button
                        && role != "link"
                    {
                        let clicked = self.click(target, data.x as f32, data.y as f32);
                        let mut s = state.borrow_mut();
                        s.result.handled |= clicked;
                        s.result.default_prevented = clicked;
                    } else if role == "slider" && !node.control.read_only {
                        let min = node.control.min.unwrap_or(0.);
                        let max = node.control.max.unwrap_or(100.).max(min);
                        let step = node.control.step.unwrap_or(1.);
                        let old: f64 = node.text.parse().unwrap_or(min);
                        let horizontal = if node.style.text_direction == TextDirection::RightToLeft
                        {
                            -step
                        } else {
                            step
                        };
                        let next = match data.key.as_str() {
                            "ArrowRight" => Some(old + horizontal),
                            "ArrowLeft" => Some(old - horizontal),
                            "ArrowUp" => Some(old + step),
                            "ArrowDown" => Some(old - step),
                            "Home" => Some(min),
                            "End" => Some(max),
                            "PageUp" => Some(old + step * 10.),
                            "PageDown" => Some(old - step * 10.),
                            _ => None,
                        };
                        if let (Some(next), Some(f)) = (next, &node.on_input) {
                            f(InputEvent {
                                target,
                                value: next.clamp(min, max).to_string(),
                                composing: false,
                            });
                            let mut s = state.borrow_mut();
                            s.result.handled = true;
                            s.result.default_prevented = true;
                        }
                    } else if role == "combobox"
                        && !node.control.options.is_empty()
                        && !node.control.read_only
                    {
                        let choices: Vec<_> = node
                            .control
                            .options
                            .iter()
                            .filter(|c| !c.disabled)
                            .collect();
                        if !choices.is_empty() {
                            let at = choices
                                .iter()
                                .position(|c| c.value == node.text)
                                .unwrap_or(0);
                            let next = match data.key.as_str() {
                                "ArrowDown" => Some((at + 1) % choices.len()),
                                "ArrowUp" => Some((at + choices.len() - 1) % choices.len()),
                                "Home" => Some(0),
                                "End" => Some(choices.len() - 1),
                                _ => None,
                            };
                            if let (Some(next), Some(f)) = (next, &node.on_input) {
                                f(InputEvent {
                                    target,
                                    value: choices[next].value.clone(),
                                    composing: false,
                                });
                                let mut s = state.borrow_mut();
                                s.result.handled = true;
                                s.result.default_prevented = true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let mut result = state.borrow().result.clone();
        result.pointer_released |= pointer_released;
        if let Some(id) = result.pointer_capture {
            if self.pointer_captures.len() < 64
                || self.pointer_captures.contains_key(&data.pointer_id)
            {
                self.pointer_captures.insert(data.pointer_id, id);
            } else {
                result.pointer_capture = None;
            }
        }
        if result.pointer_released
            || matches!(data.kind, EventKind::PointerUp | EventKind::PointerCancel)
        {
            result.pointer_released |= self.pointer_captures.remove(&data.pointer_id).is_some();
        }
        result
    }
    pub fn focus(&mut self, id: u64) {
        fn path(m: &Mount, id: u64, out: &mut Vec<Rc<dyn Fn()>>) -> bool {
            let b = m.borrow();
            let before = out.len();
            if let Some(f) = &b.node.on_focus {
                out.push(f.clone());
            }
            if b.id == id || b.children.iter().any(|c| path(c, id, out)) {
                return true;
            }
            out.truncate(before);
            false
        }
        let mut callbacks = vec![];
        if path(&self.root, id, &mut callbacks) {
            for f in callbacks {
                f();
            }
        }
    }
    pub fn canvas(&mut self, id: u64, event: drawing::CanvasEvent) -> bool {
        if let Some(f) = self.find(id).and_then(|n| n.on_canvas) {
            f(event);
            true
        } else {
            false
        }
    }
    pub fn scroll(&mut self, id: u64, delta: f64) -> bool {
        if !delta.is_finite() {
            return false;
        }
        fn path(m: &Mount, id: u64, out: &mut Vec<Rc<dyn Fn(f64)>>) -> bool {
            let b = m.borrow();
            let before = out.len();
            if let Some(f) = &b.node.on_scroll {
                out.push(f.clone());
            }
            if b.id == id || b.children.iter().any(|c| path(c, id, out)) {
                true
            } else {
                out.truncate(before);
                false
            }
        }
        let mut callbacks = vec![];
        if path(&self.root, id, &mut callbacks) {
            if let Some(f) = callbacks.last() {
                f(delta);
                return true;
            }
        }
        false
    }
    pub fn click_id(&mut self, id: &str) -> bool {
        self.snapshot()
            .find(id)
            .map(|n| n.id)
            .is_some_and(|id| self.click(id, 0., 0.))
    }
    pub fn scope_count(&self) -> usize {
        self.inner.borrow().owners.len()
    }
    pub fn counts(&self) -> (usize, usize, usize) {
        let b = self.inner.borrow();
        (b.owners.len(), b.values.len(), b.observers.len())
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        dispose(&self.inner, &self.root)
    }
}
