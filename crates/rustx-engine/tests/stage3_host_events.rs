//! Deterministic host normalization, metadata admission and committed scrolling.
use rustx_core::*;
use rustx_engine::{vello::kurbo::Point, Engine};
use std::{cell::RefCell, rc::Rc};

#[test]
fn drop_metadata_is_bounded_redacted_and_cancellable_over_the_committed_path() {
    let log = Rc::new(RefCell::new(vec![]));
    let out = log.clone();
    let mut engine = Engine::new(move |_| {
        let capture = out.clone();
        let target = out.clone();
        let bubble = out.clone();
        Node::widget(Widget::Column)
            .on_event_capture(EventKind::FileDrop, move |e| {
                capture.borrow_mut().push("capture");
                assert_eq!(e.data.files[0].size, Some(42));
            })
            .on_file_drop(move |_| bubble.borrow_mut().push("bubble"))
            .children(vec![Node::widget(Widget::Button)
                .id("drop")
                .value("Drop metadata")
                .on_file_drop(move |e| {
                    target.borrow_mut().push("target");
                    e.prevent_default();
                })])
    });
    engine.frame(500., 200., 1.);
    let rect = engine.boxes.as_ref().unwrap().find("drop").unwrap().rect;
    let mut event = EventData::new(EventKind::FileDrop);
    event.x = rect.center().x;
    event.y = rect.center().y;
    event.files = vec![DroppedFileMetadata {
        name: "private-name.json".into(),
        mime: Some("application/json".into()),
        size: Some(42),
    }];
    assert!(event.is_valid());
    assert!(!format!("{event:?} {:?}", event.files).contains("private-name"));
    assert!(engine.host_event(event.clone()).default_prevented);
    assert_eq!(&*log.borrow(), &["capture", "target", "bubble"]);
    for files in [
        vec![event.files[0].clone(); 17],
        vec![DroppedFileMetadata {
            name: "/private/file".into(),
            ..event.files[0].clone()
        }],
        vec![DroppedFileMetadata {
            name: "x".repeat(1025),
            ..event.files[0].clone()
        }],
        vec![DroppedFileMetadata {
            size: Some(MAX_HOST_ID + 1),
            ..event.files[0].clone()
        }],
    ] {
        let mut invalid = event.clone();
        invalid.files = files;
        assert!(!invalid.is_valid());
        assert!(engine.host_event(invalid).default_prevented);
    }
    assert_eq!(
        log.borrow().len(),
        3,
        "invalid metadata must not reach application callbacks"
    );
}

#[test]
fn resize_and_resource_notifications_target_root_and_preserve_typed_payloads() {
    let log = Rc::new(RefCell::new(vec![]));
    let out = log.clone();
    let mut engine = Engine::new(move |_| {
        let resize = out.clone();
        let error = out.clone();
        Node::widget(Widget::Column)
            .on_resize(move |e| resize.borrow_mut().push(e.data))
            .on_resource_error(move |e| error.borrow_mut().push(e.data))
            .children(vec![Node::widget(Widget::Button)
                .id("focus")
                .value("Focus")])
    });
    engine.frame(500., 200., 1.);
    let id = engine.runtime.snapshot().find("focus").unwrap().id;
    engine.focus_node(id);
    let mut resize = EventData::new(EventKind::Resize);
    resize.width = 800.;
    resize.height = 600.;
    resize.scale = 2.;
    engine.host_event(resize);
    let mut error = EventData::new(EventKind::ResourceError);
    error.resource_error = Some(ResourceFailure {
        source: ResourceSource::Http,
        code: "timeout".into(),
        status: None,
    });
    engine.host_event(error);
    assert_eq!(log.borrow().len(), 2);
    assert_eq!(log.borrow()[0].width, 800.);
    assert_eq!(log.borrow()[0].scale, 2.);
    assert_eq!(
        log.borrow()[1].resource_error.as_ref().unwrap().code,
        "timeout"
    );
}

#[test]
fn scroll_events_report_actual_committed_offsets_once_and_ignore_cancelled_wheels_and_boundaries() {
    let state = Rc::new(RefCell::new(None));
    let saved = state.clone();
    let log = Rc::new(RefCell::new(vec![]));
    let out = log.clone();
    let mut engine = Engine::new(move |cx| {
        let state = ScrollState::new(&cx, 60., 500.);
        *saved.borrow_mut() = Some(state.clone());
        let out = out.clone();
        Node::widget(Widget::Column)
            .on_event(EventKind::Scroll, move |e| {
                out.borrow_mut().push(e.data.delta_y)
            })
            .on_event_capture(EventKind::Wheel, |e| e.prevent_default())
            .children(vec![scroll_view(
                state,
                Node::text("Scrollable").style(Style::column().height(500.)),
            )])
    });
    engine.frame(500., 200., 1.);
    let state = state.borrow().clone().unwrap();
    assert!(log.borrow().is_empty());
    state.scroll_by(25.);
    assert!(engine.frame(500., 200., 1.));
    assert_eq!(&*log.borrow(), &[25.]);
    assert!(!engine.frame(500., 200., 1.));
    assert_eq!(log.borrow().len(), 1);
    engine.scroll_at(Point::new(10., 10.), 30.);
    engine.frame(500., 200., 1.);
    assert_eq!(state.offset.get(), 25.);
    assert_eq!(log.borrow().len(), 1);
    state.scroll_to(500.);
    engine.frame(500., 200., 1.);
    assert_eq!(&*log.borrow(), &[25., 415.]);
    assert!(!state.scroll_by(100.));
    engine.frame(500., 200., 1.);
    assert_eq!(log.borrow().len(), 2);
}
