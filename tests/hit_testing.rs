use olydraw::scene::{ElementKind, Point, Rect, Scene, ShapeKind, Style};

// stroke_width 2.0 -> expected hit tolerance = width/2 + 4 = 5px.
fn style() -> Style {
    Style {
        color: [0, 0, 0, 255],
        stroke_width: 2.0,
    }
}

fn p(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

fn shape(kind: ShapeKind, x1: f32, y1: f32, x2: f32, y2: f32) -> ElementKind {
    ElementKind::Shape {
        kind,
        start: p(x1, y1),
        end: p(x2, y2),
    }
}

#[test]
fn freehand_stroke_is_hit_on_and_near_the_path_only() {
    let mut scene = Scene::new();
    let points = (0..=100).map(|x| p(x as f32, 100.0)).collect();
    let a = scene.add(ElementKind::Freehand { points }, style());
    assert_eq!(scene.hit_top(p(50.0, 100.0)), Some(a), "exactly on path");
    assert_eq!(scene.hit_top(p(50.0, 104.0)), Some(a), "within tolerance");
    assert_eq!(scene.hit_top(p(50.0, 106.0)), None, "outside tolerance");
}

#[test]
fn rect_hits_on_outline_but_not_interior() {
    let mut scene = Scene::new();
    let a = scene.add(shape(ShapeKind::Rect, 10.0, 10.0, 110.0, 110.0), style());
    assert_eq!(scene.hit_top(p(60.0, 10.0)), Some(a), "top edge");
    assert_eq!(scene.hit_top(p(10.0, 60.0)), Some(a), "left edge");
    assert_eq!(scene.hit_top(p(60.0, 60.0)), None, "unfilled interior");
    assert_eq!(scene.hit_top(p(60.0, 120.0)), None, "outside");
}

#[test]
fn ellipse_hits_on_outline_but_not_center() {
    let mut scene = Scene::new();
    let a = scene.add(shape(ShapeKind::Ellipse, 10.0, 10.0, 110.0, 110.0), style());
    assert_eq!(scene.hit_top(p(110.0, 60.0)), Some(a), "rightmost point");
    assert_eq!(scene.hit_top(p(60.0, 10.0)), Some(a), "topmost point");
    assert_eq!(scene.hit_top(p(60.0, 60.0)), None, "center");
}

#[test]
fn diamond_hits_on_edges_but_not_center_or_corners_of_bbox() {
    let mut scene = Scene::new();
    let a = scene.add(shape(ShapeKind::Diamond, 0.0, 0.0, 100.0, 100.0), style());
    // Edge from top vertex (50,0) to right vertex (100,50): midpoint (75,25).
    assert_eq!(scene.hit_top(p(75.0, 25.0)), Some(a), "edge midpoint");
    assert_eq!(scene.hit_top(p(50.0, 50.0)), None, "center");
    assert_eq!(
        scene.hit_top(p(2.0, 2.0)),
        None,
        "bbox corner is outside diamond"
    );
}

#[test]
fn line_and_arrow_hit_along_segment_only() {
    let mut scene = Scene::new();
    let l = scene.add(shape(ShapeKind::Line, 0.0, 0.0, 100.0, 100.0), style());
    assert_eq!(scene.hit_top(p(50.0, 50.0)), Some(l));
    assert_eq!(scene.hit_top(p(60.0, 40.0)), None, "14px off the segment");
    let mut scene2 = Scene::new();
    let a = scene2.add(shape(ShapeKind::Arrow, 0.0, 100.0, 100.0, 100.0), style());
    assert_eq!(scene2.hit_top(p(50.0, 100.0)), Some(a));
    assert_eq!(scene2.hit_top(p(50.0, 110.0)), None);
}

#[test]
fn text_hits_inside_its_area_only() {
    let mut scene = Scene::new();
    let t = scene.add(
        ElementKind::Text {
            pos: p(10.0, 10.0),
            content: "Hello".to_string(),
            size: 16.0,
        },
        style(),
    );
    assert_eq!(scene.hit_top(p(12.0, 15.0)), Some(t), "inside text box");
    assert_eq!(scene.hit_top(p(10.0, 200.0)), None, "far below");
    assert_eq!(scene.hit_top(p(400.0, 15.0)), None, "far right");
}

#[test]
fn topmost_overlapping_element_wins() {
    let mut scene = Scene::new();
    let _bottom = scene.add(shape(ShapeKind::Line, 0.0, 50.0, 100.0, 50.0), style());
    let top = scene.add(shape(ShapeKind::Line, 50.0, 0.0, 50.0, 100.0), style());
    assert_eq!(scene.hit_top(p(50.0, 50.0)), Some(top));
}

#[test]
fn marquee_selects_intersecting_elements() {
    let mut scene = Scene::new();
    let inside = scene.add(shape(ShapeKind::Rect, 10.0, 10.0, 20.0, 20.0), style());
    let crossing = scene.add(shape(ShapeKind::Line, 0.0, 25.0, 200.0, 25.0), style());
    let outside = scene.add(shape(ShapeKind::Rect, 300.0, 300.0, 320.0, 320.0), style());
    let hits = scene.hit_rect(Rect::new(p(0.0, 0.0), p(50.0, 50.0)));
    assert!(hits.contains(&inside), "fully contained element");
    assert!(hits.contains(&crossing), "partially intersecting element");
    assert!(!hits.contains(&outside), "element fully outside");
}

#[test]
fn eraser_segment_hits_crossed_elements_only() {
    let mut scene = Scene::new();
    let crossed = scene.add(shape(ShapeKind::Line, 0.0, 100.0, 100.0, 100.0), style());
    let missed = scene.add(shape(ShapeKind::Line, 0.0, 300.0, 100.0, 300.0), style());
    let hits = scene.hit_segment(p(50.0, 90.0), p(50.0, 110.0));
    assert!(hits.contains(&crossed));
    assert!(!hits.contains(&missed));
    let near_miss = scene.hit_segment(p(50.0, 110.0), p(50.0, 120.0));
    assert!(!near_miss.contains(&crossed), "segment stops 10px short");
}
