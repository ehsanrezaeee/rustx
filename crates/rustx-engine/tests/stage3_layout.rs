//! Shared retained-layout contracts; these do not certify a browser or GPU.
use rustx_core::*;
use rustx_engine::{vello::kurbo::Point, Engine};

#[test]
fn stack_children_overlap_and_reserve_the_largest_child_extent() {
    let mut engine = Engine::new(|_| {
        Node::widget(Widget::Column)
            .style(Style::column().gap(0.))
            .children(vec![
                Node::widget(Widget::Stack)
                    .id("stack")
                    .style(Style::grid(1).gap(0.))
                    .children(vec![
                        Node::widget(Widget::Box)
                            .id("back")
                            .style(Style::column().width(100.).height(80.)),
                        Node::widget(Widget::Box)
                            .id("front")
                            .style(Style::column().width(60.).height(40.)),
                    ]),
                Node::text("after").id("after"),
            ])
    });
    engine.frame(500., 300., 1.);
    let boxes = engine.boxes.as_ref().unwrap();
    let back = boxes.find("back").unwrap().rect;
    let front = boxes.find("front").unwrap().rect;
    let stack = boxes.find("stack").unwrap().rect;
    assert_eq!(back.origin(), front.origin());
    assert!(stack.height() >= 80.);
    assert_eq!(boxes.find("after").unwrap().rect.y0, stack.y1);
    let snapshot = engine.runtime.snapshot();
    engine
        .layout
        .compute(&snapshot, 500., 300., &mut engine.text);
    assert_eq!(
        engine.layout.changed, 0,
        "unchanged stack must retain Taffy styles"
    );
}

#[test]
fn popup_is_measured_at_viewport_and_painted_outside_its_clipped_parent() {
    let mut engine = Engine::new(|_| {
        Node::widget(Widget::Column)
            .id("clip")
            .style(Style::column().width(100.).height(40.).clip())
            .children(vec![
                Node::widget(Widget::Button)
                    .id("anchor")
                    .value("Open")
                    .style(Style::column().width(80.).height(30.)),
                Node::widget(Widget::Popover)
                    .id("popup")
                    .anchor("anchor")
                    .style(Style::column().width(180.).height(80.))
                    .children(vec![Node::widget(Widget::Button)
                        .id("option")
                        .value("A retained option")
                        .style(Style::column().width(180.).height(60.))]),
            ])
    });
    engine.frame(500., 300., 1.);
    let boxes = engine.boxes.as_ref().unwrap();
    let clip = boxes.find("clip").unwrap();
    let popup = boxes.find("popup").unwrap();
    let option = boxes.find("option").unwrap();
    assert_eq!(popup.rect.width(), 180.);
    assert!(popup.rect.y1 > clip.rect.y1);
    assert!(clip.find("popup").is_none());
    assert!(boxes
        .children
        .iter()
        .any(|b| b.node.app_id.as_deref() == Some("popup")));
    assert_eq!(
        engine
            .painter
            .hit(Point::new(option.rect.x0 + 120., option.rect.y0 + 20.)),
        Some(option.node.id)
    );
    // The projection's semantic relationships retain the application hierarchy.
    assert!(engine
        .runtime
        .snapshot()
        .find("clip")
        .unwrap()
        .find("popup")
        .is_some());
}

#[test]
fn anchored_popup_follows_outer_scroll_and_flips_inside_the_viewport() {
    let mut engine = Engine::new(|_| {
        Node::widget(Widget::Column)
            .style(Style::column().gap(0.))
            .children(vec![
                Node::widget(Widget::Box).style(Style::column().height(100.)),
                Node::widget(Widget::Button)
                    .id("anchor")
                    .value("Open")
                    .style(Style::column().width(100.).height(30.)),
                Node::widget(Widget::Popover)
                    .id("popup")
                    .anchor("anchor")
                    .style(Style::column().width(180.).height(60.)),
                Node::widget(Widget::Box).style(Style::column().height(700.)),
            ])
    });
    engine.frame(500., 300., 1.);
    let initial = engine.boxes.as_ref().unwrap().find("popup").unwrap().rect;
    let anchor = engine.boxes.as_ref().unwrap().find("anchor").unwrap().rect;
    assert_eq!(initial.y0, anchor.y1 + 4.);
    engine.scroll(40.);
    engine.frame(500., 300., 1.);
    let boxes = engine.boxes.as_ref().unwrap();
    let popup = boxes.find("popup").unwrap().rect;
    let anchor = boxes.find("anchor").unwrap().rect;
    assert_eq!(popup.y0, anchor.y1 + 4.);
    assert_eq!(popup.y0, initial.y0 - 40.);
    engine.frame(500., 140., 1.);
    let boxes = engine.boxes.as_ref().unwrap();
    let popup = boxes.find("popup").unwrap().rect;
    let anchor = boxes.find("anchor").unwrap().rect;
    assert!(popup.y1 <= anchor.y0 - 4. + f64::EPSILON || popup.y0 == 8.);
    assert!(popup.y1 <= 140.);
}

#[test]
fn modal_is_centered_after_resize_and_stays_fixed_when_background_scrolls() {
    let mut engine = Engine::new(|_| {
        Node::widget(Widget::Column).children(vec![
            Node::widget(Widget::Box).style(Style::column().height(1000.)),
            Node::widget(Widget::Dialog)
                .id("modal")
                .label("Dialog")
                .style(Style::column().width(200.).height(100.))
                .children(vec![Node::widget(Widget::Button).value("Close")]),
        ])
    });
    engine.frame(500., 400., 1.);
    let rect = engine.boxes.as_ref().unwrap().find("modal").unwrap().rect;
    assert_eq!(rect.origin(), Point::new(150., 150.));
    engine.scroll(80.);
    engine.frame(500., 400., 1.);
    assert_eq!(
        engine.boxes.as_ref().unwrap().find("modal").unwrap().rect,
        rect
    );
    engine.frame(320., 240., 1.);
    let rect = engine.boxes.as_ref().unwrap().find("modal").unwrap().rect;
    assert_eq!(rect.origin(), Point::new(60., 70.));
    assert!(rect.x1 <= 320. && rect.y1 <= 240.);
}
