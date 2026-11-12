use rustx::*;
use std::{cell::RefCell, rc::Rc};

#[test]
fn spinner_uses_scoped_timers_and_unmount_cancels_the_pending_tick() {
    let services = ServiceClient::default();
    let tasks = TaskQueue::default();
    let saved = Rc::new(RefCell::new(None));
    let host = services.clone();
    let scheduler = tasks.clone();
    let out = saved.clone();
    let mut runtime = Runtime::new(move |cx| {
        cx.provide(host.clone());
        cx.provide(scheduler.clone());
        let visible = cx.signal(true);
        *out.borrow_mut() = Some(visible.clone());
        dynamic(move || {
            if visible.get() {
                spinner("Loading")
            } else {
                Node::text("settled")
            }
        })
    });
    runtime.flush();
    tasks.poll();
    let initial = runtime.snapshot().text_content();
    let command = services.drain().into_iter().next().unwrap();
    assert!(matches!(
        command.operation,
        ServiceOperation::Timer { delay_ms: 150 }
    ));
    assert!(services.complete(SERVICE_VERSION, command.id, Ok(ServiceValue::Unit)));
    tasks.poll();
    runtime.flush();
    assert_ne!(runtime.snapshot().text_content(), initial);
    let tick = services.drain().into_iter().next().unwrap();
    saved.borrow().as_ref().unwrap().set(false).unwrap();
    runtime.flush();
    assert_eq!(tasks.pending(), 0);
    assert_eq!(services.pending(), 0);
    assert!(!services.complete(SERVICE_VERSION, tick.id, Ok(ServiceValue::Unit)));
    assert!(services
        .drain()
        .iter()
        .any(|c| matches!(c.operation,ServiceOperation::Cancel{resource}if resource==tick.id)));
}
