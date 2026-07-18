use super::element::{Element, ElementKind, Point, Rect, ShapeKind};

/// Extra slack around a stroke so thin lines stay clickable.
const BASE_TOLERANCE: f32 = 4.0;

fn tolerance(e: &Element) -> f32 {
    e.style.stroke_width / 2.0 + BASE_TOLERANCE
}

pub fn hits_point(e: &Element, p: Point) -> bool {
    match &e.kind {
        ElementKind::Text { .. } => text_bbox(e).is_some_and(|b| contains(b, p)),
        _ => outlines(e)
            .iter()
            .any(|line| dist_point_polyline(p, line) <= tolerance(e)),
    }
}

pub fn hits_rect(e: &Element, rect: Rect) -> bool {
    if let Some(b) = text_bbox(e) {
        return rects_intersect(b, rect);
    }
    outlines(e).iter().any(|line| {
        line.iter().any(|p| contains(rect, *p))
            || line
                .windows(2)
                .any(|w| segment_intersects_rect(w[0], w[1], rect))
    })
}

pub fn hits_segment(e: &Element, a: Point, b: Point) -> bool {
    if let Some(bb) = text_bbox(e) {
        return contains(bb, a) || contains(bb, b) || segment_intersects_rect(a, b, bb);
    }
    let tol = tolerance(e);
    outlines(e).iter().any(|line| {
        line.windows(2)
            .any(|w| dist_seg_seg(a, b, w[0], w[1]) <= tol)
    })
}

/// Element stroke geometry as one or more polylines.
fn outlines(e: &Element) -> Vec<Vec<Point>> {
    match &e.kind {
        ElementKind::Freehand { points } => vec![points.clone()],
        ElementKind::Text { .. } => Vec::new(),
        ElementKind::Shape { kind, start, end } => {
            let (min, max) = normalize(*start, *end);
            match kind {
                ShapeKind::Rect => vec![vec![
                    min,
                    Point::new(max.x, min.y),
                    max,
                    Point::new(min.x, max.y),
                    min,
                ]],
                ShapeKind::Diamond => {
                    let (cx, cy) = ((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
                    vec![vec![
                        Point::new(cx, min.y),
                        Point::new(max.x, cy),
                        Point::new(cx, max.y),
                        Point::new(min.x, cy),
                        Point::new(cx, min.y),
                    ]]
                }
                ShapeKind::Ellipse => {
                    let (cx, cy) = ((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
                    let (rx, ry) = ((max.x - min.x) / 2.0, (max.y - min.y) / 2.0);
                    let pts = (0..=64)
                        .map(|i| {
                            let t = i as f32 / 64.0 * std::f32::consts::TAU;
                            Point::new(cx + rx * t.cos(), cy + ry * t.sin())
                        })
                        .collect();
                    vec![pts]
                }
                ShapeKind::Line | ShapeKind::Arrow => vec![vec![*start, *end]],
            }
        }
    }
}

/// Approximate visual bounds of a text element (monospace-ish estimate).
fn text_bbox(e: &Element) -> Option<Rect> {
    let ElementKind::Text { pos, content, size } = &e.kind else {
        return None;
    };
    let lines: Vec<&str> = content.lines().collect();
    let max_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let width = max_chars as f32 * size * 0.6;
    let height = lines.len().max(1) as f32 * size * 1.2;
    Some(Rect::new(*pos, Point::new(pos.x + width, pos.y + height)))
}

fn normalize(a: Point, b: Point) -> (Point, Point) {
    (
        Point::new(a.x.min(b.x), a.y.min(b.y)),
        Point::new(a.x.max(b.x), a.y.max(b.y)),
    )
}

fn contains(r: Rect, p: Point) -> bool {
    p.x >= r.min.x && p.x <= r.max.x && p.y >= r.min.y && p.y <= r.max.y
}

fn rects_intersect(a: Rect, b: Rect) -> bool {
    a.min.x <= b.max.x && b.min.x <= a.max.x && a.min.y <= b.max.y && b.min.y <= a.max.y
}

fn dist_point_segment(p: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0);
    let (px, py) = (a.x + t * dx, a.y + t * dy);
    ((p.x - px).powi(2) + (p.y - py).powi(2)).sqrt()
}

fn dist_point_polyline(p: Point, line: &[Point]) -> f32 {
    if line.len() == 1 {
        return dist_point_segment(p, line[0], line[0]);
    }
    line.windows(2)
        .map(|w| dist_point_segment(p, w[0], w[1]))
        .fold(f32::INFINITY, f32::min)
}

fn segments_cross(a1: Point, a2: Point, b1: Point, b2: Point) -> bool {
    fn orient(a: Point, b: Point, c: Point) -> f32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }
    let (d1, d2) = (orient(b1, b2, a1), orient(b1, b2, a2));
    let (d3, d4) = (orient(a1, a2, b1), orient(a1, a2, b2));
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

fn dist_seg_seg(a1: Point, a2: Point, b1: Point, b2: Point) -> f32 {
    if segments_cross(a1, a2, b1, b2) {
        return 0.0;
    }
    dist_point_segment(a1, b1, b2)
        .min(dist_point_segment(a2, b1, b2))
        .min(dist_point_segment(b1, a1, a2))
        .min(dist_point_segment(b2, a1, a2))
}

fn segment_intersects_rect(a: Point, b: Point, r: Rect) -> bool {
    if contains(r, a) || contains(r, b) {
        return true;
    }
    let corners = [
        r.min,
        Point::new(r.max.x, r.min.y),
        r.max,
        Point::new(r.min.x, r.max.y),
    ];
    (0..4).any(|i| segments_cross(a, b, corners[i], corners[(i + 1) % 4]))
}
