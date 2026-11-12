//! Deterministic shared widget/event contracts; no browser or AT certification.
use rustx::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn committed_event_path_has_capture_target_bubble_and_cancelled_default() {
    let log = Rc::new(RefCell::new(vec![]));
    let clicked = Rc::new(Cell::new(false));
    let a = log.clone();
    let b = log.clone();
    let c = log.clone();
    let hit = clicked.clone();
    let mut rt = Runtime::new(move |_| {
        let a = a.clone();
        let b = b.clone();
        let c = c.clone();
        let hit = hit.clone();
        Node::new(Kind::Div)
            .on_event_capture(EventKind::Click, move |e| {
                assert_eq!(e.phase, EventPhase::Capture);
                a.borrow_mut().push("capture");
            })
            .on_event(EventKind::Click, move |e| {
                assert_eq!(e.phase, EventPhase::Bubble);
                c.borrow_mut().push("bubble");
            })
            .children(vec![Node::new(Kind::Button)
                .id("target")
                .on_event(EventKind::Click, move |e| {
                    assert_eq!(e.phase, EventPhase::Target);
                    b.borrow_mut().push("target");
                    e.prevent_default();
                })
                .on_click(move |_| hit.set(true))])
    });
    let id = rt.snapshot().find("target").unwrap().id;
    let result = rt.dispatch(id, EventData::new(EventKind::Click));
    assert_eq!(*log.borrow(), ["capture", "target", "bubble"]);
    assert!(result.default_prevented);
    assert!(!clicked.get());
}

#[test]
fn immediate_stop_keeps_later_handlers_and_ancestors_from_running() {
    let hits = Rc::new(Cell::new(0));
    let a = hits.clone();
    let b = hits.clone();
    let mut rt = Runtime::new(move |_| {
        let a = a.clone();
        let b = b.clone();
        Node::new(Kind::Div)
            .on_event(EventKind::Click, move |_| a.set(a.get() + 100))
            .children(vec![Node::new(Kind::Button)
                .id("b")
                .on_event(EventKind::Click, |e| e.stop_immediate_propagation())
                .on_event(EventKind::Click, move |_| b.set(b.get() + 1))])
    });
    rt.click_id("b");
    assert_eq!(hits.get(), 0);
}

#[test]
fn pointer_capture_routes_move_and_releases_on_cancel_or_disposal() {
    let shown = Rc::new(RefCell::new(None));
    let out = shown.clone();
    let moved = Rc::new(Cell::new(0));
    let m = moved.clone();
    let mut rt = Runtime::new(move |cx| {
        let visible = cx.signal(true);
        *out.borrow_mut() = Some(visible.clone());
        let m = m.clone();
        dynamic(move || {
            if visible.get() {
                let m = m.clone();
                Node::new(Kind::Button)
                    .id("drag")
                    .on_pointer_down(|e| e.capture_pointer())
                    .on_pointer_move(move |_| m.set(m.get() + 1))
            } else {
                Node::fragment(vec![])
            }
        })
    });
    let id = rt.snapshot().find("drag").unwrap().id;
    let result = rt.dispatch(id, EventData::pointer(EventKind::PointerDown, 7, 1., 2.));
    assert_eq!(result.pointer_capture, Some(id));
    rt.dispatch(0, EventData::pointer(EventKind::PointerMove, 7, 9., 10.));
    assert_eq!(moved.get(), 1);
    assert!(
        rt.dispatch(0, EventData::pointer(EventKind::PointerCancel, 7, 0., 0.))
            .pointer_released
    );
    assert_eq!(rt.captured_pointer(7), None);
    rt.dispatch(id, EventData::pointer(EventKind::PointerDown, 7, 0., 0.));
    shown.borrow().as_ref().unwrap().set(false).unwrap();
    rt.flush();
    assert_eq!(rt.captured_pointer(7), None);
}

#[test]
fn disabled_and_readonly_controls_reject_default_actions() {
    let hits = Rc::new(Cell::new(0));
    let a = hits.clone();
    let b = hits.clone();
    let mut rt = Runtime::new(move |_| {
        let a = a.clone();
        let b = b.clone();
        Node::fragment(vec![
            Node::widget(Widget::Button)
                .id("off")
                .disabled(true)
                .on_click(move |_| a.set(a.get() + 1)),
            Node::widget(Widget::TextInput)
                .id("read")
                .value("fixed")
                .read_only(true)
                .on_input(move |_| b.set(b.get() + 1)),
        ])
    });
    assert!(!rt.click_id("off"));
    let id = rt.snapshot().find("read").unwrap().id;
    assert!(!rt.input(id, "changed".into(), false));
    assert_eq!(hits.get(), 0);
    assert_eq!(rt.edit_value(id).as_deref(), Some("fixed"));
}

#[test]
fn controlled_checkbox_slider_and_choices_emit_typed_values() {
    let values = Rc::new(RefCell::new(vec![]));
    let a = values.clone();
    let b = values.clone();
    let c = values.clone();
    let mut rt = Runtime::new(move |_| {
        let a = a.clone();
        let b = b.clone();
        let c = c.clone();
        Node::fragment(vec![
            Node::widget(Widget::Checkbox)
                .id("check")
                .checked(false)
                .on_input(move |e| a.borrow_mut().push(e.value)),
            Node::widget(Widget::Slider)
                .id("slider")
                .value(9)
                .min(0.)
                .max(10.)
                .step(2.)
                .on_input(move |e| b.borrow_mut().push(e.value)),
            Node::widget(Widget::Select)
                .id("choice")
                .value("a")
                .options(vec![Choice::new("a", "Alpha"), Choice::new("b", "Beta")])
                .on_input(move |e| c.borrow_mut().push(e.value)),
        ])
    });
    rt.click_id("check");
    let slider = rt.snapshot().find("slider").unwrap().id;
    assert!(
        rt.dispatch(slider, EventData::key("ArrowRight"))
            .default_prevented
    );
    let choice = rt.snapshot().find("choice").unwrap().id;
    assert!(
        rt.dispatch(choice, EventData::key("ArrowDown"))
            .default_prevented
    );
    assert_eq!(*values.borrow(), ["true", "10", "b"]);
}

#[test]
fn password_values_are_masked_in_every_snapshot_and_semantic_dump() {
    let mut rt = Runtime::new(|_| {
        Node::widget(Widget::PasswordInput)
            .id("secret")
            .editing(Editor::new("first-secret"))
            .value("first-secret")
    });
    let id = rt.snapshot().find("secret").unwrap().id;
    assert!(!format!("{:?}", rt.snapshot()).contains("first-secret"));
    assert!(!format!("{:?}", rt.semantics()).contains("first-secret"));
    assert_eq!(rt.edit_value(id).as_deref(), Some("first-secret"));
    assert!(rt.edit_sync(id, "🌍second-secret".into(), 15, 15, false));
    assert!(!format!("{:?}", rt.snapshot()).contains("second-secret"));
}

#[test]
fn editor_identity_and_selection_survive_controlled_patches() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut rt = Runtime::new(move |cx| {
        let value = cx.signal("hello".to_string());
        *out.borrow_mut() = Some(value.clone());
        dynamic(move || {
            let edit = value.clone();
            Node::widget(Widget::TextInput)
                .id("field")
                .value(value.get())
                .on_input(move |e| {
                    let _ = edit.set(e.value);
                })
        })
    });
    let id = rt.snapshot().find("field").unwrap().id;
    assert!(rt.edit_sync(id, "hello!".into(), 2, 4, false));
    rt.flush();
    assert_eq!(rt.snapshot().find("field").unwrap().id, id);
    assert_eq!(
        rt.editor(id).unwrap().selection(),
        Selection {
            anchor: 2,
            focus: 4
        }
    );
    saved
        .borrow()
        .as_ref()
        .unwrap()
        .set("reset".into())
        .unwrap();
    rt.flush();
    assert_eq!(rt.edit_value(id).as_deref(), Some("reset"));
}

#[test]
fn nested_scroll_consumes_once_and_chains_at_boundary() {
    let states = Rc::new(RefCell::new(None));
    let out = states.clone();
    let mut rt = Runtime::new(move |cx| {
        let outer = ScrollState::new(&cx, 100., 500.);
        let inner = ScrollState::new(&cx, 50., 100.);
        *out.borrow_mut() = Some((outer.clone(), inner.clone()));
        scroll_view(
            outer,
            scroll_view(inner, Node::text("content").id("target")),
        )
    });
    let id = rt.snapshot().find("target").unwrap().id;
    let mut event = EventData::new(EventKind::Wheel);
    event.delta_y = 50.;
    assert!(rt.dispatch(id, event.clone()).default_prevented);
    let (outer, inner) = states.borrow().clone().unwrap();
    assert_eq!(outer.offset.get(), 0.);
    assert_eq!(inner.offset.get(), 50.);
    rt.dispatch(id, event);
    assert_eq!(outer.offset.get(), 50.);
    assert_eq!(inner.offset.get(), 50.);
}

#[test]
fn table_sort_and_selection_remain_keyed_and_controlled() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut rt = Runtime::new(move |cx| {
        let state = TableState::new(
            &cx,
            vec![
                TableRecord {
                    key: "z".into(),
                    cells: vec!["Zebra".into()],
                },
                TableRecord {
                    key: "a".into(),
                    cells: vec!["Alpha".into()],
                },
            ],
        );
        *out.borrow_mut() = Some(state.clone());
        data_table(
            state,
            vec![TableColumn {
                label: "Name".into(),
            }],
            "Records",
        )
    });
    let state = saved.borrow().clone().unwrap();
    state.sort(0);
    rt.flush();
    let text = rt.snapshot().text_content();
    assert!(text.find("Alpha").unwrap() < text.find("Zebra").unwrap());
    state.selected.set(Some("z".into())).unwrap();
    state.sort(0);
    rt.flush();
    assert_eq!(state.selected.get().as_deref(), Some("z"));
}

#[test]
fn full_widget_markup_and_typed_semantics_are_available() {
    let rt = Runtime::new(|_| {
        rustx! {
            <Column>
                <Heading level={ 1 }>
                    "Heading"
                </Heading>
                <Checkbox checked={ true } disabled={ false }>
                    "Consent"
                </Checkbox>
                <PasswordInput value="secret" />
                <TextArea value="lines" />
                <Slider value="5" min={ 0. } max={ 10. } />
                <Select options={ vec![Choice::new("one", "One")] } value="one" />
                <Dialog modal={ true } label="Dialog">
                    <Button>
                        "Close"
                    </Button>
                </Dialog>
                <Table>
                    <TableRow>
                        <TableCell>
                            "Cell"
                        </TableCell>
                    </TableRow>
                </Table>
                <Alert>
                    "Alert"
                </Alert>
                <Progress value="50" />
            </Column>
        }
    });
    fn roles(n: &SemanticNode, out: &mut Vec<&'static str>) {
        out.push(n.role);
        for c in &n.children {
            roles(c, out);
        }
    }
    let mut actual = vec![];
    roles(&rt.semantics(), &mut actual);
    for expected in [
        "heading",
        "checkbox",
        "textbox",
        "slider",
        "combobox",
        "dialog",
        "table",
        "row",
        "cell",
        "alert",
        "progressbar",
    ] {
        assert!(actual.contains(&expected), "missing {expected}");
    }
}

#[test]
fn ordinary_paragraph_selection_is_readonly_keyed_and_reactive_without_duplicate_text() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut rt = Runtime::new(move |cx| {
        let text = cx.signal("A🌍 سلام".to_owned());
        *out.borrow_mut() = Some(text.clone());
        rustx! {
            <Paragraph id="copyable" selectable={ true }>
                { move || text.get() }
            </Paragraph>
        }
    });
    let snapshot = rt.snapshot();
    let text = snapshot.find("copyable").unwrap();
    let id = text.id;
    assert_eq!(text.kind, Kind::Text);
    assert_eq!(text.text_content(), "A🌍 سلام");
    assert!(text.focusable() && text.interactive);
    assert!(text.children.is_empty());
    let editor = rt.editor(id).unwrap();
    editor.select_all();
    assert_eq!(editor.copy().unwrap(), "A🌍 سلام");
    assert_eq!(editor.insert("changed"), Err(EditError::ReadOnly));
    assert!(!rt.edit_sync(id, "changed".into(), 7, 7, false));
    fn find_text(node: &SemanticNode) -> Option<&SemanticNode> {
        if node.app_id.as_deref() == Some("copyable") {
            Some(node)
        } else {
            node.children.iter().find_map(find_text)
        }
    }
    let semantics = rt.semantics();
    let semantics = find_text(&semantics).unwrap();
    assert_eq!(semantics.role, "textbox");
    assert!(semantics.control.read_only && semantics.control.multiline);
    assert_eq!(semantics.value, "A🌍 سلام");
    saved
        .borrow()
        .as_ref()
        .unwrap()
        .set("Updated text".to_owned())
        .unwrap();
    rt.flush();
    let snapshot = rt.snapshot();
    assert_eq!(snapshot.find("copyable").unwrap().id, id);
    assert_eq!(
        snapshot.find("copyable").unwrap().text_content(),
        "Updated text"
    );
    assert_eq!(editor.value(), "Updated text");
    assert!(editor.is_read_only());
}

#[test]
fn select_popup_commits_named_options_and_closes_with_escape() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut rt = Runtime::new(move |cx| {
        let value = cx.signal("a".to_string());
        *out.borrow_mut() = Some(value.clone());
        select_with_id(
            "choice",
            value,
            vec![Choice::new("a", "Alpha"), Choice::new("b", "Beta")],
            "Choice",
        )
    });
    assert!(rt.click_id("choice"));
    rt.flush();
    assert!(rt.snapshot().find("choice-options").is_some());
    fn find_option(n: &Snapshot) -> Option<NodeId> {
        if n.role == Some("option") && n.text == "Beta" {
            Some(n.id)
        } else {
            n.children.iter().find_map(find_option)
        }
    }
    let id = find_option(&rt.snapshot()).unwrap();
    rt.click(id, 0., 0.);
    rt.flush();
    assert_eq!(saved.borrow().as_ref().unwrap().get(), "b");
    assert!(rt.snapshot().find("choice-options").is_none());
    rt.click_id("choice");
    rt.flush();
    let id = rt.snapshot().find("choice").unwrap().id;
    rt.dispatch(id, EventData::key("Escape"));
    rt.flush();
    assert!(rt.snapshot().find("choice-options").is_none());
}

#[test]
fn combobox_filters_then_commits_without_overwriting_value_while_typing() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut rt = Runtime::new(move |cx| {
        let value = cx.signal("a".to_string());
        *out.borrow_mut() = Some(value.clone());
        combobox_with_id(
            "city",
            value,
            vec![Choice::new("a", "Athens"), Choice::new("b", "Berlin")],
            "City",
        )
    });
    let id = rt.snapshot().find("city").unwrap().id;
    rt.edit_sync(id, "Ber".into(), 3, 3, false);
    rt.flush();
    assert_eq!(saved.borrow().as_ref().unwrap().get(), "a");
    assert!(rt.snapshot().find("city-options").is_some());
    rt.dispatch(id, EventData::key("Enter"));
    rt.flush();
    assert_eq!(saved.borrow().as_ref().unwrap().get(), "b");
    assert_eq!(rt.edit_value(id).as_deref(), Some("Berlin"));
    assert!(rt.snapshot().find("city-options").is_none());
}

#[test]
fn controlled_combobox_updates_its_label_and_closes_a_stale_search() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut rt = Runtime::new(move |cx| {
        let value = cx.signal("a".to_string());
        *out.borrow_mut() = Some(value.clone());
        combobox_with_id(
            "city",
            value,
            vec![Choice::new("a", "Athens"), Choice::new("b", "Berlin")],
            "City",
        )
    });
    let id = rt.snapshot().find("city").unwrap().id;
    assert_eq!(rt.edit_value(id).as_deref(), Some("Athens"));
    rt.edit_sync(id, "Ath".into(), 3, 3, false);
    rt.flush();
    assert!(rt.snapshot().find("city-options").is_some());
    let value = saved.borrow().as_ref().unwrap().clone();
    value.set("b".to_string()).unwrap();
    while rt.needs_update() {
        rt.flush();
    }
    assert_eq!(rt.edit_value(id).as_deref(), Some("Berlin"));
    assert!(rt.snapshot().find("city-options").is_none());
    value.set("unknown".to_string()).unwrap();
    while rt.needs_update() {
        rt.flush();
    }
    assert_eq!(rt.edit_value(id).as_deref(), Some("unknown"));
    value.set("a".to_string()).unwrap();
    while rt.needs_update() {
        rt.flush();
    }
    assert_eq!(rt.edit_value(id).as_deref(), Some("Athens"));
}

#[test]
fn combobox_keyboard_navigation_skips_disabled_options() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut rt = Runtime::new(move |cx| {
        let value = cx.signal(String::new());
        *out.borrow_mut() = Some(value.clone());
        combobox_with_id(
            "city",
            value,
            vec![
                Choice::new("a", "Athens"),
                Choice {
                    disabled: true,
                    ..Choice::new("b", "Berlin")
                },
                Choice::new("c", "Cairo"),
            ],
            "City",
        )
    });
    let id = rt.snapshot().find("city").unwrap().id;
    rt.dispatch(id, EventData::key("ArrowDown"));
    rt.flush();
    assert!(rt
        .snapshot()
        .find("city-option-2")
        .unwrap()
        .control
        .selected
        .unwrap());
    rt.dispatch(id, EventData::key("ArrowUp"));
    rt.flush();
    assert!(rt
        .snapshot()
        .find("city-option-0")
        .unwrap()
        .control
        .selected
        .unwrap());
    rt.dispatch(id, EventData::key("ArrowDown"));
    rt.flush();
    rt.dispatch(id, EventData::key("Enter"));
    rt.flush();
    assert_eq!(saved.borrow().as_ref().unwrap().get(), "c");
    assert_eq!(rt.edit_value(id).as_deref(), Some("Cairo"));
}

#[test]
fn tooltip_describes_trigger_and_toasts_stay_bounded() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut rt = Runtime::new(move |cx| {
        let queue = ToastQueue::new(&cx);
        *out.borrow_mut() = Some(queue.clone());
        Node::fragment(vec![
            tooltip(
                "help",
                Node::widget(Widget::Button).id("trigger").value("Help"),
                "Helpful description",
            ),
            toast_region(queue),
        ])
    });
    let id = rt.snapshot().find("trigger").unwrap().id;
    rt.dispatch(id, EventData::new(EventKind::Focus));
    rt.flush();
    assert_eq!(
        rt.snapshot()
            .find("trigger")
            .unwrap()
            .control
            .described_by
            .as_deref(),
        Some("help")
    );
    assert!(rt.snapshot().find("help").is_some());
    rt.dispatch(id, EventData::key("Escape"));
    rt.flush();
    assert!(rt.snapshot().find("help").is_none());
    let queue = saved.borrow().clone().unwrap();
    for i in 0..20 {
        queue.push(format!("Notice {i}"));
    }
    assert_eq!(queue.messages.get().len(), 8);
    let id = queue.messages.get()[0].id;
    queue.dismiss(id);
    assert_eq!(queue.messages.get().len(), 7);
}
