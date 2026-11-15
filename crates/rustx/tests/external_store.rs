use rustx::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn reader(store: ExternalStore<(u32, u32)>) -> Runtime {
    Runtime::new(move |cx| {
        let sub = store.subscribe(&cx).unwrap();
        dynamic(move || {
            let s = sub.get();
            format!("{}:{}:{}", s.revision, s.value.0, s.value.1)
        })
    })
}
#[test]
fn whole_snapshots_deduplicate_and_notify_independent_roots() {
    let store = ExternalStore::new((0, 0));
    let mut a = reader(store.clone());
    let mut b = reader(store.clone());
    let old = store.snapshot();
    assert_eq!(store.publish((0, 0)), Ok(false));
    assert!(!a.needs_update());
    assert!(!b.needs_update());
    for n in 1..=100 {
        assert_eq!(store.publish((n, n * 2)), Ok(true));
    }
    a.flush();
    b.flush();
    assert_eq!(a.snapshot().text_content(), "100:100:200");
    assert_eq!(a.snapshot().text_content(), b.snapshot().text_content());
    assert_eq!(*old.value, (0, 0));
    assert_eq!(old.revision, 0);
    drop(a);
    assert_eq!(store.subscriber_count(), 1);
    drop(b);
    assert_eq!(store.subscriber_count(), 0);
}
#[test]
fn all_subscribers_update_before_one_wake_per_runtime() {
    let store = ExternalStore::new(0u32);
    let handles = Rc::new(RefCell::new(Vec::new()));
    let out = handles.clone();
    let source = store.clone();
    let mut rt = Runtime::new(move |cx| {
        for _ in 0..3 {
            out.borrow_mut().push(source.subscribe(&cx).unwrap());
        }
        Node::text("")
    });
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let source = store.clone();
    rt.set_waker(move || {
        count.set(count.get() + 1);
        for sub in handles.borrow().iter() {
            assert_eq!(sub.get().revision, 1);
            assert_eq!(*sub.get().value, 9);
        }
        assert_eq!(source.publish(10), Err(ExternalStoreError::Busy));
    });
    store.publish(9).unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(*store.snapshot().value, 9);
}
#[test]
fn publish_during_mount_or_effect_is_rejected_and_can_retry() {
    let store = ExternalStore::new((1, 2));
    let source = store.clone();
    let mut rt = Runtime::new(move |cx| {
        let sub = source.subscribe(&cx).unwrap();
        assert_eq!(source.publish((2, 4)), Err(ExternalStoreError::Busy));
        let source = source.clone();
        cx.effect(move || {
            assert_eq!(source.publish((3, 6)), Err(ExternalStoreError::Busy));
            Box::new(|| {})
        });
        dynamic(move || {
            let s = sub.get();
            format!("{}:{}", s.value.0, s.value.1)
        })
    });
    assert_eq!(rt.snapshot().text_content(), "1:2");
    store.publish((4, 8)).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "4:8");
}
#[test]
fn view_cannot_publish_a_torn_commit() {
    let store = ExternalStore::new((1, 2));
    let source = store.clone();
    let rt = Runtime::new(move |cx| {
        let left = source.subscribe(&cx).unwrap();
        let right = source.subscribe(&cx).unwrap();
        let source = source.clone();
        Node::fragment(vec![
            dynamic(move || {
                let s = left.get();
                assert_eq!(source.publish((9, 18)), Err(ExternalStoreError::Busy));
                s.value.0.to_string()
            }),
            dynamic(move || right.get().value.1.to_string()),
        ])
    });
    assert_eq!(rt.snapshot().text_content(), "12");
}
#[test]
fn replacement_is_atomic_and_does_not_allocate_more_owner_resources() {
    let a = ExternalStore::new(1u32);
    let b = ExternalStore::new(2u32);
    let handle = Rc::new(RefCell::new(None));
    let out = handle.clone();
    let source = a.clone();
    let mut rt = Runtime::new(move |cx| {
        let s = source.subscribe(&cx).unwrap();
        *out.borrow_mut() = Some((cx, s.clone()));
        dynamic(move || s.get().value.to_string())
    });
    let (scope, sub) = handle.borrow().clone().unwrap();
    let counts = rt.counts();
    for _ in 0..100 {
        sub.replace_source(&b).unwrap();
        sub.replace_source(&a).unwrap();
    }
    assert_eq!(rt.counts(), counts);
    assert_eq!(scope.cleanup_count(), 1);
    sub.replace_source(&b).unwrap();
    rt.flush();
    assert_eq!(a.subscriber_count(), 0);
    assert_eq!(b.subscriber_count(), 1);
    assert_eq!(rt.snapshot().text_content(), "2");
    a.publish(7).unwrap();
    assert!(!rt.needs_update());
    b.publish(8).unwrap();
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "8");
    sub.disconnect();
    sub.disconnect();
    assert_eq!(scope.cleanup_count(), 0);
    assert!(sub.try_get().is_err());
    assert_eq!(b.subscriber_count(), 0);
    assert_eq!(sub.replace_source(&a), Err(ExternalStoreError::Disposed));
}
#[test]
fn capacity_failure_keeps_previous_source_and_disposal_frees_slots() {
    let full = ExternalStore::new(1u32);
    let source = full.clone();
    let rt = Runtime::new(move |cx| {
        for _ in 0..MAX_STORE_SUBSCRIPTIONS {
            source.subscribe(&cx).unwrap();
        }
        assert!(matches!(
            source.subscribe(&cx),
            Err(ExternalStoreError::Capacity)
        ));
        Node::text("")
    });
    let old = ExternalStore::new(2u32);
    let source = old.clone();
    let handle = Rc::new(RefCell::new(None));
    let out = handle.clone();
    let other = Runtime::new(move |cx| {
        let s = source.subscribe(&cx).unwrap();
        *out.borrow_mut() = Some(s);
        Node::text("")
    });
    let sub = handle.borrow().clone().unwrap();
    assert_eq!(sub.replace_source(&full), Err(ExternalStoreError::Capacity));
    assert_eq!(*sub.get().value, 2);
    assert_eq!(old.subscriber_count(), 1);
    drop(rt);
    assert_eq!(full.subscriber_count(), 0);
    sub.replace_source(&full).unwrap();
    assert_eq!(*sub.get().value, 1);
    assert_eq!(old.subscriber_count(), 0);
    drop(other);
    assert_eq!(full.subscriber_count(), 0);
}
#[test]
fn disposal_invalidates_retained_handles_and_unsubscribes_exactly_once() {
    let store = ExternalStore::new(0u32);
    let source = store.clone();
    let handle = Rc::new(RefCell::new(None));
    let out = handle.clone();
    let rt = Runtime::new(move |cx| {
        let s = source.subscribe(&cx).unwrap();
        *out.borrow_mut() = Some((cx, s));
        Node::text("")
    });
    let (scope, sub) = handle.borrow().clone().unwrap();
    drop(rt);
    assert_eq!(store.subscriber_count(), 0);
    assert!(sub.try_get().is_err());
    sub.disconnect();
    assert!(matches!(
        store.subscribe(&scope),
        Err(ExternalStoreError::Disposed)
    ));
    assert_eq!(store.publish(1), Ok(true));
}
#[test]
fn render_panic_does_not_leave_commit_flag_stuck() {
    let store = ExternalStore::new(0u32);
    let source = store.clone();
    let mut rt = Runtime::new(move |cx| {
        let s = source.subscribe(&cx).unwrap();
        dynamic(move || {
            let value = *s.get().value;
            assert_ne!(value, 1, "intentional test panic");
            value.to_string()
        })
    });
    store.publish(1).unwrap();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rt.flush())).is_err());
    assert_eq!(store.publish(2), Ok(true));
    rt.flush();
    assert_eq!(rt.snapshot().text_content(), "2");
}

#[test]
fn synchronous_wake_can_dispose_another_root_without_a_stale_callback() {
    let store = ExternalStore::new((0, 0));
    let a = Rc::new(RefCell::new(Some(reader(store.clone()))));
    let b = Rc::new(RefCell::new(Some(reader(store.clone()))));
    let calls = Rc::new(Cell::new(0));
    for (root, other) in [(&a, &b), (&b, &a)] {
        let other = Rc::downgrade(other);
        let count = calls.clone();
        root.borrow_mut().as_mut().unwrap().set_waker(move || {
            count.set(count.get() + 1);
            other.upgrade().unwrap().borrow_mut().take();
        });
    }
    store.publish((1, 2)).unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(store.subscriber_count(), 1);
}

#[test]
fn synchronous_wake_can_mount_a_reader_of_the_new_snapshot() {
    let store = ExternalStore::new((0, 0));
    let mut rt = reader(store.clone());
    let mounted = Rc::new(RefCell::new(None));
    let out = mounted.clone();
    let source = store.clone();
    rt.set_waker(move || {
        *out.borrow_mut() = Some(reader(source.clone()));
    });
    store.publish((3, 6)).unwrap();
    assert_eq!(
        mounted.borrow().as_ref().unwrap().snapshot().text_content(),
        "1:3:6"
    );
    assert_eq!(store.subscriber_count(), 2);
}
