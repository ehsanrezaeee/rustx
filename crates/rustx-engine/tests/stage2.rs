//! Shared engine CPU/core contracts. GPU/browser/device acceptance is separate.
use rustx_core::drawing::{
    Bounds, Camera, CanvasScene, DrawShape, PathSegment, Primitive, Transform,
};
use rustx_core::{
    dynamic, Align, ColorToken, Justify, Kind, Node, ResponsiveStyle, Style, TextAlign,
    TextDirection, Theme,
};
use rustx_engine::vello::{
    kurbo::{Point, Shape},
    peniko::{Blob, Image, ImageFormat},
};
use rustx_engine::{
    cache::BudgetCache,
    cpu::CpuRenderer,
    drawing::geometry,
    text::{byte_to_utf16, utf16_to_byte, TextEngine},
    Engine,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[test]
fn layout_reflows_at_breakpoints_and_theme_changes_do_not_remount() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Div)
            .id("grid")
            .style(Style::grid(1).gap(10.).at_width(ResponsiveStyle {
                min_width: 700.,
                columns: Some(2),
                font_size: None,
                padding: None,
                direction: None,
            }))
            .children(vec![
                Node::text("Alpha").id("a").style(
                    Style::column()
                        .height(40.)
                        .background_token(ColorToken::Surface),
                ),
                Node::text("Beta")
                    .id("b")
                    .style(Style::column().height(40.).color_token(ColorToken::Text)),
            ])
    });
    engine.frame(390., 600., 1.);
    let small = engine.boxes.as_ref().unwrap();
    let a = small.find("a").unwrap().rect;
    let b = small.find("b").unwrap().rect;
    let counts = engine.runtime.counts();
    let a_id = small.find("a").unwrap().node.id;
    assert_eq!(a.x0, b.x0);
    assert!(b.y0 >= a.y1);
    engine.frame(900., 600., 1.);
    let wide = engine.boxes.as_ref().unwrap();
    let a = wide.find("a").unwrap().rect;
    let b = wide.find("b").unwrap().rect;
    assert_eq!(a.y0, b.y0);
    assert!(b.x0 >= a.x1);
    engine.set_theme(Theme::LIGHT);
    assert!(engine.frame(900., 600., 1.));
    let boxes = engine.boxes.as_ref().unwrap();
    assert_eq!(
        boxes.find("a").unwrap().node.style.background,
        Some(Theme::LIGHT.surface)
    );
    assert_eq!(boxes.find("b").unwrap().node.style.color, Theme::LIGHT.text);
    assert_eq!(boxes.find("a").unwrap().node.id, a_id);
    assert_eq!(engine.runtime.counts(), counts);
    assert!(!engine.frame(900., 600., 1.));
}

#[test]
fn layout_alignment_growth_and_width_constraints_are_applied() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Div)
            .id("row")
            .style(
                Style::row()
                    .width(400.)
                    .height(100.)
                    .gap(0.)
                    .align(Align::Center)
                    .justify(Justify::SpaceBetween),
            )
            .children(vec![
                Node::new(Kind::Div)
                    .id("left")
                    .style(Style::column().width(20.).height(30.)),
                Node::new(Kind::Div)
                    .id("right")
                    .style(Style::column().width(40.).height(30.)),
            ])
    });
    engine.frame(500., 300., 1.);
    let boxes = engine.boxes.as_ref().unwrap();
    let a = boxes.find("left").unwrap().rect;
    let b = boxes.find("right").unwrap().rect;
    assert_eq!(a.y0, 35.);
    assert_eq!(b.y0, 35.);
    assert_eq!(a.x0, 0.);
    assert_eq!(b.x1, 400.);
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Div)
            .style(Style::row().width(400.).gap(0.))
            .children(vec![
                Node::new(Kind::Div)
                    .id("flex")
                    .style(Style::column().grow(1.).min_width(60.).max_width(140.)),
                Node::new(Kind::Div)
                    .id("fixed")
                    .style(Style::column().width(100.)),
            ])
    });
    engine.frame(500., 300., 1.);
    assert_eq!(
        engine
            .boxes
            .as_ref()
            .unwrap()
            .find("flex")
            .unwrap()
            .rect
            .width(),
        140.
    );
}

#[test]
fn retained_layout_avoids_invalidation_for_color_only_updates() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut engine = Engine::new(move |cx| {
        let value = cx.signal(0xff0000);
        *out.borrow_mut() = Some(value.clone());
        Node::new(Kind::Div).style(Style::row()).children(vec![
            dynamic(move || {
                Node::text("Stable measure")
                    .id("target")
                    .style(Style::column().color(value.get()))
            }),
            Node::text("Independent").id("sibling"),
        ])
    });
    engine.frame(800., 300., 1.);
    let before = engine.boxes.as_ref().unwrap().find("sibling").unwrap().rect;
    saved.borrow().as_ref().unwrap().set(0x0000ff).unwrap();
    engine.frame(800., 300., 1.);
    assert_eq!(engine.layout.changed, 0);
    assert_eq!(
        engine.boxes.as_ref().unwrap().find("sibling").unwrap().rect,
        before
    );
    assert_eq!(
        engine
            .boxes
            .as_ref()
            .unwrap()
            .find("target")
            .unwrap()
            .node
            .style
            .color,
        0x0000ff
    );
}

#[test]
fn text_cache_reuses_shapes_reflows_and_invalidation_follows_font_readiness() {
    let mut text = TextEngine::default();
    let source = "سلام RustX — English 123 🦀 🚀";
    let wide = text.shape(source, 20., 800.);
    let count = text.shape_count;
    let again = text.shape(source, 20., 800.);
    assert!(Arc::ptr_eq(&wide, &again));
    assert_eq!(text.shape_count, count);
    assert_eq!(wide.missing, 0);
    assert!(wide.runs.len() >= 2);
    let narrow = text.shape(source, 20., 90.);
    assert!(narrow.height > wide.height);
    let epoch = text.epoch;
    assert!(text.add_font(vec![0; 32]).is_err());
    assert_eq!(text.epoch, epoch);
    text.add_font(include_bytes!("../assets/DejaVuSans.ttf").to_vec())
        .unwrap();
    assert_eq!(text.epoch, epoch + 1);
    assert_eq!(text.cache_stats(), (0, 0));
    let refreshed = text.shape(source, 20., 800.);
    assert!(!Arc::ptr_eq(&wide, &refreshed));
    assert_eq!(refreshed.missing, 0);
    for i in 0..700 {
        text.shape(&format!("Fixture {i} سلام"), 16., 200.);
    }
    assert!(text.cache_stats().0 <= 512);
    assert!(text.cache_stats().1 <= 8 * 1024 * 1024);
}

#[test]
fn text_direction_alignment_clusters_and_utf16_boundaries_share_the_model() {
    let source = "abc سلام 🦀\nEnglish e\u{301}";
    let mut text = TextEngine::default();
    for direction in [
        TextDirection::Auto,
        TextDirection::LeftToRight,
        TextDirection::RightToLeft,
    ] {
        let shaped = text.shape_with(source, 20., 500., TextAlign::Start, direction);
        assert_eq!(shaped.missing, 0);
        assert!(shaped.clusters.iter().any(|c| c.rtl));
        assert!(shaped.clusters.iter().any(|c| !c.rtl));
        for cluster in &shaped.clusters {
            assert!(cluster.start <= cluster.end && cluster.end <= source.len());
            assert!(source.is_char_boundary(cluster.start) && source.is_char_boundary(cluster.end));
            assert!(cluster.x.is_finite() && cluster.y.is_finite());
        }
    }
    let left = text.shape_with(
        "abc",
        20.,
        500.,
        TextAlign::Start,
        TextDirection::LeftToRight,
    );
    let center = text.shape_with(
        "abc",
        20.,
        500.,
        TextAlign::Center,
        TextDirection::LeftToRight,
    );
    let right = text.shape_with("abc", 20., 500., TextAlign::End, TextDirection::LeftToRight);
    let min = |s: &rustx_engine::text::Shaped| {
        s.clusters.iter().map(|c| c.x).fold(f32::INFINITY, f32::min)
    };
    assert!(min(&left) < min(&center) && min(&center) < min(&right));
    let sample = "a🦀ب";
    for byte in [0, 1, 5, 7] {
        let utf16 = byte_to_utf16(sample, byte).unwrap();
        assert_eq!(utf16_to_byte(sample, utf16), Some(byte));
    }
    assert_eq!(utf16_to_byte(sample, 2), None);
    assert_eq!(byte_to_utf16(sample, 2), None);
    assert_eq!(byte_to_utf16(sample, 100), None);
}

#[test]
fn font_readiness_invalidates_existing_layout_once() {
    let mut engine = Engine::new(|_| Node::text("Measured سلام 🦀").id("text"));
    engine.frame(400., 300., 1.);
    assert!(!engine.frame(400., 300., 1.));
    let id = engine.boxes.as_ref().unwrap().find("text").unwrap().node.id;
    engine
        .text
        .add_font(include_bytes!("../assets/DejaVuSans.ttf").to_vec())
        .unwrap();
    assert!(engine.frame(400., 300., 1.));
    assert!(engine.layout.changed > 0);
    assert!(!engine.frame(400., 300., 1.));
    assert_eq!(
        engine.boxes.as_ref().unwrap().find("text").unwrap().node.id,
        id
    );
}

#[test]
fn budget_cache_obeys_byte_entry_and_lru_constraints() {
    let mut cache = BudgetCache::new(3, 10);
    cache.insert(1, "a", 4);
    cache.insert(2, "b", 4);
    assert_eq!(cache.get(&1), Some(&"a"));
    cache.insert(3, "c", 4);
    assert_eq!(cache.get(&2), None);
    assert_eq!(cache.len(), 2);
    assert_eq!(cache.bytes(), 8);
    cache.insert(4, "oversized", 11);
    assert_eq!(cache.get(&4), None);
    cache.insert(1, "replace", 2);
    assert_eq!(cache.bytes(), 6);
    cache.clear();
    assert_eq!((cache.len(), cache.bytes()), (0, 0));
    let mut cache = BudgetCache::new(0, 10);
    cache.insert(1, "a", 1);
    assert_eq!(cache.len(), 0);
}

#[test]
fn drawing_geometry_rebases_large_coordinates_before_backend_float_conversion() {
    let base = 900_000_000_000.;
    let shape = DrawShape::rect(7, Bounds::new(base + 5., base + 7., 20., 30.), 0xff0000);
    let camera = Camera {
        x: base,
        y: base,
        zoom: 2.,
    };
    let (path, transform) = geometry(&shape, camera, (10., 20.));
    assert_eq!(path.bounding_box().width(), 20.);
    assert_eq!(path.bounding_box().height(), 30.);
    assert_eq!(transform * Point::new(0., 0.), Point::new(20., 34.));
    assert_eq!(transform * Point::new(20., 30.), Point::new(60., 94.));
}

#[test]
fn cpu_drawings_honor_canvas_clip_path_fill_and_camera() {
    let scene = Rc::new(
        CanvasScene::new(vec![
            DrawShape::rect(1, Bounds::new(0., 0., 100., 100.), 0xff0000),
            DrawShape {
                id: 2,
                bounds: Bounds::new(0., 0., 40., 40.),
                fill: 0x00ff00,
                primitive: Primitive::Path {
                    segments: vec![
                        PathSegment::Move(0., 0.),
                        PathSegment::Line(40., 0.),
                        PathSegment::Line(0., 40.),
                        PathSegment::Close,
                    ],
                    stroke: None,
                },
                transform: Transform::translate(10., 10.),
            },
        ])
        .unwrap(),
    );
    let mut engine = Engine::new(move |_| {
        Node::new(Kind::Div)
            .style(Style::column().padding(10.))
            .children(vec![Node::new(Kind::Canvas)
                .style(Style::column().width(50.).height(50.).clip())
                .drawing(scene.clone())])
    });
    engine.frame(80., 80., 1.);
    let mut cpu = CpuRenderer::new(80, 80);
    cpu.draw(engine.boxes.as_ref().unwrap(), &mut engine.text, 80, 80, 1.);
    let inside = cpu.pixmap.pixel(25, 25).unwrap();
    assert_eq!((inside.red(), inside.green(), inside.blue()), (0, 255, 0));
    let beneath = cpu.pixmap.pixel(52, 52).unwrap();
    assert_eq!(
        (beneath.red(), beneath.green(), beneath.blue()),
        (255, 0, 0)
    );
    let outside = cpu.pixmap.pixel(65, 25).unwrap();
    assert_eq!(
        (outside.red(), outside.green(), outside.blue()),
        (11, 16, 24)
    );
    assert_eq!(cpu.drawing_visible, 2);
    assert_eq!(engine.painter.drawing_visible, 2);
}

#[test]
fn cpu_cropped_opacity_preserves_parent_clip_and_global_geometry() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Div)
            .style(Style::column().padding(10.))
            .children(vec![Node::new(Kind::Div)
                .style(
                    Style::column()
                        .width(40.)
                        .height(40.)
                        .clip()
                        .background(0xffffff),
                )
                .children(vec![Node::new(Kind::Div).style(
                    Style::column()
                        .width(80.)
                        .height(80.)
                        .translate(10., 10.)
                        .opacity(0.5)
                        .background(0x000000),
                )])])
    });
    engine.frame(100., 100., 1.);
    let mut cpu = CpuRenderer::new(100, 100);
    cpu.draw(
        engine.boxes.as_ref().unwrap(),
        &mut engine.text,
        100,
        100,
        1.,
    );
    assert_eq!(cpu.pixmap.pixel(15, 15).unwrap().red(), 255);
    let overlap = cpu.pixmap.pixel(25, 25).unwrap();
    assert!((126..=129).contains(&overlap.red()));
    assert_eq!(cpu.pixmap.pixel(55, 25).unwrap().red(), 11);
}

#[test]
fn image_registration_rejects_invalid_sizes_and_removal_invalidates_caches() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Image)
            .value("test")
            .style(Style::column().width(20.).height(20.))
    });
    let valid = Image::new(
        Blob::new(Arc::new(vec![255u8; 16])),
        ImageFormat::Rgba8,
        2,
        2,
    );
    engine.painter.image("test", valid).unwrap();
    engine.frame(40., 40., 1.);
    assert_eq!(engine.painter.resource_stats().0, 1);
    let invalid = Image::new(Blob::new(Arc::new(vec![0u8; 3])), ImageFormat::Rgba8, 2, 2);
    assert!(engine.painter.image("bad", invalid).is_err());
    engine.painter.remove_image("test");
    assert_eq!(engine.painter.resource_stats().0, 0);
    assert_eq!(engine.painter.resource_stats().2, 0);
}

#[test]
fn explicit_text_direction_preserves_trailing_paragraphs() {
    let mut text = TextEngine::default();
    let source = "abc\n\n";
    let auto = text.shape_with(source, 20., 200., TextAlign::Start, TextDirection::Auto);
    let forced = text.shape_with(
        source,
        20.,
        200.,
        TextAlign::Start,
        TextDirection::LeftToRight,
    );
    assert_eq!(
        auto.height, forced.height,
        "a direction hint must not remove trailing blank paragraphs"
    );
}

#[test]
fn text_coordinates_remain_finite_for_unbounded_measurement() {
    let mut text = TextEngine::default();
    let shaped = text.shape_with(
        "abc سلام",
        20.,
        f32::INFINITY,
        TextAlign::End,
        TextDirection::RightToLeft,
    );
    assert!(shaped.width.is_finite() && shaped.height.is_finite());
    assert!(shaped
        .clusters
        .iter()
        .all(|c| c.x.is_finite() && c.y.is_finite()));
}

#[test]
fn image_resource_updates_repaint_an_otherwise_idle_engine() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Image)
            .value("image")
            .style(Style::column().width(20.).height(20.))
    });
    engine.frame(40., 40., 1.);
    assert!(!engine.frame(40., 40., 1.));
    let image = Image::new(
        Blob::new(Arc::new(vec![255u8; 16])),
        ImageFormat::Rgba8,
        2,
        2,
    );
    engine.painter.image("image", image).unwrap();
    assert!(
        engine.frame(40., 40., 1.),
        "new image readiness must repaint without a component-state change"
    );
    assert!(!engine.frame(40., 40., 1.));
    engine.painter.remove_image("image");
    assert!(engine.frame(40., 40., 1.));
    assert!(!engine.frame(40., 40., 1.));
}

#[test]
fn cpu_image_pixels_use_straight_rgba_source_alpha() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Image)
            .value("alpha")
            .style(Style::column().width(20.).height(20.))
    });
    let rgba: Vec<u8> = std::iter::repeat_n([255, 0, 0, 128], 4).flatten().collect();
    let image = Image::new(Blob::new(Arc::new(rgba)), ImageFormat::Rgba8, 2, 2);
    engine.painter.image("alpha", image).unwrap();
    engine.frame(30., 30., 1.);
    let mut cpu = CpuRenderer::new(30, 30);
    cpu.sync_images(&engine.painter);
    cpu.draw(engine.boxes.as_ref().unwrap(), &mut engine.text, 30, 30, 1.);
    let pixel = cpu.pixmap.pixel(10, 10).unwrap();
    assert!(
        (132..=134).contains(&pixel.red()),
        "red={}; straight RGBA must be premultiplied before software compositing",
        pixel.red()
    );
    assert!((7..=9).contains(&pixel.green()) && (11..=13).contains(&pixel.blue()));
}

#[test]
fn cpu_image_alpha_multiplier_matches_the_shared_asset_contract() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Image)
            .value("alpha")
            .style(Style::column().width(20.).height(20.))
    });
    let image = Image::new(
        Blob::new(Arc::new(vec![255u8; 16])),
        ImageFormat::Rgba8,
        2,
        2,
    )
    .with_alpha(0.5);
    engine.painter.image("alpha", image).unwrap();
    engine.frame(30., 30., 1.);
    let mut cpu = CpuRenderer::new(30, 30);
    cpu.sync_images(&engine.painter);
    cpu.draw(engine.boxes.as_ref().unwrap(), &mut engine.text, 30, 30, 1.);
    let pixel = cpu.pixmap.pixel(10, 10).unwrap();
    assert!(
        (132..=134).contains(&pixel.red())
            && (135..=137).contains(&pixel.green())
            && (139..=141).contains(&pixel.blue()),
        "actual pixel={pixel:?}"
    );
}

#[test]
fn cpu_culling_keeps_visible_shadow_of_an_offscreen_clipped_node() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Div).children(vec![Node::new(Kind::Div).style(
            Style::column()
                .width(20.)
                .height(20.)
                .translate(80., 10.)
                .clip()
                .shadow(0xffffff, 0., (-30., 0.)),
        )])
    });
    engine.frame(80., 80., 1.);
    let mut cpu = CpuRenderer::new(80, 80);
    cpu.draw(engine.boxes.as_ref().unwrap(), &mut engine.text, 80, 80, 1.);
    assert!(
        cpu.pixmap.pixel(55, 15).unwrap().red() > 200,
        "the visible shadow lies inside the viewport even though its owner box lies outside"
    );
}
