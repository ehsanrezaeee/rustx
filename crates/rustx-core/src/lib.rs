//! Mount-once components; owned signals and automatically tracked reactive bindings.
mod style;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashMap, HashSet},
    rc::{Rc, Weak},
};
pub use style::*;
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
#[derive(Clone, Debug)]
pub struct InputEvent {
    pub target: u64,
    pub value: String,
    pub composing: bool,
}
#[derive(Clone)]
pub struct Node {
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
    pub fn text_content(&self) -> String {
        let mut s = self.text.clone();
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
fn reconcile(i: &Rc<RefCell<Inner>>, old: Option<Mount>, node: Node, parent: Option<u64>) -> Mount {
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
        self.inner.borrow().stats
    }
    pub fn needs_update(&self) -> bool {
        !self.inner.borrow().dirty.is_empty()
    }
    pub fn snapshot(&self) -> Snapshot {
        fn snap(m: &Mount) -> Snapshot {
            let b = m.borrow();
            Snapshot {
                id: b.id,
                kind: b.node.kind,
                text: b.node.text.clone(),
                style: b.node.style.clone(),
                app_id: b.node.id.clone(),
                label: b.node.label.clone(),
                interactive: b.node.on_click.is_some() || b.node.on_input.is_some(),
                children: b.children.iter().map(snap).collect(),
            }
        }
        snap(&self.root)
    }
    pub fn semantics(&self) -> SemanticNode {
        fn sem(s: Snapshot) -> SemanticNode {
            let role = match s.kind {
                Kind::Button => "button",
                Kind::Input => "textbox",
                Kind::Text => "text",
                Kind::Image => "img",
                _ => "group",
            };
            SemanticNode {
                id: s.id,
                role,
                label: s.label.clone().unwrap_or_else(|| {
                    if s.kind == Kind::Button {
                        s.text_content()
                    } else {
                        s.text.clone()
                    }
                }),
                value: if s.kind == Kind::Input {
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
    pub fn click(&mut self, id: u64, x: f32, y: f32) -> bool {
        if let Some(f) = self.find(id).and_then(|n| n.on_click) {
            f(ClickEvent { target: id, x, y });
            true
        } else {
            false
        }
    }
    pub fn input(&mut self, id: u64, value: String, composing: bool) -> bool {
        if let Some(f) = self.find(id).and_then(|n| n.on_input) {
            f(InputEvent {
                target: id,
                value,
                composing,
            });
            true
        } else {
            false
        }
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
#[derive(Clone, Debug, serde::Serialize)]
pub struct HttpRequest {
    pub id: u64,
    pub url: String,
    pub max_bytes: usize,
    pub timeout_ms: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostError {
    Unsupported(&'static str),
    Cancelled,
    Timeout,
    TooLarge,
    Transport(String),
    Http(u16),
    Decode(String),
}
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "kind")]
pub enum HostCommand {
    Get(HttpRequest),
    Cancel { id: u64 },
}
type Callback = Box<dyn FnOnce(Result<HttpResponse, HostError>)>;
struct Requests {
    next: u64,
    queue: Vec<HostCommand>,
    pending: HashMap<u64, (Scope, u64, Callback)>,
}
#[derive(Clone)]
pub struct HttpClient(Rc<RefCell<Requests>>);
impl Default for HttpClient {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(Requests {
            next: 1,
            queue: vec![],
            pending: HashMap::new(),
        })))
    }
}
impl HttpClient {
    pub fn get(
        &self,
        scope: &Scope,
        url: impl Into<String>,
        f: impl FnOnce(Result<HttpResponse, HostError>) + 'static,
    ) -> u64 {
        let id = {
            let mut b = self.0.borrow_mut();
            let id = b.next;
            assert!(id <= MAX_HOST_ID, "host ID space exhausted");
            b.next = b.next.checked_add(1).expect("request ID exhausted");
            if !scope.is_alive() {
                drop(b);
                f(Err(HostError::Cancelled));
                return id;
            }
            b.pending.insert(id, (scope.clone(), 0, Box::new(f)));
            b.queue.push(HostCommand::Get(HttpRequest {
                id,
                url: url.into(),
                max_bytes: 1_048_576,
                timeout_ms: 10000,
            }));
            id
        };
        let weak = Rc::downgrade(&self.0);
        let cleanup = scope.on_cleanup(move || {
            if let Some(i) = weak.upgrade() {
                let cb = {
                    let mut b = i.borrow_mut();
                    let c = b.pending.remove(&id);
                    if c.is_some() {
                        b.queue.push(HostCommand::Cancel { id });
                    }
                    c
                };
                drop(cb)
            }
        });
        if let Some(v) = self.0.borrow_mut().pending.get_mut(&id) {
            v.1 = cleanup;
        }
        id
    }
    pub fn cancel(&self, id: u64) {
        let cb = {
            let mut b = self.0.borrow_mut();
            let cb = b.pending.remove(&id);
            if cb.is_some() {
                b.queue.push(HostCommand::Cancel { id });
            }
            cb
        };
        if let Some((s, c, cb)) = cb {
            s.remove_cleanup(c);
            if s.is_alive() {
                cb(Err(HostError::Cancelled))
            }
        }
    }
    pub fn complete(&self, id: u64, result: Result<HttpResponse, HostError>) -> bool {
        let cb = self.0.borrow_mut().pending.remove(&id);
        if let Some((s, c, cb)) = cb {
            s.remove_cleanup(c);
            if s.is_alive() {
                cb(result);
                return true;
            }
        }
        false
    }
    pub fn drain(&self) -> Vec<HostCommand> {
        std::mem::take(&mut self.0.borrow_mut().queue)
    }
    pub fn pending(&self) -> usize {
        self.0.borrow().pending.len()
    }
}
