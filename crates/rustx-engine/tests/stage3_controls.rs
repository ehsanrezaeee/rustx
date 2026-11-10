//! Shared Rust control geometry and actual software pixel state transitions.
use rustx_core::{dynamic, Node, Signal, Style, Widget};
use rustx_engine::vello::kurbo::Shape;
use rustx_engine::{
    controls::{decorations, indicator_bounds, label_inset},
    cpu::CpuRenderer,
    Engine,
};
use std::{cell::RefCell, rc::Rc};

fn assert_indicator_paths_contained(control: &rustx_engine::layout::BoxNode) {
    let bounds = indicator_bounds(control).unwrap();
    for (path, _) in decorations(control) {
        let painted = path.bounding_box();
        assert!(
            painted.x0 >= bounds.x0 - 0.000_001
                && painted.x1 <= bounds.x1 + 0.000_001
                && painted.y0 >= bounds.y0 - 0.000_001
                && painted.y1 <= bounds.y1 + 0.000_001,
            "indicator decoration {painted:?} escapes {bounds:?}"
        );
    }
}

fn assert_pixel_color(pixmap: &tiny_skia::Pixmap, x: u32, y: u32, color: u32) {
    let pixel = pixmap.pixel(x, y).unwrap();
    for (actual, expected) in [
        (pixel.red(), (color >> 16) as u8),
        (pixel.green(), (color >> 8) as u8),
        (pixel.blue(), color as u8),
    ] {
        assert!(
            actual.abs_diff(expected) <= 3,
            "pixel at {x},{y}: {pixel:?}; expected {color:06x}"
        );
    }
}

#[test]
fn indicators_reserve_intrinsic_label_width_and_paint_state_without_changing_labels() {
    let state = Rc::new(RefCell::new(None::<Signal<bool>>));
    let out = state.clone();
    let mut engine = Engine::new(move |cx| {
        let checked = cx.signal(false);
        *out.borrow_mut() = Some(checked.clone());
        dynamic(move || {
            let checked = checked.get();
            Node::widget(Widget::Row)
                .style(Style::row().gap(0.))
                .children(vec![
                    Node::widget(Widget::Button).id("plain").value("Choice"),
                    Node::widget(Widget::Checkbox)
                        .id("checkbox")
                        .value("Choice")
                        .checked(checked),
                    Node::widget(Widget::Button)
                        .role("radio")
                        .id("radio")
                        .value("Choice")
                        .checked(checked),
                    Node::widget(Widget::Switch)
                        .id("switch")
                        .value("Choice")
                        .checked(checked),
                ])
        })
    });
    engine.frame(800., 100., 1.);
    let mut cpu = CpuRenderer::new(800, 100);
    cpu.draw(
        engine.boxes.as_ref().unwrap(),
        &mut engine.text,
        800,
        100,
        1.,
    );
    let before = cpu.pixmap.clone();
    let boxes = engine.boxes.as_ref().unwrap();
    let plain_width = boxes.find("plain").unwrap().rect.width();
    for id in ["checkbox", "radio", "switch"] {
        let control = boxes.find(id).unwrap();
        let indicator = indicator_bounds(control).unwrap();
        let inset = f64::from(label_inset(&control.node));
        assert!(
            (control.rect.width() - plain_width - inset).abs() < 0.01,
            "{id} intrinsic width must include the indicator and gap"
        );
        assert_eq!(
            indicator.x1 + 8.,
            control.rect.x0 + f64::from(control.node.style.padding) + inset
        );
        assert!(indicator.y0 >= control.rect.y0 && indicator.y1 <= control.rect.y1);
        assert_indicator_paths_contained(control);
        if id != "switch" {
            assert_pixel_color(
                &before,
                indicator.x0.ceil() as u32,
                indicator.center().y.floor() as u32,
                0x95a4bd,
            );
            assert_pixel_color(
                &before,
                indicator.center().x.floor() as u32,
                indicator.center().y.floor() as u32,
                control.node.style.background.unwrap(),
            );
        }
    }
    state.borrow().as_ref().unwrap().set(true).unwrap();
    assert!(engine.frame(800., 100., 1.));
    cpu.draw(
        engine.boxes.as_ref().unwrap(),
        &mut engine.text,
        800,
        100,
        1.,
    );
    let boxes = engine.boxes.as_ref().unwrap();
    for id in ["checkbox", "radio", "switch"] {
        let control = boxes.find(id).unwrap();
        let indicator = indicator_bounds(control).unwrap();
        assert_indicator_paths_contained(control);
        if id == "radio" {
            assert_pixel_color(
                &cpu.pixmap,
                indicator.center().x.floor() as u32,
                indicator.center().y.floor() as u32,
                0xb5f36b,
            );
            assert_pixel_color(
                &cpu.pixmap,
                indicator.x0.ceil() as u32,
                indicator.center().y.floor() as u32,
                0xb5f36b,
            );
        }
        let mut changed = 0;
        for y in indicator.y0 as u32..indicator.y1 as u32 {
            for x in indicator.x0 as u32..indicator.x1 as u32 {
                changed += usize::from(before.pixel(x, y) != cpu.pixmap.pixel(x, y));
            }
        }
        assert!(
            changed > 30,
            "{id} must visibly distinguish checked and unchecked state"
        );
        let label_x =
            control.rect.x0 + f64::from(control.node.style.padding + label_inset(&control.node));
        for y in control.rect.y0.ceil() as u32..control.rect.y1.floor() as u32 {
            for x in label_x.ceil() as u32..control.rect.x1.floor() as u32 {
                assert_eq!(
                    before.pixel(x, y),
                    cpu.pixmap.pixel(x, y),
                    "{id} indicator must not overwrite its label"
                );
            }
        }
    }
}
