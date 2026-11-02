//! Portable retained drawing data. Backend geometry is derived from these values.
use kurbo::{BezPath, Rect, RoundedRect, Shape, Stroke, StrokeOpts};
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
impl Bounds {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            x0: x,
            y0: y,
            x1: x + w,
            y1: y + h,
        }
    }
    pub fn width(self) -> f64 {
        self.x1 - self.x0
    }
    pub fn height(self) -> f64 {
        self.y1 - self.y0
    }
    pub fn intersects(self, b: Self) -> bool {
        self.x0 < b.x1 && self.x1 > b.x0 && self.y0 < b.y1 && self.y1 > b.y0
    }
    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
    fn valid(self) -> bool {
        [self.x0, self.y0, self.x1, self.y1]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1e12)
            && self.width() > 0.
            && self.height() > 0.
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform(pub [f64; 6]);
impl Default for Transform {
    fn default() -> Self {
        Self([1., 0., 0., 1., 0., 0.])
    }
}
impl Transform {
    pub fn translate(x: f64, y: f64) -> Self {
        Self([1., 0., 0., 1., x, y])
    }
    pub fn rotate(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        Self([c, s, -s, c, 0., 0.])
    }
    pub fn point(self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }
    pub fn inverse_point(self, x: f64, y: f64) -> Option<(f64, f64)> {
        let [a, b, c, d, e, f] = self.0;
        let det = a * d - b * c;
        if !det.is_finite() || det.abs() < 1e-12 {
            return None;
        }
        Some((
            (d * (x - e) - c * (y - f)) / det,
            (-b * (x - e) + a * (y - f)) / det,
        ))
    }
    pub fn bounds(self, r: Bounds) -> Bounds {
        let points = [
            self.point(r.x0, r.y0),
            self.point(r.x1, r.y0),
            self.point(r.x1, r.y1),
            self.point(r.x0, r.y1),
        ];
        Bounds {
            x0: points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
            y0: points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
            x1: points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
            y1: points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            zoom: 1.,
        }
    }
}
impl Camera {
    pub fn valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.zoom.is_finite()
            && (0.05..=20.).contains(&self.zoom)
    }
    pub fn pan(&mut self, dx: f64, dy: f64) {
        if dx.is_finite() && dy.is_finite() {
            self.x = (self.x - dx / self.zoom).clamp(-1e12, 1e12);
            self.y = (self.y - dy / self.zoom).clamp(-1e12, 1e12);
        }
    }
    pub fn zoom_at(&mut self, x: f64, y: f64, factor: f64) {
        if !factor.is_finite() || factor <= 0. || !x.is_finite() || !y.is_finite() {
            return;
        }
        let z = (self.zoom * factor).clamp(0.05, 20.);
        self.x += x / self.zoom - x / z;
        self.y += y / self.zoom - y / z;
        self.zoom = z;
    }
    pub fn project(self, x: f64, y: f64) -> (f64, f64) {
        ((x - self.x) * self.zoom, (y - self.y) * self.zoom)
    }
}
#[derive(Clone, Debug)]
pub enum PathSegment {
    Move(f64, f64),
    Line(f64, f64),
    Quad(f64, f64, f64, f64),
    Cubic(f64, f64, f64, f64, f64, f64),
    Close,
}
#[derive(Clone, Debug)]
pub enum Primitive {
    Rect {
        radius: f64,
    },
    Path {
        segments: Vec<PathSegment>,
        stroke: Option<(f64, u32)>,
    },
    Text {
        text: String,
        size: f32,
    },
    Image {
        key: String,
    },
}
#[derive(Clone, Debug)]
pub struct DrawShape {
    pub id: u64,
    pub bounds: Bounds,
    pub fill: u32,
    pub primitive: Primitive,
    pub transform: Transform,
}
impl DrawShape {
    pub fn path(&self) -> BezPath {
        let mut path = BezPath::new();
        match &self.primitive {
            Primitive::Rect { radius } => {
                return RoundedRect::from_rect(
                    Rect::new(
                        self.bounds.x0,
                        self.bounds.y0,
                        self.bounds.x1,
                        self.bounds.y1,
                    ),
                    *radius,
                )
                .to_path(0.01)
            }
            Primitive::Path { segments, .. } => {
                for p in segments {
                    match *p {
                        PathSegment::Move(x, y) => path.move_to((x, y)),
                        PathSegment::Line(x, y) => path.line_to((x, y)),
                        PathSegment::Quad(a, b, c, d) => path.quad_to((a, b), (c, d)),
                        PathSegment::Cubic(a, b, c, d, e, f) => {
                            path.curve_to((a, b), (c, d), (e, f))
                        }
                        PathSegment::Close => path.close_path(),
                    }
                }
            }
            _ => {
                return Rect::new(
                    self.bounds.x0,
                    self.bounds.y0,
                    self.bounds.x1,
                    self.bounds.y1,
                )
                .to_path(0.01)
            }
        }
        path
    }
    pub fn paint_bounds(&self) -> Bounds {
        let mut b = self.bounds;
        if let Primitive::Path {
            stroke: Some((width, _)),
            ..
        } = &self.primitive
        {
            b.x0 -= width / 2.;
            b.y0 -= width / 2.;
            b.x1 += width / 2.;
            b.y1 += width / 2.;
        }
        self.transform.bounds(b)
    }
    pub fn rect(id: u64, bounds: Bounds, fill: u32) -> Self {
        Self {
            id,
            bounds,
            fill,
            primitive: Primitive::Rect { radius: 6. },
            transform: Default::default(),
        }
    }
}
#[derive(Debug)]
pub struct CanvasScene {
    shapes: Vec<DrawShape>,
    buckets: HashMap<(i64, i64), Vec<usize>>,
    large: Vec<usize>,
}
impl CanvasScene {
    pub fn new(shapes: Vec<DrawShape>) -> Result<Self, &'static str> {
        if shapes.len() > 100_000 {
            return Err("drawing limit: 100,000 shapes");
        }
        let mut seen = HashSet::new();
        let mut buckets: HashMap<_, Vec<_>> = HashMap::new();
        let mut large = vec![];
        for (i, s) in shapes.iter().enumerate() {
            if !seen.insert(s.id)
                || !s.bounds.valid()
                || !s.transform.0.iter().all(|v| v.is_finite())
                || s.transform.inverse_point(0., 0.).is_none()
            {
                return Err("invalid bounds, transform or duplicate drawing ID");
            }
            match &s.primitive {
                Primitive::Path { segments, stroke } => {
                    if segments.len() > 4096
                        || stroke.is_some_and(|(w, _)| !w.is_finite() || w < 0.)
                    {
                        return Err("invalid path");
                    }
                    for p in segments {
                        let coords = match p {
                            PathSegment::Move(x, y) | PathSegment::Line(x, y) => vec![*x, *y],
                            PathSegment::Quad(a, b, c, d) => vec![*a, *b, *c, *d],
                            PathSegment::Cubic(a, b, c, d, e, f) => vec![*a, *b, *c, *d, *e, *f],
                            PathSegment::Close => vec![],
                        };
                        if coords.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
                            return Err("invalid path coordinate");
                        }
                        if coords
                            .chunks_exact(2)
                            .any(|p| !s.bounds.contains(p[0], p[1]))
                        {
                            return Err("path control point outside declared bounds");
                        }
                    }
                }
                Primitive::Text { text, size }
                    if text.len() > 8192 || !size.is_finite() || *size < 1. || *size > 256. =>
                {
                    return Err("invalid drawing text")
                }
                Primitive::Rect { radius } if !radius.is_finite() || *radius < 0. => {
                    return Err("invalid radius")
                }
                _ => (),
            }
            let b = s.paint_bounds();
            if !b.valid() {
                return Err("invalid transformed bounds");
            }
            let (x0, y0, x1, y1) = (
                (b.x0 / 256.).floor() as i64,
                (b.y0 / 256.).floor() as i64,
                (b.x1 / 256.).floor() as i64,
                (b.y1 / 256.).floor() as i64,
            );
            if (x1 - x0 + 1).saturating_mul(y1 - y0 + 1) > 64 {
                large.push(i);
            } else {
                for x in x0..=x1 {
                    for y in y0..=y1 {
                        buckets.entry((x, y)).or_default().push(i);
                    }
                }
            }
        }
        Ok(Self {
            shapes,
            buckets,
            large,
        })
    }
    pub fn len(&self) -> usize {
        self.shapes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }
    pub fn shape(&self, index: usize) -> &DrawShape {
        &self.shapes[index]
    }
    pub fn query(&self, b: Bounds) -> Vec<usize> {
        if !b.valid() {
            return vec![];
        }
        let (x0, y0, x1, y1) = (
            (b.x0 / 256.).floor() as i64,
            (b.y0 / 256.).floor() as i64,
            (b.x1 / 256.).floor() as i64,
            (b.y1 / 256.).floor() as i64,
        );
        let mut ids = self.large.clone();
        if (x1 - x0 + 1).saturating_mul(y1 - y0 + 1) > 4096 {
            ids = (0..self.len()).collect();
        } else {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    if let Some(v) = self.buckets.get(&(x, y)) {
                        ids.extend(v);
                    }
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids.retain(|i| self.shapes[*i].paint_bounds().intersects(b));
        ids
    }
    pub fn visible(&self, camera: Camera, width: f64, height: f64) -> Vec<usize> {
        if !camera.valid() {
            return vec![];
        }
        self.query(Bounds::new(
            camera.x,
            camera.y,
            width / camera.zoom,
            height / camera.zoom,
        ))
    }
    pub fn hit(&self, camera: Camera, x: f64, y: f64) -> Option<u64> {
        if !camera.valid() {
            return None;
        }
        let (x, y) = (camera.x + x / camera.zoom, camera.y + y / camera.zoom);
        self.query(Bounds::new(x, y, 0.01, 0.01))
            .iter()
            .rev()
            .find_map(|i| {
                let s = &self.shapes[*i];
                let p = s.transform.inverse_point(x, y)?;
                let path = s.path();
                let point = kurbo::Point::new(p.0, p.1);
                let filled = path.winding(point) != 0;
                let stroked = if let Primitive::Path {
                    stroke: Some((width, _)),
                    ..
                } = &s.primitive
                {
                    kurbo::stroke(
                        path.iter(),
                        &Stroke::new(*width),
                        &StrokeOpts::default(),
                        0.01,
                    )
                    .winding(point)
                        != 0
                } else {
                    false
                };
                (filled || stroked).then_some(s.id)
            })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CanvasEvent {
    Pan { dx: f64, dy: f64 },
    Zoom { x: f64, y: f64, factor: f64 },
    Pick { x: f64, y: f64 },
}
