//! Shared deterministic interaction; actual OS/browser/AT acceptance is separate.
use rustx_core::*;
use rustx_engine::{vello::kurbo::Point, Engine};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn ordinary_text_supports_keyboard_selection_copy_reflow_and_route_disposal() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut engine = Engine::new(move |cx| {
        let shown = cx.signal(true);
        *out.borrow_mut() = Some(shown.clone());
        dynamic(move || {
            if shown.get() {
                Node::text("A👩‍👩‍👧‍👦e\u{301} سلام")
                    .id("copyable")
                    .selectable(true)
            } else {
                Node::text("New route")
            }
        })
    });
    engine.frame(500., 200., 1.);
    let id = engine.runtime.snapshot().find("copyable").unwrap().id;
    engine.focus_node(id);
    assert!(engine.key("a", false, true, false, false));
    let editor = engine.runtime.editor(id).unwrap();
    assert_eq!(editor.copy().unwrap(), "A👩‍👩‍👧‍👦e\u{301} سلام");
    let selection = editor.selection();
    assert!(!engine.insert_text("overwrite"));
    assert_eq!(editor.value(), "A👩‍👩‍👧‍👦e\u{301} سلام");
    engine.frame(180., 400., 1.);
    assert_eq!(editor.selection(), selection);
    assert_eq!(engine.focused_node(), Some(id));
    saved.borrow().as_ref().unwrap().set(false).unwrap();
    engine.frame(180., 400., 1.);
    assert!(engine.runtime.editor(id).is_none());
    assert_eq!(engine.focused_node(), None);
}

#[test]
fn ordinary_text_pointer_drag_uses_shaped_graphemes_capture_and_cancellable_defaults() {
    let mut engine = Engine::new(|_| {
        Node::widget(Widget::Column).children(vec![
            Node::text("A🌍B سلام")
                .id("copyable")
                .selectable(true)
                .style(Style::column().width(400.).font_size(24.)),
            Node::text("Cancelled")
                .id("cancelled")
                .selectable(true)
                .on_pointer_down(|event| event.prevent_default()),
        ])
    });
    engine.frame(600., 300., 1.);
    let rect = engine
        .boxes
        .as_ref()
        .unwrap()
        .find("copyable")
        .unwrap()
        .rect;
    let shaped = engine.text.shape("A🌍B سلام", 24., rect.width() as f32);
    let emoji = shaped
        .clusters
        .iter()
        .find(|cluster| cluster.start == 1)
        .unwrap();
    let y = rect.y0 + f64::from(emoji.y + emoji.height * 0.5);
    let start = Point::new(rect.x0 + f64::from(emoji.x), y);
    let end = Point::new(rect.x0 + f64::from(emoji.x + emoji.width), y);
    engine.pointer_down(start);
    let id = engine.runtime.snapshot().find("copyable").unwrap().id;
    assert_eq!(engine.runtime.captured_pointer(0), Some(id));
    engine.pointer_move(end);
    engine.pointer_up(end);
    assert_eq!(engine.runtime.captured_pointer(0), None);
    assert_eq!(engine.runtime.editor(id).unwrap().copy().unwrap(), "🌍");
    engine.frame(600., 300., 1.);
    let cancelled = engine.boxes.as_ref().unwrap().find("cancelled").unwrap();
    let cancelled_id = cancelled.node.id;
    let cancelled_rect = cancelled.rect;
    engine.pointer_down(Point::new(cancelled_rect.x0 + 10., cancelled_rect.y0 + 8.));
    assert_eq!(engine.runtime.captured_pointer(0), None);
    assert_eq!(engine.focused_node(), Some(id));
    assert!(engine
        .runtime
        .editor(cancelled_id)
        .unwrap()
        .selection()
        .is_collapsed());
    engine.cancel();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ordinary_text_native_projection_accepts_selection_and_rejects_replacement() {
    use accesskit::{
        Action, ActionData, ActionRequest, NodeId as A11yId, TextPosition, TextSelection,
    };
    let mut engine = Engine::new(|_| Node::text("A🌍B").id("copyable").selectable(true));
    engine.frame(400., 200., 1.);
    let id = engine.runtime.snapshot().find("copyable").unwrap().id;
    let update = rustx_engine::accessibility::project(&engine, 1.);
    let projected = update.nodes.iter().find(|(node, _)| node.0 == id).unwrap();
    assert!(projected.1.supports_action(Action::SetTextSelection));
    assert!(!projected.1.supports_action(Action::ReplaceSelectedText));
    let line = projected.1.text_selection().unwrap().anchor.node;
    let request = ActionRequest {
        action: Action::SetTextSelection,
        target_tree: accesskit::TreeId::ROOT,
        target_node: A11yId(id),
        data: Some(ActionData::SetTextSelection(TextSelection {
            anchor: TextPosition {
                node: line,
                character_index: 1,
            },
            focus: TextPosition {
                node: line,
                character_index: 2,
            },
        })),
    };
    assert!(rustx_engine::accessibility::dispatch_action(
        &mut engine,
        request
    ));
    assert_eq!(engine.runtime.editor(id).unwrap().copy().unwrap(), "🌍");
    assert!(!rustx_engine::accessibility::dispatch_action(
        &mut engine,
        ActionRequest {
            action: Action::ReplaceSelectedText,
            target_tree: accesskit::TreeId::ROOT,
            target_node: A11yId(id),
            data: Some(ActionData::Value("overwrite".into())),
        }
    ));
}

#[test]
fn shared_editor_word_and_vertical_line_keys_preserve_unicode_and_preferred_column() {
    let mut engine = Engine::new(|_| {
        Node::widget(Widget::TextArea)
            .id("editor")
            .style(Style::column().width(450.).height(220.).font_size(20.))
            .value("abcd\nx\nabcd\n\nسلام e\u{301}")
    });
    engine.frame(500., 300., 1.);
    let id = engine.runtime.snapshot().find("editor").unwrap().id;
    engine.focus_node(id);
    let editor = engine.runtime.editor(id).unwrap();
    editor.select(3, 3).unwrap();
    assert!(engine.key("ArrowDown", false, false, false, false));
    assert_eq!(editor.selection(), Selection::caret(6));
    assert!(engine.key("ArrowDown", false, false, false, false));
    assert_eq!(editor.selection(), Selection::caret(10));
    assert!(engine.key("ArrowDown", true, false, false, false));
    assert_eq!(editor.selection().anchor, 10);
    assert_eq!(editor.selection().focus, 12);
    assert_eq!(editor.copy().unwrap(), "d\n");
    assert!(engine.key("ArrowUp", false, false, false, false));
    assert_eq!(editor.selection(), Selection::caret(10));
    assert!(engine.key("End", false, true, false, false));
    assert_eq!(editor.selection(), Selection::caret(editor.value().len()));
    assert!(engine.key("ArrowLeft", true, true, false, false));
    assert_eq!(editor.copy().unwrap(), "e\u{301}");
    assert!(engine.key("Home", false, true, false, false));
    assert_eq!(editor.selection(), Selection::caret(0));
    assert!(engine.key("ArrowRight", false, true, false, false));
    assert_eq!(editor.selection(), Selection::caret(5));
}

#[test]
fn modal_traps_keyboard_blocks_background_and_restores_focus() {
    let state = Rc::new(RefCell::new(None));
    let out = state.clone();
    let hit = Rc::new(Cell::new(0));
    let hits = hit.clone();
    let mut engine = Engine::new(move |cx| {
        let modal = OverlayState::new(&cx);
        *out.borrow_mut() = Some(modal.clone());
        let hits = hits.clone();
        Node::widget(Widget::Column).children(vec![
            Node::widget(Widget::Button)
                .id("outside")
                .value("Open")
                .on_click(move |_| hits.set(hits.get() + 1)),
            dialog(
                modal,
                "Dialog",
                Node::widget(Widget::TextInput).id("inside").value("edit"),
            ),
        ])
    });
    engine.frame(600., 500., 1.);
    let outside = engine.runtime.snapshot().find("outside").unwrap().id;
    engine.focus_node(outside);
    state.borrow().as_ref().unwrap().show();
    engine.frame(600., 500., 1.);
    let inside = engine.runtime.snapshot().find("inside").unwrap().id;
    assert_eq!(engine.focused_node(), Some(inside));
    engine.focus_node(outside);
    assert_eq!(engine.focused_node(), Some(inside));
    assert!(!engine.runtime.click(outside, 0., 0.));
    assert_eq!(hit.get(), 0);
    engine.tab(false);
    assert_ne!(engine.focused_node(), Some(outside));
    engine.tab(false);
    assert_eq!(engine.focused_node(), Some(inside));
    assert!(engine.key("Escape", false, false, false, false));
    engine.frame(600., 500., 1.);
    assert_eq!(engine.focused_node(), Some(outside));
}

#[test]
fn editor_input_selection_and_undo_invalidate_retained_boxes() {
    let mut engine = Engine::new(|_| Node::widget(Widget::TextInput).id("field").value("A🌍سلام"));
    engine.frame(500., 200., 1.);
    let id = engine.runtime.snapshot().find("field").unwrap().id;
    engine.focus_node(id);
    engine.runtime.editor(id).unwrap().select_all();
    engine.runtime.invalidate_model();
    engine.insert_text("changed");
    assert!(engine.frame(500., 200., 1.));
    assert_eq!(
        engine
            .boxes
            .as_ref()
            .unwrap()
            .find("field")
            .unwrap()
            .node
            .text,
        "changed"
    );
    assert!(engine.key("z", false, true, false, false));
    engine.frame(500., 200., 1.);
    assert_eq!(
        engine
            .boxes
            .as_ref()
            .unwrap()
            .find("field")
            .unwrap()
            .node
            .text,
        "A🌍سلام"
    );
    assert!(engine.key("End", false, false, false, false));
    assert!(engine.key("ArrowLeft", true, false, false, false));
    engine.frame(500., 200., 1.);
    let selection = engine
        .boxes
        .as_ref()
        .unwrap()
        .find("field")
        .unwrap()
        .node
        .editing
        .as_ref()
        .unwrap()
        .selection;
    assert_ne!(selection.anchor, selection.focus);
}

#[test]
fn tab_roving_focus_skips_disabled_and_activates_selected_panel() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut engine = Engine::new(move |cx| {
        let active = cx.signal(0);
        *out.borrow_mut() = Some(active.clone());
        tabs(
            active,
            vec![
                TabPanel {
                    id: "one".into(),
                    label: "One".into(),
                    content: Node::text("first"),
                    disabled: false,
                },
                TabPanel {
                    id: "two".into(),
                    label: "Two".into(),
                    content: Node::text("disabled"),
                    disabled: true,
                },
                TabPanel {
                    id: "three".into(),
                    label: "Three".into(),
                    content: Node::text("third"),
                    disabled: false,
                },
            ],
        )
    });
    engine.frame(500., 300., 1.);
    let id = engine.runtime.snapshot().find("one-tab").unwrap().id;
    engine.focus_node(id);
    assert!(engine.key("ArrowRight", false, false, false, false));
    engine.frame(500., 300., 1.);
    assert_eq!(saved.borrow().as_ref().unwrap().get(), 2);
    assert!(engine.runtime.snapshot().find("three").is_some());
    assert_eq!(
        engine.focused_node(),
        Some(engine.runtime.snapshot().find("three-tab").unwrap().id)
    );
}

#[test]
fn slider_pointer_drag_obeys_range_and_keyboard_default_can_be_cancelled() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut engine = Engine::new(move |cx| {
        let value = cx.signal("0".to_string());
        *out.borrow_mut() = Some(value.clone());
        dynamic(move || {
            let update = value.clone();
            Node::widget(Widget::Slider)
                .id("slider")
                .style(Style::column().width(200.).height(40.))
                .value(value.get())
                .min(0.)
                .max(10.)
                .step(2.)
                .on_input(move |e| {
                    let _ = update.set(e.value);
                })
                .on_key_down(|e| {
                    if e.data.key == "Home" {
                        e.prevent_default();
                    }
                })
        })
    });
    engine.frame(400., 200., 1.);
    let rect = engine.boxes.as_ref().unwrap().find("slider").unwrap().rect;
    engine.pointer_down(Point::new(rect.x0 + 12., rect.y0 + 10.));
    engine.pointer_move(Point::new(rect.x1 - 12., rect.y0 + 10.));
    engine.pointer_up(Point::new(rect.x1 - 12., rect.y0 + 10.));
    engine.frame(400., 200., 1.);
    assert_eq!(saved.borrow().as_ref().unwrap().get(), "10");
    engine.key("Home", false, false, false, false);
    assert_eq!(saved.borrow().as_ref().unwrap().get(), "10");
    engine.key("ArrowLeft", false, false, false, false);
    assert_eq!(saved.borrow().as_ref().unwrap().get(), "8");
}

#[test]
fn event_local_coordinates_follow_each_committed_translated_box() {
    let captured = Rc::new(RefCell::new(vec![]));
    let parent = captured.clone();
    let child = captured.clone();
    let mut engine = Engine::new(move |_| {
        let parent = parent.clone();
        let child = child.clone();
        Node::widget(Widget::Column)
            .id("parent")
            .style(Style::column().padding(20.).translate(10., 15.))
            .on_event_capture(EventKind::PointerDown, move |e| {
                parent
                    .borrow_mut()
                    .push((e.current_target, e.local_x, e.local_y))
            })
            .children(vec![Node::widget(Widget::Button)
                .id("child")
                .value("Hit")
                .style(Style::column().width(100.).height(40.))
                .on_pointer_down(move |e| {
                    child
                        .borrow_mut()
                        .push((e.current_target, e.local_x, e.local_y))
                })])
    });
    engine.frame(500., 300., 1.);
    let parent = engine.boxes.as_ref().unwrap().find("parent").unwrap().rect;
    let child = engine.boxes.as_ref().unwrap().find("child").unwrap();
    let id = child.node.id;
    let rect = child.rect;
    engine.dispatch(
        id,
        EventData::pointer(EventKind::PointerDown, 3, rect.x0 + 7., rect.y0 + 9.),
    );
    let result = captured.borrow();
    assert_eq!(result.len(), 2);
    assert_eq!(
        (result[0].1, result[0].2),
        (rect.x0 + 7. - parent.x0, rect.y0 + 9. - parent.y0)
    );
    assert_eq!((result[1].1, result[1].2), (7., 9.));
}

#[test]
fn preventing_pointerdown_focus_keeps_native_popup_option_click_usable() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut engine = Engine::new(move |cx| {
        let value = cx.signal("a".to_string());
        *out.borrow_mut() = Some(value.clone());
        select_with_id(
            "choose",
            value,
            vec![Choice::new("a", "Alpha"), Choice::new("b", "Beta")],
            "Choice",
        )
    });
    engine.frame(500., 300., 1.);
    let control = engine.runtime.snapshot().find("choose").unwrap().id;
    engine.focus_node(control);
    engine.activate();
    engine.frame(500., 300., 1.);
    fn option(b: &rustx_engine::layout::BoxNode) -> Option<&rustx_engine::layout::BoxNode> {
        if b.node.role == Some("option") && b.node.text == "Beta" {
            Some(b)
        } else {
            b.children.iter().find_map(option)
        }
    }
    let rect = option(engine.boxes.as_ref().unwrap()).unwrap().rect;
    let p = Point::new(rect.x0 + 5., rect.y0 + 5.);
    engine.pointer_down(p);
    assert_eq!(engine.focused_node(), Some(control));
    engine.pointer_up(p);
    engine.frame(500., 300., 1.);
    assert_eq!(saved.borrow().as_ref().unwrap().get(), "b");
    assert!(engine.runtime.snapshot().find("choose-options").is_none());
}

#[test]
fn engine_dispatch_blocks_modal_background_and_releases_earlier_capture() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let hits = Rc::new(Cell::new(0));
    let clicked = hits.clone();
    let mut engine = Engine::new(move |cx| {
        let modal = OverlayState::new(&cx);
        *out.borrow_mut() = Some(modal.clone());
        let clicked = clicked.clone();
        Node::widget(Widget::Column).children(vec![
            Node::widget(Widget::Button)
                .id("background")
                .value("Background")
                .on_pointer_down(|event| event.capture_pointer())
                .on_click(move |_| clicked.set(clicked.get() + 1)),
            dialog(modal, "Modal", Node::text("Modal content")),
        ])
    });
    engine.frame(500., 300., 1.);
    let background = engine.runtime.snapshot().find("background").unwrap().id;
    engine.dispatch(
        background,
        EventData::pointer(EventKind::PointerDown, 9, 0., 0.),
    );
    assert_eq!(engine.runtime.captured_pointer(9), Some(background));
    saved.borrow().as_ref().unwrap().show();
    engine.frame(500., 300., 1.);
    assert!(
        engine
            .dispatch(background, EventData::new(EventKind::Click))
            .default_prevented
    );
    assert_eq!(hits.get(), 0);
    assert!(
        engine
            .dispatch(
                background,
                EventData::pointer(EventKind::PointerMove, 9, 1., 1.)
            )
            .default_prevented
    );
    let released = engine.dispatch(
        background,
        EventData::pointer(EventKind::PointerUp, 9, 1., 1.),
    );
    assert!(released.default_prevented && released.pointer_released);
    assert_eq!(engine.runtime.captured_pointer(9), None);
}
