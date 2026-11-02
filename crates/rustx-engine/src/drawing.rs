//! Shared camera-relative geometry used by GPU and software backends.
use rustx_core::drawing::{Camera, DrawShape, PathSegment, Primitive};
use vello::kurbo::{Affine, BezPath, Rect, RoundedRect, Shape};
pub fn geometry(shape: &DrawShape, camera: Camera, origin: (f64, f64)) -> (BezPath, Affine) {
    let b = shape.bounds;
    let (a, bm, c, d, _, _) = {
        let t = shape.transform.0;
        (t[0], t[1], t[2], t[3], t[4], t[5])
    };
    let (x, y) = shape.transform.point(b.x0, b.y0);
    let (x, y) = camera.project(x, y);
    let transform = Affine::new([
        a * camera.zoom,
        bm * camera.zoom,
        c * camera.zoom,
        d * camera.zoom,
        origin.0 + x,
        origin.1 + y,
    ]);
    let mut path = BezPath::new();
    match &shape.primitive {
        Primitive::Rect { radius } => {
            path = RoundedRect::from_rect(Rect::new(0., 0., b.width(), b.height()), *radius)
                .to_path(0.1)
        }
        Primitive::Path { segments, .. } => {
            for p in segments {
                match p {
                    PathSegment::Move(x, y) => path.move_to((x - b.x0, y - b.y0)),
                    PathSegment::Line(x, y) => path.line_to((x - b.x0, y - b.y0)),
                    PathSegment::Quad(x, y, a, c) => {
                        path.quad_to((x - b.x0, y - b.y0), (a - b.x0, c - b.y0))
                    }
                    PathSegment::Cubic(x, y, a, c, d, e) => path.curve_to(
                        (x - b.x0, y - b.y0),
                        (a - b.x0, c - b.y0),
                        (d - b.x0, e - b.y0),
                    ),
                    PathSegment::Close => path.close_path(),
                }
            }
        }
        _ => path = Rect::new(0., 0., b.width(), b.height()).to_path(0.1),
    }
    (path, transform)
}
