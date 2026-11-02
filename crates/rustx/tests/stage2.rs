//! Deterministic Stage 2 public-API contracts; these are not platform evidence.
use rustx::drawing::{Bounds, Camera, CanvasScene, DrawShape, PathSegment, Primitive, Transform};
use rustx::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn walk<'a>(node: &'a Snapshot, out: &mut Vec<&'a Snapshot>) {
    out.push(node);
    for child in &node.children {
        walk(child, out);
    }
}
fn rows(node: &Snapshot) -> Vec<usize> {
    let mut all = vec![];
    walk(node, &mut all);
    all.into_iter()
        .filter_map(|n| {
            n.position.map(|(position, total)| {
                assert_eq!(total, 100_000);
                position - 1
            })
        })
        .collect()
}

#[test]
fn virtual_list_bounds_materialized_scopes_and_retains_focused_state() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mounted = Rc::new(Cell::new(0usize));
    let disposed = Rc::new(Cell::new(0usize));
    let mounts = mounted.clone();
    let drops = disposed.clone();
    let mut runtime = Runtime::new(move |cx| {
        let list = VirtualListState::new(&cx, 100_000, 16., 320.);
        *out.borrow_mut() = Some(list.clone());
        let mounts = mounts.clone();
        let drops = drops.clone();
        virtual_list(list, move |index| {
            let mounts = mounts.clone();
            let drops = drops.clone();
            Node::component("EditableRecord", move |cx| {
                mounts.set(mounts.get() + 1);
                let drops = drops.clone();
                cx.on_cleanup(move || drops.set(drops.get() + 1));
                let count = cx.signal(0usize);
                let show = count.clone();
                Node::new(Kind::Button)
                    .id(format!("row-{index}"))
                    .on_click(move |_| {
                        let _ = count.update(|value| *value += 1);
                    })
                    .children(vec![dynamic(move || format!("{index}:{}", show.get()))])
            })
        })
    });
    let state = saved.borrow().clone().unwrap();
    assert!(rows(&runtime.snapshot()).len() <= 120);
    runtime.click_id("row-0");
    runtime.flush();
    let original = runtime.snapshot().find("row-0").unwrap().id;
    runtime.focus(original);
    runtime.flush();
    assert_eq!(state.focused.get(), Some(0));
    state.scroll_to(32_000.);
    runtime.flush();
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.find("row-0").unwrap().id, original);
    assert_eq!(snapshot.find("row-0").unwrap().text_content(), "0:1");
    assert!(snapshot.find("row-1").is_none());
    assert!(rows(&snapshot).contains(&2000));
    assert!(rows(&snapshot).len() <= 120);
    let warm = runtime.counts();
    for i in 1..=100 {
        state.scroll_to((i * 13_600) as f64);
        runtime.flush();
        assert!(rows(&runtime.snapshot()).len() <= 120);
        assert!(runtime.counts().0 <= warm.0 + 20);
    }
    state.scroll_to(0.);
    runtime.flush();
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.find("row-0").unwrap().id, original);
    assert_eq!(mounted.get() - disposed.get(), rows(&snapshot).len());
    drop(runtime);
    assert_eq!(mounted.get(), disposed.get());
    assert!(state.offset.peek().is_err());
}

#[test]
fn virtual_scroll_clamps_and_ignores_nonfinite_input() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let _runtime = Runtime::new(move |cx| {
        let state = VirtualListState::new(&cx, 100_000, 16., 320.);
        *out.borrow_mut() = Some(state);
        Node::text("")
    });
    let state = saved.borrow().clone().unwrap();
    state.scroll_to(-30.);
    assert_eq!(state.offset.get(), 0.);
    state.scroll_to(1e20);
    assert_eq!(state.offset.get(), 1_599_680.);
    assert_eq!(state.range().end, 100_000);
    state.scroll_to(f64::NAN);
    state.scroll_by(f64::INFINITY);
    assert_eq!(state.offset.get(), 1_599_680.);
}

#[test]
fn responsive_queries_are_ordered_and_theme_tokens_resolve() {
    let style = Style::grid(1)
        .padding(4.)
        .font_size(12.)
        .background_token(ColorToken::Surface)
        .color_token(ColorToken::Text)
        .at_width(ResponsiveStyle {
            min_width: 900.,
            columns: Some(4),
            font_size: Some(24.),
            padding: Some(20.),
            direction: None,
        })
        .at_width(ResponsiveStyle {
            min_width: 600.,
            columns: Some(2),
            font_size: Some(18.),
            padding: Some(12.),
            direction: Some(Direction::Row),
        });
    let narrow = style.resolve(390., Theme::DARK);
    let medium = style.resolve(700., Theme::LIGHT);
    let wide = style.resolve(1200., Theme::LIGHT);
    assert_eq!(
        (narrow.columns, narrow.padding, narrow.font_size),
        (1, 4., 12.)
    );
    assert_eq!(
        (medium.columns, medium.padding, medium.font_size),
        (2, 12., 18.)
    );
    assert_eq!((wide.columns, wide.padding, wide.font_size), (4, 20., 24.));
    assert_eq!(wide.background, Some(Theme::LIGHT.surface));
    assert_eq!(wide.color, Theme::LIGHT.text);
    assert_eq!(medium.direction, Direction::Row);
    assert_eq!(style.queries.len(), 2);
    assert!(wide.queries.is_empty());
}

#[test]
fn spatial_drawing_queries_cull_ten_thousand_shapes_without_reordering() {
    let scene = CanvasScene::new(
        (0..10_000)
            .map(|i| {
                DrawShape::rect(
                    i + 1,
                    Bounds::new((i % 100) as f64 * 60., (i / 100) as f64 * 48., 48., 34.),
                    0x00ff00,
                )
            })
            .collect(),
    )
    .unwrap();
    let visible = scene.visible(Camera::default(), 1280., 720.);
    assert!(!visible.is_empty() && visible.len() <= 1000);
    assert!(visible.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(scene.hit(Camera::default(), 24., 17.), Some(1));
    assert_eq!(scene.hit(Camera::default(), 55., 40.), None);
    let mut camera = Camera::default();
    camera.pan(-120., -96.);
    assert_eq!(scene.hit(camera, 24., 17.), Some(203));
    let before = (camera.x + 160. / camera.zoom, camera.y + 120. / camera.zoom);
    camera.zoom_at(160., 120., 1.25);
    let after = (camera.x + 160. / camera.zoom, camera.y + 120. / camera.zoom);
    assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
}

#[test]
fn drawing_large_coordinates_and_transformed_topmost_hits_are_stable() {
    let base = 900_000_000_000.;
    let a = DrawShape::rect(1, Bounds::new(base, base, 20., 20.), 0xff0000);
    let b = DrawShape {
        transform: Transform::translate(5., 5.),
        id: 2,
        ..a.clone()
    };
    let scene = CanvasScene::new(vec![a, b]).unwrap();
    let camera = Camera {
        x: base,
        y: base,
        zoom: 2.,
    };
    assert_eq!(scene.hit(camera, 24., 24.), Some(2));
    assert_eq!(scene.hit(camera, 4., 4.), Some(1));
    assert_eq!(scene.visible(camera, 100., 100.), vec![0, 1]);
}

#[test]
fn drawing_rejects_duplicate_ids_invalid_geometry_and_path_limits() {
    let shape = DrawShape::rect(1, Bounds::new(0., 0., 10., 10.), 0);
    assert!(CanvasScene::new(vec![shape.clone(), shape.clone()]).is_err());
    assert!(CanvasScene::new(vec![DrawShape {
        bounds: Bounds::new(0., 0., f64::INFINITY, 10.),
        ..shape.clone()
    }])
    .is_err());
    assert!(CanvasScene::new(vec![DrawShape {
        transform: Transform([0.; 6]),
        ..shape.clone()
    }])
    .is_err());
    assert!(CanvasScene::new(vec![DrawShape {
        primitive: Primitive::Path {
            segments: vec![PathSegment::Line(0., 0.); 4097],
            stroke: None
        },
        ..shape
    }])
    .is_err());
}

#[test]
fn drawing_hits_follow_rounded_and_path_geometry_not_only_bounds() {
    let rounded = DrawShape::rect(1, Bounds::new(0., 0., 20., 20.), 0xffffff);
    let triangle = DrawShape {
        id: 2,
        bounds: Bounds::new(30., 0., 100., 100.),
        fill: 0xffffff,
        primitive: Primitive::Path {
            segments: vec![
                PathSegment::Move(30., 0.),
                PathSegment::Line(130., 0.),
                PathSegment::Line(30., 100.),
                PathSegment::Close,
            ],
            stroke: None,
        },
        transform: Transform::default(),
    };
    let scene = CanvasScene::new(vec![rounded, triangle]).unwrap();
    assert_eq!(scene.hit(Camera::default(), 0.5, 0.5), None);
    assert_eq!(scene.hit(Camera::default(), 10., 10.), Some(1));
    assert_eq!(scene.hit(Camera::default(), 50., 20.), Some(2));
    assert_eq!(scene.hit(Camera::default(), 120., 90.), None);
}

#[test]
fn drawing_queries_include_stroke_overhang_and_reject_unsafe_path_bounds() {
    let shape = DrawShape {
        id: 1,
        bounds: Bounds::new(0., 0., 20., 20.),
        fill: 0xffffff,
        primitive: Primitive::Path {
            segments: vec![
                PathSegment::Move(0., 0.),
                PathSegment::Line(20., 0.),
                PathSegment::Line(20., 20.),
                PathSegment::Line(0., 20.),
                PathSegment::Close,
            ],
            stroke: Some((4., 0xffffff)),
        },
        transform: Transform::default(),
    };
    let scene = CanvasScene::new(vec![shape.clone()]).unwrap();
    assert_eq!(scene.query(Bounds::new(-2., 5., 1., 10.)), vec![0]);
    let unsafe_path = DrawShape {
        primitive: Primitive::Path {
            segments: vec![
                PathSegment::Move(0., 0.),
                PathSegment::Cubic(0., 0., 200., 200., 20., 20.),
            ],
            stroke: None,
        },
        ..shape
    };
    assert!(CanvasScene::new(vec![unsafe_path]).is_err());
}

#[test]
fn direct_virtual_offsets_and_invalid_viewports_do_not_overflow() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut runtime = Runtime::new(move |cx| {
        let state = VirtualListState::new(&cx, 100_000, 16., 320.);
        *out.borrow_mut() = Some(state.clone());
        virtual_list(state, |i| Node::text(i))
    });
    let state = saved.borrow().clone().unwrap();
    for invalid in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN, f64::MAX] {
        state.offset.set(invalid).unwrap();
        runtime.flush();
        let range = state.range();
        assert!(range.start <= range.end && range.end <= 100_000);
        let snapshot = runtime.snapshot();
        let mut all = vec![];
        walk(&snapshot, &mut all);
        assert!(all
            .iter()
            .all(|n| n.style.translate.0.is_finite() && n.style.translate.1.is_finite()));
    }
    let scene =
        CanvasScene::new(vec![DrawShape::rect(1, Bounds::new(0., 0., 10., 10.), 0)]).unwrap();
    assert!(scene
        .visible(Camera::default(), f64::INFINITY, 100.)
        .is_empty());
    assert!(scene
        .query(Bounds::new(0., 0., f64::INFINITY, 100.))
        .is_empty());
    assert_eq!(scene.hit(Camera::default(), f64::NAN, 0.), None);
}

#[test]
fn virtual_scrolling_from_a_row_routes_to_the_owning_list() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let mut runtime = Runtime::new(move |cx| {
        let state = VirtualListState::new(&cx, 100_000, 16., 320.);
        *out.borrow_mut() = Some(state.clone());
        virtual_list(state, |i| {
            Node::new(Kind::Button)
                .id(format!("record-{i}"))
                .children(vec![Node::text(i)])
        })
    });
    let state = saved.borrow().clone().unwrap();
    let row = runtime.snapshot().find("record-0").unwrap().id;
    assert!(
        runtime.scroll(row, 128.),
        "a wheel event over a row must reach its list owner"
    );
    runtime.flush();
    assert_eq!(state.offset.get(), 128.);
}
