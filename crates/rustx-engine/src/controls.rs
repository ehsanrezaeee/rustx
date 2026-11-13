//! Renderer-independent control geometry, expressed in logical Rust coordinates.
use crate::layout::BoxNode;
use rustx_core::Snapshot;
use vello::kurbo::{BezPath, Rect, RoundedRect, Shape};

/// Space reserved before a control's visible label, including its indicator gap.
pub fn label_inset(node: &Snapshot) -> f32 {
    match node.role {
        Some("switch") => 42.,
        Some("checkbox" | "radio" | "menuitemcheckbox" | "menuitemradio") => 26.,
        _ => 0.,
    }
}

pub fn indicator_bounds(b: &BoxNode) -> Option<Rect> {
    let inset = label_inset(&b.node);
    if inset == 0. {
        return None;
    }
    let width = if b.node.role == Some("switch") {
        34.
    } else {
        18.
    };
    let style = &b.node.style;
    let x = b.rect.x0 + f64::from(style.padding_left());
    let y = b.rect.y0
        + f64::from(style.padding_top())
        + (b.rect.height() - f64::from(style.padding_vertical()) - 18.) * 0.5;
    Some(Rect::new(x, y, x + width, y + 18.))
}

pub fn decorations(b: &BoxNode) -> Vec<(BezPath, u32)> {
    let c = &b.node.control;
    let mut result = vec![];
    let rounded = |rect, radius| RoundedRect::from_rect(rect, radius).to_path(0.1);
    if let Some(rect) = indicator_bounds(b) {
        let checked = c.checked.unwrap_or(false);
        let accent = if c.disabled { 0x65748b } else { 0xb5f36b };
        let border = if c.disabled { 0x52647b } else { 0x95a4bd };
        let fill = b.node.style.background.unwrap_or(0x182230);
        match b.node.role {
            Some("switch") => {
                result.push((rounded(rect, 9.), if checked { accent } else { border }));
                let knob_x = rect.x0 + if checked { 19. } else { 3. };
                result.push((
                    rounded(
                        Rect::new(knob_x, rect.y0 + 3., knob_x + 12., rect.y1 - 3.),
                        6.,
                    ),
                    0xf1f5ff,
                ));
            }
            Some("radio" | "menuitemradio") => {
                result.push((rounded(rect, 9.), if checked { accent } else { border }));
                result.push((rounded(rect.inset(-2.), 7.), fill));
                if checked {
                    result.push((rounded(rect.inset(-5.), 4.), accent));
                }
            }
            _ => {
                result.push((rounded(rect, 3.), if checked { accent } else { border }));
                if checked {
                    let mut check = BezPath::new();
                    for (i, (x, y)) in [
                        (3., 8.),
                        (7., 12.),
                        (14., 4.),
                        (16., 6.),
                        (7., 16.),
                        (1., 10.),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let point = (rect.x0 + x, rect.y0 + y);
                        if i == 0 {
                            check.move_to(point);
                        } else {
                            check.line_to(point);
                        }
                    }
                    check.close_path();
                    result.push((check, 0x182230));
                } else {
                    result.push((rounded(rect.inset(-2.), 1.), fill));
                }
            }
        }
    }
    if matches!(b.node.role, Some("slider" | "progressbar")) {
        let value = b.node.text.trim_end_matches('%').parse::<f64>().ok();
        let min = c.min.unwrap_or(0.);
        let max = c.max.unwrap_or(100.).max(min + f64::EPSILON);
        let ratio = value.map_or(0.3, |v| ((v - min) / (max - min)).clamp(0., 1.));
        let x = b.rect.x0 + 8.;
        let y = (b.rect.y1 - 6.).max(b.rect.y0);
        let width = (b.rect.width() - 16.).max(1.);
        result.push((rounded(Rect::new(x, y, x + width, y + 3.), 0.), 0x52647b));
        if ratio > 0. {
            result.push((
                rounded(Rect::new(x, y, x + width * ratio, y + 3.), 0.),
                0xb5f36b,
            ));
        }
        if b.node.role == Some("slider") {
            let at = x + width * ratio;
            result.push((
                rounded(Rect::new(at - 4., y - 4., at + 4., y + 7.), 0.),
                0xf1f5ff,
            ));
        }
    }
    result
}
