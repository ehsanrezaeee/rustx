use rustx::prelude::*;
use std::{cell::RefCell, rc::Rc};
#[component]
fn Item(cx: Scope, name: String) -> View {
    let value = cx.signal(0);
    let shown = value.clone();
    rustx! {<Div><Button id={name.clone()} on_click={move |_|{value.update(|v|*v+=1).unwrap();}}>{name}</Button><Text>{move||shown.get().to_string()}</Text></Div>}
}
#[test]
fn setup_mounts_once() {
    let calls = Rc::new(RefCell::new(0));
    let h = Rc::new(RefCell::new(None));
    let (c, out) = (calls.clone(), h.clone());
    let mut rt = Runtime::new(move |cx| {
        *c.borrow_mut() += 1;
        let s = cx.signal(0);
        *out.borrow_mut() = Some(s.clone());
        dynamic(move || s.get().to_string())
    });
    h.borrow().as_ref().unwrap().set(3).unwrap();
    rt.flush();
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(rt.snapshot().text_content(), "3");
}
#[test]
fn keyed_state_and_disposal() {
    let h = Rc::new(RefCell::new(None));
    let out = h.clone();
    let mut rt = Runtime::new(move |cx| {
        let s = cx.signal(vec!["a".to_string(), "b".to_string()]);
        *out.borrow_mut() = Some(s.clone());
        dynamic(move || keyed(s.get(), |n| n.clone(), |name| rustx! {<Item name={name}/>}))
    });
    rt.click_id("a");
    rt.flush();
    let counts = rt.counts();
    h.borrow()
        .as_ref()
        .unwrap()
        .update(|v| v.reverse())
        .unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "b0a1");
    assert_eq!(rt.counts(), counts);
    h.borrow().as_ref().unwrap().set(vec![]).unwrap();
    rt.flush();
    assert!(rt.counts().0 < counts.0);
    h.borrow().as_ref().unwrap().set(vec!["a".into()]).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "a0");
}
#[test]
fn memo_effect_cleanup() {
    let h = Rc::new(RefCell::new(None));
    let out = h.clone();
    let log = Rc::new(RefCell::new(vec![]));
    let events = log.clone();
    let mut rt = Runtime::new(move |cx| {
        let s = cx.signal(1);
        *out.borrow_mut() = Some(s.clone());
        let m = cx.memo(move || s.get() * 2);
        let e = m.clone();
        let log = events.clone();
        cx.effect(move || {
            let v = e.get();
            log.borrow_mut().push(v);
            let log = log.clone();
            Box::new(move || log.borrow_mut().push(-v))
        });
        dynamic(move || m.get().to_string())
    });
    assert_eq!(*log.borrow(), vec![2]);
    h.borrow().as_ref().unwrap().set(2).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "4");
    assert_eq!(*log.borrow(), vec![2, -2, 4]);
    drop(rt);
    assert_eq!(*log.borrow(), vec![2, -2, 4, -4]);
    assert_eq!(h.borrow().as_ref().unwrap().set(9), Err(Disposed));
}
#[test]
fn stale_dependencies_removed() {
    let h = Rc::new(RefCell::new(None));
    let out = h.clone();
    let mut rt = Runtime::new(move |cx| {
        let a = cx.signal(1);
        let b = cx.signal(2);
        let c = cx.signal(true);
        *out.borrow_mut() = Some((a.clone(), b.clone(), c.clone()));
        dynamic(move || {
            if c.get() {
                a.get().to_string()
            } else {
                b.get().to_string()
            }
        })
    });
    let (a, b, c) = h.borrow().clone().unwrap();
    c.set(false).unwrap();
    rt.flush();
    a.set(10).unwrap();
    assert!(!rt.needs_update());
    b.set(20).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "20");
}
#[test]
#[should_panic(expected = "duplicate sibling key")]
fn duplicate_keys() {
    Runtime::new(|_| Node::fragment(vec![Node::text("a").key(1), Node::text("b").key(1)]));
}
#[test]
fn disposed_http_completion_rejected() {
    let h = connected_http();
    let c = h.clone();
    let out = Rc::new(RefCell::new(0));
    let log = out.clone();
    let rt = Runtime::new(move |cx| {
        let log = log.clone();
        c.get(&cx, "/users", move |_| *log.borrow_mut() += 1);
        Node::text("pending")
    });
    assert_eq!(h.pending(), 1);
    h.drain();
    drop(rt);
    assert_eq!(h.pending(), 0);
    assert!(!h.complete(
        HOST_PROTOCOL_VERSION,
        1,
        Ok(HttpResponse {
            status: 200,
            body: vec![]
        })
    ));
    assert_eq!(*out.borrow(), 0);
    assert!(matches!(
        h.drain().first(),
        Some(HostCommand::Cancel { .. })
    ));
}
#[test]
fn cancel_once() {
    let h = connected_http();
    let c = h.clone();
    let calls = Rc::new(RefCell::new(0));
    let log = calls.clone();
    let _rt = Runtime::new(move |cx| {
        let log = log.clone();
        c.get(&cx, "/users", move |r| {
            assert_eq!(r.unwrap_err(), HostError::Cancelled);
            *log.borrow_mut() += 1;
        });
        Node::text("")
    });
    h.cancel(1);
    h.cancel(1);
    assert!(!h.complete(
        HOST_PROTOCOL_VERSION,
        1,
        Ok(HttpResponse {
            status: 200,
            body: vec![]
        })
    ));
    assert_eq!(*calls.borrow(), 1);
}
#[test]
fn context_children_fragments() {
    #[component]
    fn Card(cx: Scope, children: Vec<View>) -> View {
        assert_eq!(cx.context::<u32>(), Some(42));
        Node::fragment(children)
    }
    let rt = Runtime::new(|cx| {
        cx.provide(42u32);
        rustx! {<><Card><Text>{"child"}</Text></Card></>}
    });
    assert_eq!(rt.snapshot().text_content(), "child");
}
#[test]
fn batches() {
    let h = Rc::new(RefCell::new(None));
    let out = h.clone();
    let calls = Rc::new(RefCell::new(0));
    let c = calls.clone();
    let mut rt = Runtime::new(move |cx| {
        let s = cx.signal(0);
        *out.borrow_mut() = Some(s.clone());
        let c = c.clone();
        dynamic(move || {
            *c.borrow_mut() += 1;
            s.get().to_string()
        })
    });
    let before = *calls.borrow();
    for n in 0..100 {
        h.borrow().as_ref().unwrap().set(n).unwrap();
    }
    rt.flush();
    assert_eq!(*calls.borrow(), before + 1);
}
#[test]
fn default_generic_named_slots() {
    #[component]
    fn Panel<T: ToString>(
        value: T,
        #[prop(default = 7)] count: u32,
        #[prop(default)] children: Vec<View>,
        header: View,
    ) -> View {
        Node::fragment(vec![
            header,
            Node::text(format!("{}:{count}", value.to_string())),
            Node::fragment(children),
        ])
    }
    let rt = Runtime::new(|_| rustx! {<Panel value={42u32} header={Node::text("header:")}/>});
    assert_eq!(rt.snapshot().text_content(), "header:42:7");
}
#[test]
fn completed_requests_release_cleanup() {
    let h = Rc::new(RefCell::new(None));
    let out = h.clone();
    let _rt = Runtime::new(move |cx| {
        *out.borrow_mut() = Some(cx);
        Node::text("")
    });
    let s = h.borrow().clone().unwrap();
    let c = connected_http();
    for _ in 0..100 {
        let id = c.get(&s, "/users", |_| {});
        c.drain();
        c.complete(
            HOST_PROTOCOL_VERSION,
            id,
            Ok(HttpResponse {
                status: 200,
                body: vec![],
            }),
        );
    }
    assert_eq!(s.cleanup_count(), 0);
    assert_eq!(c.pending(), 0);
}
#[test]
fn node_refs_clear_on_disposal() {
    let r = NodeRef::default();
    let copy = r.clone();
    let rt = Runtime::new(move |_| rustx! {<Text node_ref={copy.clone()}>{"hello"}</Text>});
    assert!(r.get().is_some());
    drop(rt);
    assert_eq!(r.get(), None);
}
#[test]
#[should_panic(expected = "effect update cycle")]
fn effect_cycles_are_bounded() {
    let mut rt = Runtime::new(|cx| {
        let s = cx.signal(0);
        cx.effect(move || {
            s.set(s.get() + 1).unwrap();
            Box::new(|| {})
        });
        Node::text("")
    });
    for _ in 0..101 {
        rt.flush();
    }
}
mod imported {
    use rustx::prelude::*;
    #[component]
    pub fn Readout(value: Signal<i32>, prefix: String) -> View {
        rustx! {<Text>{move||format!("{prefix}{}",value.get())}</Text>}
    }
}
#[test]
fn imported_reactive_and_initial_props() {
    use imported::{Readout, ReadoutProps};
    let h = Rc::new(RefCell::new(None));
    let out = h.clone();
    let mut rt = Runtime::new(move |cx| {
        let c = cx.signal(0);
        *out.borrow_mut() = Some(c.clone());
        dynamic(
            move || rustx! {<Readout value={c.clone()} prefix={format!("initial {}: ",c.get())}/>},
        )
    });
    let baseline = rt.counts();
    h.borrow().as_ref().unwrap().set(7).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "initial 0: 7");
    assert_eq!(rt.counts(), baseline);
}

fn connected_http() -> HttpClient {
    let h = HttpClient::default();
    h.connect(HostCapabilities::get_cancel()).unwrap();
    h
}

#[test]
fn semantic_text_includes_literal_and_reactive_children() {
    let h = Rc::new(RefCell::new(None));
    let out = h.clone();
    let mut rt = Runtime::new(move |cx| {
        let value = cx.signal("Hello".to_string());
        *out.borrow_mut() = Some(value.clone());
        rustx! {<Text id="readout">{move || value.get()}</Text>}
    });
    fn label(n: &SemanticNode) -> Option<String> {
        if n.app_id.as_deref() == Some("readout") {
            Some(n.label.clone())
        } else {
            n.children.iter().find_map(label)
        }
    }
    assert_eq!(label(&rt.semantics()).as_deref(), Some("Hello"));
    h.borrow().as_ref().unwrap().set("سلام".into()).unwrap();
    rt.flush();
    assert_eq!(label(&rt.semantics()).as_deref(), Some("سلام"));
}
