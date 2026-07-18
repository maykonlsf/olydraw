use olydraw::scene::Point;
use olydraw::tools::freehand::decimate;

fn p(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

fn dist_point_to_segment(pt: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return ((pt.x - a.x).powi(2) + (pt.y - a.y).powi(2)).sqrt();
    }
    let t = (((pt.x - a.x) * dx + (pt.y - a.y) * dy) / len2).clamp(0.0, 1.0);
    let (px, py) = (a.x + t * dx, a.y + t * dy);
    ((pt.x - px).powi(2) + (pt.y - py).powi(2)).sqrt()
}

fn dist_point_to_polyline(pt: Point, line: &[Point]) -> f32 {
    line.windows(2)
        .map(|w| dist_point_to_segment(pt, w[0], w[1]))
        .fold(f32::INFINITY, f32::min)
}

#[test]
fn noisy_straight_line_collapses_to_few_points() {
    let input: Vec<Point> = (0..1000)
        .map(|i| {
            let x = i as f32 * 0.5;
            let jitter = if i % 2 == 0 { 0.4 } else { -0.4 };
            p(x, 100.0 + jitter)
        })
        .collect();
    let out = decimate(&input, 1.5);
    assert!(
        out.len() <= 10,
        "near-straight noisy stroke should collapse, got {} points",
        out.len()
    );
}

#[test]
fn endpoints_are_preserved_exactly() {
    let input: Vec<Point> = (0..100)
        .map(|i| p(i as f32, (i as f32 * 0.3).sin() * 20.0))
        .collect();
    let out = decimate(&input, 1.5);
    assert_eq!(out.first(), input.first().copied().as_ref());
    assert_eq!(out.last(), input.last().copied().as_ref());
}

#[test]
fn simplified_path_stays_within_epsilon_of_original() {
    let epsilon = 1.5;
    let input: Vec<Point> = (0..500)
        .map(|i| {
            let t = i as f32 * 0.02;
            p(t * 40.0, (t * 2.0).sin() * 60.0)
        })
        .collect();
    let out = decimate(&input, epsilon);
    assert!(out.len() < input.len(), "curve should be simplified");
    for pt in &input {
        let d = dist_point_to_polyline(*pt, &out);
        assert!(
            d <= epsilon + 1e-3,
            "original point ({}, {}) deviates {}px from simplified path",
            pt.x,
            pt.y,
            d
        );
    }
}

#[test]
fn sharp_corner_is_retained() {
    let mut input: Vec<Point> = (0..=50).map(|i| p(i as f32, 0.0)).collect();
    input.extend((1..=50).map(|i| p(50.0, i as f32)));
    let out = decimate(&input, 1.5);
    let corner = p(50.0, 0.0);
    assert!(
        out.iter()
            .any(|q| (q.x - corner.x).abs() < 1e-3 && (q.y - corner.y).abs() < 1e-3),
        "corner point must survive simplification"
    );
}

#[test]
fn two_point_stroke_is_unchanged() {
    let input = vec![p(0.0, 0.0), p(10.0, 10.0)];
    assert_eq!(decimate(&input, 1.5), input);
}

#[test]
fn single_point_and_empty_input_pass_through() {
    assert_eq!(decimate(&[p(3.0, 4.0)], 1.5), vec![p(3.0, 4.0)]);
    assert_eq!(decimate(&[], 1.5), Vec::<Point>::new());
}
