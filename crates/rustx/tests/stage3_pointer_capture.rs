use rustx::*;
use std::{cell::RefCell, rc::Rc};

#[test]
fn terminal_pointer_events_release_capture_when_target_becomes_ineligible() {
    for terminal in [EventKind::PointerUp, EventKind::PointerCancel] {
        for change in ["disabled", "hidden", "modal"] {
            let saved = Rc::new(RefCell::new(None));
            let out = saved.clone();
            let mut runtime = Runtime::new(move |cx| {
                let changed = cx.signal(false);
                *out.borrow_mut() = Some(changed.clone());
                dynamic(move || {
                    Node::widget(Widget::Column).children(vec![
                        Node::widget(Widget::Button)
                            .id("drag")
                            .control(ControlState {
                                disabled: change == "disabled" && changed.get(),
                                hidden: change == "hidden" && changed.get(),
                                ..Default::default()
                            })
                            .on_pointer_down(|e| e.capture_pointer()),
                        if change == "modal" && changed.get() {
                            Node::widget(Widget::Dialog).label("Modal")
                        } else {
                            Node::fragment(vec![])
                        },
                    ])
                })
            });
            let target = runtime.snapshot().find("drag").unwrap().id;
            runtime.dispatch(
                target,
                EventData::pointer(EventKind::PointerDown, 7, 0., 0.),
            );
            assert_eq!(runtime.captured_pointer(7), Some(target));
            saved.borrow().as_ref().unwrap().set(true).unwrap();
            runtime.flush();
            let result = runtime.dispatch(0, EventData::pointer(terminal, 7, 0., 0.));
            assert!(result.pointer_released, "{terminal:?}: {change}");
            assert_eq!(runtime.captured_pointer(7), None, "{terminal:?}: {change}");
        }
    }
}
