//! Spatial buckets, viewport culling, and camera-relative f64 coordinates.
use std::collections::HashMap;
use vello::{
    kurbo::{Affine, Point, Rect, RoundedRect},
    peniko::Fill,
    Scene,
};
#[derive(Clone, Debug)]
pub struct Shape {
    pub id: u64,
    pub bounds: Rect,
    pub color: u32,
}
#[derive(Clone, Copy, Debug)]
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
    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.x -= dx / self.zoom;
        self.y -= dy / self.zoom;
    }
    pub fn zoom_at(&mut self, p: Point, factor: f64) {
        let z = (self.zoom * factor).clamp(0.25, 4.);
        self.x += p.x / self.zoom - p.x / z;
        self.y += p.y / self.zoom - p.y / z;
        self.zoom = z;
    }
    pub fn project(&self, p: Point) -> Point {
        Point::new((p.x - self.x) * self.zoom, (p.y - self.y) * self.zoom)
    }
}
pub struct World {
    pub camera: Camera,
    shapes: Vec<Shape>,
    buckets: HashMap<(i64, i64), Vec<usize>>,
    pub visible: usize,
}
impl World {
    pub fn new(shapes: Vec<Shape>) -> Self {
        let mut buckets: HashMap<_, Vec<_>> = HashMap::new();
        for (i, s) in shapes.iter().enumerate() {
            for x in (s.bounds.x0 / 256.).floor() as i64..=(s.bounds.x1 / 256.).floor() as i64 {
                for y in (s.bounds.y0 / 256.).floor() as i64..=(s.bounds.y1 / 256.).floor() as i64 {
                    buckets.entry((x, y)).or_default().push(i)
                }
            }
        }
        Self {
            camera: Camera::default(),
            shapes,
            buckets,
            visible: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.shapes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }
    pub fn query(&self, r: Rect) -> Vec<usize> {
        let mut ids = vec![];
        for x in (r.x0 / 256.).floor() as i64..=(r.x1 / 256.).floor() as i64 {
            for y in (r.y0 / 256.).floor() as i64..=(r.y1 / 256.).floor() as i64 {
                if let Some(bucket) = self.buckets.get(&(x, y)) {
                    ids.extend(
                        bucket
                            .iter()
                            .copied()
                            .filter(|i| self.shapes[*i].bounds.intersect(r).area() > 0.),
                    );
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }
    pub fn hit(&self, p: Point) -> Option<u64> {
        let p = Point::new(
            self.camera.x + p.x / self.camera.zoom,
            self.camera.y + p.y / self.camera.zoom,
        );
        self.query(Rect::new(p.x, p.y, p.x + 0.01, p.y + 0.01))
            .iter()
            .rev()
            .find_map(|i| {
                self.shapes[*i]
                    .bounds
                    .contains(p)
                    .then_some(self.shapes[*i].id)
            })
    }
    pub fn paint(&mut self, scene: &mut Scene, viewport: Rect) {
        let c = self.camera;
        let ids = self.query(Rect::new(
            c.x,
            c.y,
            c.x + viewport.width() / c.zoom,
            c.y + viewport.height() / c.zoom,
        ));
        self.visible = ids.len();
        for i in ids {
            let s = &self.shapes[i];
            let p = c.project(s.bounds.origin());
            let rect = Rect::from_origin_size(
                (viewport.x0 + p.x, viewport.y0 + p.y),
                (s.bounds.width() * c.zoom, s.bounds.height() * c.zoom),
            );
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                crate::color(s.color),
                None,
                &RoundedRect::from_rect(rect, 6. * c.zoom),
            );
        }
    }
}
#[derive(Debug, PartialEq)]
pub struct VirtualRange {
    pub start: usize,
    pub end: usize,
    pub before: f32,
    pub after: f32,
}
pub fn virtual_range(
    count: usize,
    row_height: f32,
    offset: f32,
    viewport: f32,
    overscan: usize,
) -> VirtualRange {
    let row = row_height.max(1.);
    let offset = offset.max(0.).min((count as f32 * row - viewport).max(0.));
    let start = ((offset / row).floor() as usize)
        .saturating_sub(overscan)
        .min(count);
    let end = (((offset + viewport.max(0.)) / row).ceil() as usize)
        .saturating_add(overscan)
        .min(count);
    VirtualRange {
        start,
        end,
        before: start as f32 * row,
        after: (count - end) as f32 * row,
    }
}
