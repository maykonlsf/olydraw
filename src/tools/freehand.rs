use crate::scene::Point;

/// Simplifies a polyline, keeping endpoints exact and every retained point
/// within `epsilon` of the original path (Ramer–Douglas–Peucker).
pub fn decimate(points: &[Point], epsilon: f32) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    rdp(points, 0, points.len() - 1, epsilon, &mut keep);
    points
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(p, _)| *p)
        .collect()
}

fn rdp(points: &[Point], first: usize, last: usize, epsilon: f32, keep: &mut [bool]) {
    if last <= first + 1 {
        return;
    }
    let (a, b) = (points[first], points[last]);
    let mut max_dist = 0.0f32;
    let mut max_idx = first;
    for (i, p) in points.iter().enumerate().take(last).skip(first + 1) {
        let d = dist_point_segment(*p, a, b);
        if d > max_dist {
            max_dist = d;
            max_idx = i;
        }
    }
    if max_dist > epsilon {
        keep[max_idx] = true;
        rdp(points, first, max_idx, epsilon, keep);
        rdp(points, max_idx, last, epsilon, keep);
    }
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
