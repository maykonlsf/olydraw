use olydraw::export::png::scene_to_png;
use olydraw::scene::{ElementKind, Point, Scene, ShapeKind, Style};

fn p(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

fn decode(bytes: &[u8]) -> image::RgbaImage {
    image::load_from_memory(bytes)
        .expect("valid PNG")
        .to_rgba8()
}

#[test]
fn output_has_requested_dimensions_and_transparent_background() {
    let png = scene_to_png(&Scene::new(), 64, 48).expect("export");
    let img = decode(&png);
    assert_eq!((img.width(), img.height()), (64, 48));
    assert!(
        img.pixels().all(|px| px.0[3] == 0),
        "empty scene must be fully transparent"
    );
}

#[test]
fn rect_outline_paints_edge_pixels_but_not_interior() {
    let mut scene = Scene::new();
    scene.add(
        ElementKind::Shape {
            kind: ShapeKind::Rect,
            start: p(10.0, 10.0),
            end: p(50.0, 50.0),
        },
        Style {
            color: [255, 0, 0, 255],
            stroke_width: 4.0,
        },
    );
    let img = decode(&scene_to_png(&scene, 64, 64).expect("export"));
    let edge = img.get_pixel(30, 10);
    assert!(edge.0[3] > 0, "top edge must be painted");
    assert!(
        edge.0[0] > edge.0[1] && edge.0[0] > edge.0[2],
        "edge must be red-dominant, got {:?}",
        edge.0
    );
    assert_eq!(img.get_pixel(30, 30).0[3], 0, "interior stays transparent");
    assert_eq!(img.get_pixel(2, 2).0[3], 0, "outside stays transparent");
}

#[test]
fn freehand_stroke_paints_along_its_path() {
    let mut scene = Scene::new();
    scene.add(
        ElementKind::Freehand {
            points: (5..60).map(|x| p(x as f32, 32.0)).collect(),
        },
        Style {
            color: [0, 0, 255, 255],
            stroke_width: 3.0,
        },
    );
    let img = decode(&scene_to_png(&scene, 64, 64).expect("export"));
    assert!(img.get_pixel(32, 32).0[3] > 0, "pixel on stroke path");
    assert_eq!(img.get_pixel(32, 5).0[3], 0, "pixel far from path");
}

#[test]
fn text_renders_visible_pixels_near_its_position() {
    let mut scene = Scene::new();
    scene.add(
        ElementKind::Text {
            pos: p(5.0, 5.0),
            content: "Hi".to_string(),
            size: 24.0,
        },
        Style {
            color: [0, 0, 0, 255],
            stroke_width: 2.0,
        },
    );
    let img = decode(&scene_to_png(&scene, 96, 64).expect("export"));
    let painted = img
        .enumerate_pixels()
        .filter(|(x, y, px)| *x < 60 && *y < 45 && px.0[3] > 0)
        .count();
    assert!(painted > 20, "text glyphs must paint pixels, got {painted}");
    let stray = img
        .enumerate_pixels()
        .filter(|(_, y, px)| *y > 50 && px.0[3] > 0)
        .count();
    assert_eq!(stray, 0, "no pixels far below the text");
}

/// Golden regression: compares against a committed reference image.
/// Regenerate with: BLESS=1 cargo test --test png_export
#[test]
fn golden_scene_matches_reference() {
    let mut scene = Scene::new();
    let style = Style {
        color: [220, 40, 40, 255],
        stroke_width: 3.0,
    };
    scene.add(
        ElementKind::Shape {
            kind: ShapeKind::Rect,
            start: p(8.0, 8.0),
            end: p(56.0, 40.0),
        },
        style,
    );
    scene.add(
        ElementKind::Shape {
            kind: ShapeKind::Arrow,
            start: p(16.0, 56.0),
            end: p(56.0, 56.0),
        },
        Style {
            color: [40, 40, 220, 255],
            stroke_width: 3.0,
        },
    );
    let img = decode(&scene_to_png(&scene, 64, 64).expect("export"));

    let golden_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/shapes.png");
    if std::env::var("BLESS").is_ok() {
        std::fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        img.save(&golden_path).unwrap();
        return;
    }
    let golden = image::open(&golden_path)
        .unwrap_or_else(|_| panic!("golden missing — run BLESS=1 cargo test --test png_export"))
        .to_rgba8();
    assert_eq!(
        (img.width(), img.height()),
        (golden.width(), golden.height())
    );
    let tolerance = 2i16;
    for (a, b) in img.pixels().zip(golden.pixels()) {
        for c in 0..4 {
            assert!(
                (a.0[c] as i16 - b.0[c] as i16).abs() <= tolerance,
                "pixel channel diff beyond tolerance: {:?} vs {:?}",
                a.0,
                b.0
            );
        }
    }
}
