//! Editing decorations use the same shaped clusters as visible Rust text.
use crate::{
    layout::BoxNode,
    text::{Shaped, TextEngine},
};
use vello::kurbo::Rect;

/// Nearest visual grapheme boundary on the nearest shaped line. Interpolate
/// within a multi-grapheme ligature while respecting its bidi direction.
pub fn hit_offset(shaped: &Shaped, content: &str, x: f64, y: f64) -> usize {
    use unicode_segmentation::UnicodeSegmentation;
    let distance = |line: &crate::text::TextLine| {
        if y < f64::from(line.y) {
            f64::from(line.y) - y
        } else {
            (y - f64::from(line.y + line.height)).max(0.)
        }
    };
    let Some(line) = shaped
        .lines
        .iter()
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
    else {
        return if y > 0. { content.len() } else { 0 };
    };
    let mut closest = None::<(f64, usize)>;
    for cluster in shaped.clusters.iter().filter(|cluster| cluster.y == line.y) {
        let Some(text) = content.get(cluster.start..cluster.end) else {
            continue;
        };
        let mut boundaries: Vec<_> = text.grapheme_indices(true).map(|(byte, _)| byte).collect();
        boundaries.push(text.len());
        let segments = boundaries.len().saturating_sub(1).max(1);
        for (index, byte) in boundaries.into_iter().enumerate() {
            let fraction = index as f64 / segments as f64;
            let edge = f64::from(cluster.x)
                + f64::from(cluster.width) * if cluster.rtl { 1. - fraction } else { fraction };
            let delta = (x - edge).abs();
            if closest.is_none_or(|(old, _)| delta < old) {
                closest = Some((delta, cluster.start + byte));
            }
        }
    }
    closest.map_or(line.start, |(_, byte)| byte)
}

/// A logical selection can have several visual rectangles in a bidi paragraph.
pub fn selection_rects(shaped: &Shaped, start: usize, end: usize) -> Vec<Rect> {
    if start == end {
        return vec![];
    }
    let (start, end) = (start.min(end), start.max(end));
    shaped
        .clusters
        .iter()
        .filter(|c| c.start < end && c.end > start)
        .map(|c| {
            Rect::new(
                c.x as f64,
                c.y as f64,
                (c.x + c.width) as f64,
                (c.y + c.height) as f64,
            )
        })
        .collect()
}

pub fn caret_rect(shaped: &Shaped, offset: usize, font_size: f32) -> Rect {
    if let Some(line) = shaped
        .lines
        .iter()
        .find(|line| line.start == offset && line.end == offset)
    {
        return Rect::new(0., f64::from(line.y), 1.5, f64::from(line.y + line.height));
    }
    let at = shaped
        .clusters
        .iter()
        .find(|c| c.start == offset)
        .map(|c| (if c.rtl { c.x + c.width } else { c.x }, c.y, c.height))
        .or_else(|| {
            shaped
                .clusters
                .iter()
                .rev()
                .find(|c| c.end <= offset)
                .map(|c| (if c.rtl { c.x } else { c.x + c.width }, c.y, c.height))
        })
        .unwrap_or((0., 0., font_size * 1.4));
    Rect::new(
        at.0 as f64,
        at.1 as f64,
        at.0 as f64 + 1.5,
        (at.1 + at.2) as f64,
    )
}

pub fn decorations(b: &BoxNode, text: &mut TextEngine) -> Vec<(Rect, u32, f32)> {
    let Some(editor) = &b.node.editing else {
        return vec![];
    };
    let s = &b.node.style;
    let shaped = text.shape_node(
        &b.node,
        (b.rect.width() as f32 - s.padding_horizontal()).max(1.),
    );
    let offset = vello::kurbo::Vec2::new(
        b.rect.x0 + s.padding_left() as f64,
        b.rect.y0 + s.padding_top() as f64 - f64::from(editor.scroll_y),
    );
    let mut rects = selection_rects(&shaped, editor.selection.anchor, editor.selection.focus)
        .into_iter()
        .map(|r| (r + offset, 0x78b4ff, 0.35))
        .collect::<Vec<_>>();
    if editor.selection.anchor == editor.selection.focus {
        rects.push((
            caret_rect(&shaped, editor.selection.focus, s.font_size) + offset,
            s.color,
            1.,
        ));
    }
    if editor.composing {
        let preedit = editor.preedit.unwrap_or(editor.selection);
        for r in selection_rects(&shaped, preedit.anchor, preedit.focus) {
            rects.push((
                Rect::new(r.x0, r.y1 - 2., r.x1, r.y1 - 1.) + offset,
                0xb5f36b,
                1.,
            ));
        }
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bidi_selection_uses_visual_cluster_rectangles() {
        let mut text = TextEngine::default();
        let shaped = text.shape("hello سلام 🌍", 20., 400.);
        let selected = selection_rects(&shaped, 6, "hello سلام".len());
        assert!(!selected.is_empty());
        assert!(selected.iter().all(|r| r.width() > 0. && r.height() > 0.));
        let caret = caret_rect(&shaped, "hello سلام 🌍".len(), 20.);
        assert!(caret.x0.is_finite() && caret.width() > 0.);
    }
}
