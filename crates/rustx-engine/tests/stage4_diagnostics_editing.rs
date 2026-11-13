use rustx_core::*;
use rustx_engine::{cpu::CpuRenderer, Engine};
#[test]
fn multiline_viewport_reveals_caret_scrolls_and_invalidates_both_scene_and_pixels() {
    let value = (0..50)
        .map(|n| format!("Line {n}: hello سلام\n"))
        .collect::<String>();
    let editor = Editor::new(value.clone());
    let model = editor.clone();
    let mut engine = Engine::new(move |_| {
        Node::widget(Widget::TextArea)
            .id("code")
            .value(&value)
            .editing(model.clone())
            .style(
                Style::column()
                    .width(260.)
                    .height(80.)
                    .padding(8.)
                    .font_size(18.)
                    .clip(),
            )
    });
    engine.frame(400., 200., 1.);
    let id = engine.runtime.snapshot().find("code").unwrap().id;
    engine.focus_node(id);
    let mut cpu = CpuRenderer::new(400, 200);
    cpu.draw(
        engine.boxes.as_ref().unwrap(),
        &mut engine.text,
        400,
        200,
        1.,
    );
    let first = cpu.pixmap.data().to_vec();
    assert!(engine.key("End", false, false, false, false));
    engine.frame(400., 200., 1.);
    assert!(editor.scroll_y() > 100.);
    cpu.draw(
        engine.boxes.as_ref().unwrap(),
        &mut engine.text,
        400,
        200,
        1.,
    );
    assert_ne!(first, cpu.pixmap.data());
    assert!(engine.painter.painted > 0);
    assert!(engine.key("Home", false, false, false, false));
    engine.frame(400., 200., 1.);
    assert_eq!(editor.scroll_y(), 0.);
    assert!(!editor.scroll_by(-100.));
    assert!(editor.scroll_by(100.));
    engine.runtime.invalidate_model();
    engine.frame(400., 200., 1.);
    assert!(editor.scroll_y() > 0.);
    editor.set_value("short").unwrap();
    engine.runtime.invalidate_model();
    engine.frame(400., 200., 1.);
    assert_eq!(editor.scroll_y(), 0.);
    assert!(!editor.scroll_by(f32::NAN));
}
#[test]
fn diagnostics_are_opt_in_bounded_redacted_and_source_linked() {
    let mut engine = Engine::new(|_| {
        Node::widget(Widget::PasswordInput)
            .id("password")
            .value("private-password")
            .source(file!(), line!(), column!())
    });
    engine.frame(400., 200., 1.);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&engine.diagnostics.json()).unwrap()["frames"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    engine.diagnostics.enable(true);
    let id = engine.runtime.snapshot().find("password").unwrap().id;
    for n in 0..300 {
        let mut event = EventData::new(EventKind::KeyDown);
        event.key = "s".into();
        event.value = "private-password".into();
        engine.dispatch(id, event);
        engine.frame(400. + (n % 2) as f32, 200., 1.);
    }
    let raw = engine.diagnostics.json();
    assert!(!raw.contains("private-password"));
    let data: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(data["frames"].as_array().unwrap().len(), 120);
    assert_eq!(data["events"].as_array().unwrap().len(), 256);
    assert!(data["events"][0]["path"].as_array().unwrap().len() > 0);
    let snapshot = engine.runtime.snapshot();
    assert!(
        snapshot
            .find("password")
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .line
            > 0
    );
    assert!(engine
        .runtime
        .ownership()
        .iter()
        .any(|owner| owner["id"] == snapshot.find("password").unwrap().owner));
    engine.diagnostics.enable(false);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&engine.diagnostics.json()).unwrap()["events"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}
