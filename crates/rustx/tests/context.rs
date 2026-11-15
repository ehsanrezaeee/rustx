use rustx::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn nearest_provider_shadows_without_leaking_to_siblings() {
    let rt = Runtime::new(|cx| {
        cx.provide(1u32);
        Node::fragment(vec![
            Node::component("nested", |cx| {
                assert_eq!(cx.context::<u32>(), Some(1));
                cx.provide(2u32);
                Node::component("leaf", |cx| {
                    Node::text(cx.context::<u32>().unwrap().to_string())
                })
            }),
            Node::component("sibling", |cx| {
                assert_eq!(cx.context::<String>(), None);
                Node::text(cx.context::<u32>().unwrap().to_string())
            }),
        ])
    });
    assert_eq!(rt.snapshot().text_content(), "21");
}

#[test]
fn plain_replacement_is_lookup_only_and_does_not_remount() {
    let scope = Rc::new(RefCell::new(None));
    let out = scope.clone();
    let mounts = Rc::new(Cell::new(0));
    let calls = mounts.clone();
    let mut rt = Runtime::new(move |cx| {
        calls.set(calls.get() + 1);
        cx.provide(1u32);
        *out.borrow_mut() = Some(cx.clone());
        dynamic(move || cx.context::<u32>().unwrap().to_string())
    });
    let scope = scope.borrow().clone().unwrap();
    scope.provide(2u32);
    assert_eq!(scope.context::<u32>(), Some(2));
    assert!(!rt.needs_update());
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "1");
    assert_eq!(mounts.get(), 1);
}

#[test]
fn independent_roots_and_cross_runtime_reads_do_not_subscribe() {
    let scope = Rc::new(RefCell::new(None));
    let out = scope.clone();
    let a = Runtime::new(move |cx| {
        cx.provide(10u32);
        cx.provide(cx.signal(10u32));
        *out.borrow_mut() = Some(cx);
        Node::text("a")
    });
    let a_scope = scope.borrow().clone().unwrap();
    let signal = a_scope.context::<Signal<u32>>().unwrap();
    let foreign = signal.clone();
    let b = Runtime::new(move |cx| {
        assert!(cx.context::<Signal<u32>>().is_none());
        assert!(cx.context::<u32>().is_none());
        cx.provide(20u32);
        let foreign = foreign.clone();
        dynamic(move || format!("{}:{}", cx.context::<u32>().unwrap(), foreign.get()))
    });
    assert_eq!(b.snapshot().text_content(), "20:10");
    signal.set(11).unwrap();
    assert!(!a.needs_update());
    assert!(!b.needs_update());
    drop(a);
    assert!(!a_scope.is_alive());
    assert!(a_scope.context::<Signal<u32>>().is_none());
    assert_eq!(signal.try_get(), Err(Disposed));
}

#[test]
fn provided_signals_track_only_current_dynamic_reads() {
    let handles = Rc::new(RefCell::new(None));
    let out = handles.clone();
    let mut rt = Runtime::new(move |cx| {
        let first = cx.signal(1u32);
        let second = cx.signal(2u32);
        let choose_first = cx.signal(true);
        cx.provide((first.clone(), second.clone()));
        *out.borrow_mut() = Some((first, second, choose_first.clone()));
        dynamic(move || {
            let (a, b) = cx.context::<(Signal<u32>, Signal<u32>)>().unwrap();
            if choose_first.get() { a.get() } else { b.get() }.to_string()
        })
    });
    let (a, b, choose) = handles.borrow().clone().unwrap();
    b.set(20).unwrap();
    assert!(!rt.needs_update());
    a.set(10).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "10");
    choose.set(false).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "20");
    a.set(11).unwrap();
    assert!(!rt.needs_update());
    b.set(21).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "21");
}

#[test]
fn provider_unmount_invalidates_handles_and_remounts_fresh() {
    let toggle = Rc::new(RefCell::new(None));
    let out = toggle.clone();
    let scopes = Rc::new(RefCell::new(Vec::new()));
    let captured = scopes.clone();
    let cleanups = Rc::new(Cell::new(0));
    let cleaned = cleanups.clone();
    let mut rt = Runtime::new(move |cx| {
        let visible = cx.signal(true);
        *out.borrow_mut() = Some(visible.clone());
        let captured = captured.clone();
        let cleaned = cleaned.clone();
        dynamic(move || {
            if !visible.get() {
                return Node::text("");
            }
            let captured = captured.clone();
            let cleaned = cleaned.clone();
            Node::component("provider", move |cx| {
                let value = cx.signal(7u32);
                cx.provide(value.clone());
                captured.borrow_mut().push((cx.clone(), value));
                let cleaned = cleaned.clone();
                cx.on_cleanup(move || cleaned.set(cleaned.get() + 1));
                Node::component("consumer", |cx| {
                    let value = cx.context::<Signal<u32>>().unwrap();
                    dynamic(move || value.get().to_string())
                })
            })
        })
    });
    let baseline = rt.counts();
    let old = scopes.borrow()[0].clone();
    old.1.set(8).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "8");
    toggle.borrow().as_ref().unwrap().set(false).unwrap();
    rt.flush();
    assert_eq!(cleanups.get(), 1);
    assert!(!old.0.is_alive());
    assert!(old.0.context::<Signal<u32>>().is_none());
    assert_eq!(old.1.set(9), Err(Disposed));
    assert_eq!(old.0.cleanup_count(), 0);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| old.0.provide(1u32))).is_err()
    );
    toggle.borrow().as_ref().unwrap().set(true).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "7");
    assert_eq!(rt.counts(), baseline);
    assert_eq!(scopes.borrow().len(), 2);
    drop(rt);
    assert_eq!(cleanups.get(), 2);
    assert_eq!(scopes.borrow()[1].1.try_get(), Err(Disposed));
}

// Clone is application code, and can read the runtime while a context is retrieved.
struct ReadingClone(Signal<u32>);
impl Clone for ReadingClone {
    fn clone(&self) -> Self {
        assert_eq!(self.0.get(), 42);
        Self(self.0.clone())
    }
}
#[test]
fn context_clone_can_read_a_live_signal() {
    let rt = Runtime::new(|cx| {
        cx.provide(ReadingClone(cx.signal(42u32)));
        assert!(cx.context::<ReadingClone>().is_some());
        Node::text("ok")
    });
    assert_eq!(rt.snapshot().text_content(), "ok");
}

struct OnDrop(Option<Box<dyn Fn()>>);
impl Drop for OnDrop {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}
#[test]
fn replacing_context_drops_old_value_outside_runtime_borrow() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let rt = Runtime::new(move |cx| {
        cx.provide(42u32);
        let read = cx.clone();
        let count = count.clone();
        cx.provide(OnDrop(Some(Box::new(move || {
            assert_eq!(read.context::<u32>(), Some(42));
            count.set(count.get() + 1);
        }))));
        cx.provide(OnDrop(None));
        Node::text("ok")
    });
    assert_eq!(calls.get(), 1);
    drop(rt);
    assert_eq!(calls.get(), 1);
}

#[test]
fn disposing_context_drops_after_owner_invalidation_without_borrow() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let rt = Runtime::new(move |cx| {
        let read = cx.clone();
        let count = count.clone();
        cx.provide(OnDrop(Some(Box::new(move || {
            assert!(!read.is_alive());
            assert_eq!(read.context::<u32>(), None);
            count.set(count.get() + 1);
        }))));
        Node::text("ok")
    });
    drop(rt);
    assert_eq!(calls.get(), 1);
}

#[test]
fn replacing_a_provided_signal_does_not_rebind_existing_consumers() {
    let handles = Rc::new(RefCell::new(None));
    let out = handles.clone();
    let mut rt = Runtime::new(move |cx| {
        let first = cx.signal(1u32);
        let second = cx.signal(2u32);
        cx.provide(first.clone());
        *out.borrow_mut() = Some((cx.clone(), first, second));
        Node::component("consumer", |cx| {
            let retained = cx.context::<Signal<u32>>().unwrap();
            dynamic(move || retained.get().to_string())
        })
    });
    let (scope, first, second) = handles.borrow().clone().unwrap();
    scope.provide(second.clone());
    assert_eq!(scope.context::<Signal<u32>>().unwrap().get(), 2);
    second.set(20).unwrap();
    assert!(!rt.needs_update());
    first.set(10).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "10");
}

#[test]
fn context_resources_drop_before_owner_cleanup_and_children_before_parents() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let log = order.clone();
    let rt = Runtime::new(move |cx| {
        let dropped = log.clone();
        cx.provide(OnDrop(Some(Box::new(move || {
            dropped.borrow_mut().push("parent context")
        }))));
        let cleaned = log.clone();
        cx.on_cleanup(move || cleaned.borrow_mut().push("parent cleanup"));
        let log = log.clone();
        Node::component("child", move |cx| {
            let dropped = log.clone();
            cx.provide(OnDrop(Some(Box::new(move || {
                dropped.borrow_mut().push("child context")
            }))));
            let cleaned = log.clone();
            cx.on_cleanup(move || cleaned.borrow_mut().push("child cleanup"));
            Node::text("")
        })
    });
    drop(rt);
    assert_eq!(
        *order.borrow(),
        [
            "child context",
            "child cleanup",
            "parent context",
            "parent cleanup"
        ]
    );
}
