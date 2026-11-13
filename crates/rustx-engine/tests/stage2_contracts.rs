//! Additional blueprint contracts found during the sequential four-stage review.
use rustx_core::*;
use rustx_engine::{cpu::CpuRenderer, text::TextEngine, vello::kurbo::Point, Engine};
use std::sync::Arc;

#[test]
fn percentages_constraints_and_physical_logical_spacing_share_retained_layout() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Div)
            .style(
                Style::column()
                    .width(400.)
                    .height(240.)
                    .padding_edges(Insets::new(10., 20., 30., 40.))
                    .gap(0.),
            )
            .children(vec![Node::new(Kind::Div).id("half").style(
                Style::column()
                    .width_percent(50.)
                    .height_percent(50.)
                    .min_height(30.)
                    .max_height(70.),
            )])
    });
    engine.frame(500., 300., 1.);
    let child = engine.boxes.as_ref().unwrap().find("half").unwrap();
    assert_eq!(
        (
            child.rect.x0,
            child.rect.y0,
            child.rect.width(),
            child.rect.height()
        ),
        (40., 10., 170., 70.)
    );
    let id = child.node.id;
    engine.frame(600., 300., 1.);
    assert_eq!(
        engine.boxes.as_ref().unwrap().find("half").unwrap().node.id,
        id
    );

    let mut engine = Engine::new(|_| {
        Node::new(Kind::Div)
            .style(
                Style::column()
                    .width(300.)
                    .gap(0.)
                    .text_direction(TextDirection::RightToLeft)
                    .padding_logical(5., 12., 7., 24.),
            )
            .children(vec![Node::new(Kind::Div).id("logical").style(
                Style::column()
                    .width(40.)
                    .height(20.)
                    .text_direction(TextDirection::RightToLeft)
                    .margin_logical(3., 6., 0., 9.),
            )])
    });
    engine.frame(500., 200., 1.);
    let child = engine.boxes.as_ref().unwrap().find("logical").unwrap();
    assert_eq!((child.rect.x0, child.rect.y0), (18., 8.));
    assert_eq!(
        Style::column().width_percent(50.).width(90.).width_length,
        Length::Auto
    );
    assert_eq!(
        Style::column().width(90.).width_length(Length::Auto).width,
        None
    );
}

#[test]
fn absolute_percent_offsets_and_z_order_agree_between_pixels_and_hit_testing() {
    let mut engine = Engine::new(|_| {
        let item = |id, ink, z| {
            Node::new(Kind::Button).id(id).on_click(|_| {}).style(
                Style::column()
                    .absolute()
                    .left(Length::Percent(25.))
                    .top(Length::Px(20.))
                    .width(80.)
                    .height(40.)
                    .background(ink)
                    .z_index(z),
            )
        };
        Node::new(Kind::Div)
            .style(Style::column().width(400.).height(150.))
            .children(vec![item("front", 0xff0000, 5), item("back", 0x0000ff, 1)])
    });
    engine.frame(500., 200., 1.);
    let root = engine.boxes.as_ref().unwrap();
    let front = root.find("front").unwrap();
    assert_eq!((front.rect.x0, front.rect.y0), (100., 20.));
    assert_eq!(
        engine.painter.hit(Point::new(120., 35.)),
        Some(front.node.id)
    );
    let mut cpu = CpuRenderer::new(500, 200);
    cpu.draw(root, &mut engine.text, 500, 200, 1.);
    let pixel = cpu.pixmap.pixel(120, 35).unwrap();
    assert_eq!((pixel.red(), pixel.green(), pixel.blue()), (255, 0, 0));
}

#[test]
fn typography_cache_line_height_and_grapheme_ellipsis_keep_original_source_offsets() {
    let mut text = TextEngine::default();
    let style = Style::column()
        .font_size(18.)
        .font_family("DejaVu Sans")
        .font_weight(500)
        .line_height(32.);
    let source = "Hello e\u{301} 👩‍👩‍👧‍👦 فارسی یک متن طولانی برای بررسی\nAnother line of text";
    let full = text.shape_styled(source, 120., &style);
    assert!(full.lines.len() > 2);
    assert_eq!(full.lines[0].height, 32.);
    assert!(Arc::ptr_eq(&full, &text.shape_styled(source, 120., &style)));
    let different_weight = text.shape_styled(source, 120., &style.clone().font_weight(700));
    assert!(!Arc::ptr_eq(&full, &different_weight));
    for direction in [TextDirection::LeftToRight, TextDirection::RightToLeft] {
        let limited = style.clone().ellipsis(2).text_direction(direction);
        let shaped = text.shape_styled(source, 120., &limited);
        assert!(shaped.lines.len() <= 2);
        assert!(shaped.width <= 120.01);
        assert!(shaped
            .clusters
            .iter()
            .all(|cluster| source.is_char_boundary(cluster.start)
                && source.is_char_boundary(cluster.end)));
        assert!(shaped
            .clusters
            .iter()
            .any(|cluster| cluster.end == source.len()));
    }
}

#[test]
fn editing_never_elides_text_and_asymmetric_padding_is_shared_with_painted_caret() {
    let value = "line one\nline two\nline three";
    let editor = Editor::new(value);
    let owned = editor.clone();
    let mut engine = Engine::new(move |_| {
        Node::widget(Widget::TextArea)
            .id("editor")
            .value(value)
            .editing(owned.clone())
            .style(
                Style::column()
                    .width(220.)
                    .height(80.)
                    .padding_edges(Insets::new(10., 5., 6., 30.))
                    .line_height(24.)
                    .ellipsis(1),
            )
    });
    engine.frame(300., 200., 1.);
    let b = engine.boxes.as_ref().unwrap().find("editor").unwrap();
    let shaped = engine.text.shape_node(
        &b.node,
        b.rect.width() as f32 - b.node.style.padding_horizontal(),
    );
    assert_eq!(shaped.lines.len(), 3);
    let caret = rustx_engine::editing::decorations(b, &mut engine.text)
        .into_iter()
        .find(|(_, _, alpha)| *alpha == 1.)
        .unwrap()
        .0;
    assert_eq!((caret.x0, caret.y0), (b.rect.x0 + 30., b.rect.y0 + 10.));
}
