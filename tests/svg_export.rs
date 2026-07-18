use olydraw::export::svg::scene_to_svg;
use olydraw::scene::{ElementKind, Point, Scene, ShapeKind, Style};

fn style(color: [u8; 4], width: f32) -> Style {
    Style {
        color,
        stroke_width: width,
    }
}

fn p(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

#[test]
fn empty_scene_produces_valid_empty_svg() {
    let svg = scene_to_svg(&Scene::new());
    assert!(
        svg.trim_start().starts_with("<svg"),
        "starts with <svg: {svg}"
    );
    assert!(
        svg.trim_end().ends_with("</svg>"),
        "ends with </svg>: {svg}"
    );
    assert!(!svg.contains("<path"));
    assert!(!svg.contains("<text"));
}

#[test]
fn freehand_becomes_a_stroked_unfilled_path() {
    let mut scene = Scene::new();
    scene.add(
        ElementKind::Freehand {
            points: vec![p(0.0, 0.0), p(10.0, 5.0), p(20.0, 0.0)],
        },
        style([255, 0, 0, 255], 3.0),
    );
    let svg = scene_to_svg(&scene);
    assert!(svg.contains("<path"), "expected a <path>: {svg}");
    assert!(svg.contains("#ff0000"), "stroke color as hex: {svg}");
    assert!(svg.contains("stroke-width=\"3\""), "stroke width: {svg}");
    assert!(svg.contains("fill=\"none\""), "unfilled: {svg}");
}

#[test]
fn shapes_map_to_svg_primitives() {
    let mut scene = Scene::new();
    let s = style([0, 0, 255, 255], 2.0);
    scene.add(
        ElementKind::Shape {
            kind: ShapeKind::Rect,
            start: p(10.0, 10.0),
            end: p(50.0, 40.0),
        },
        s,
    );
    scene.add(
        ElementKind::Shape {
            kind: ShapeKind::Ellipse,
            start: p(0.0, 0.0),
            end: p(20.0, 20.0),
        },
        s,
    );
    scene.add(
        ElementKind::Shape {
            kind: ShapeKind::Line,
            start: p(0.0, 0.0),
            end: p(9.0, 9.0),
        },
        s,
    );
    let svg = scene_to_svg(&scene);
    assert!(svg.contains("<rect"), "{svg}");
    assert!(svg.contains("<ellipse"), "{svg}");
    assert!(svg.contains("<line"), "{svg}");
    assert!(svg.contains("#0000ff"), "{svg}");
}

#[test]
fn arrow_renders_a_visible_arrowhead() {
    let mut scene = Scene::new();
    scene.add(
        ElementKind::Shape {
            kind: ShapeKind::Arrow,
            start: p(0.0, 0.0),
            end: p(100.0, 0.0),
        },
        style([0, 0, 0, 255], 2.0),
    );
    let svg = scene_to_svg(&scene);
    // An arrow is more than a bare line: some head geometry must exist.
    let line_count = svg.matches("<line").count();
    let has_head = line_count >= 3 || svg.contains("<polyline") || svg.contains("<path");
    assert!(has_head, "arrowhead geometry missing: {svg}");
}

#[test]
fn text_content_is_escaped() {
    let mut scene = Scene::new();
    scene.add(
        ElementKind::Text {
            pos: p(10.0, 20.0),
            content: "A<&\"B".to_string(),
            size: 16.0,
        },
        style([0, 128, 0, 255], 2.0),
    );
    let svg = scene_to_svg(&scene);
    assert!(svg.contains("<text"), "{svg}");
    assert!(svg.contains("&lt;"), "'<' must be escaped: {svg}");
    assert!(svg.contains("&amp;"), "'&' must be escaped: {svg}");
    assert!(!svg.contains("A<&"), "raw unescaped content leaked: {svg}");
}

#[test]
fn semi_transparent_color_carries_opacity() {
    let mut scene = Scene::new();
    scene.add(
        ElementKind::Shape {
            kind: ShapeKind::Line,
            start: p(0.0, 0.0),
            end: p(10.0, 0.0),
        },
        style([255, 0, 0, 128], 2.0),
    );
    let svg = scene_to_svg(&scene);
    assert!(
        svg.contains("stroke-opacity") || svg.contains("rgba"),
        "alpha 128 must be expressed: {svg}"
    );
}
