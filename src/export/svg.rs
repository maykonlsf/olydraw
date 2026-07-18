use std::fmt::Write;

use crate::scene::{Element, ElementKind, Point, Scene, ShapeKind, Style};

use super::arrowhead_wings;

/// Serializes the scene to a standalone SVG document (annotation only).
pub fn scene_to_svg(scene: &Scene) -> String {
    let (min, max) = bounds(scene);
    let (w, h) = ((max.x - min.x).max(1.0), (max.y - min.y).max(1.0));
    let mut out = String::new();
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\">",
        num(min.x),
        num(min.y),
        num(w),
        num(h),
        num(w),
        num(h),
    );
    for e in scene.elements() {
        write_element(&mut out, e);
    }
    out.push_str("</svg>\n");
    out
}

fn write_element(out: &mut String, e: &Element) {
    let stroke = stroke_attrs(&e.style);
    match &e.kind {
        ElementKind::Freehand { points } => {
            if points.is_empty() {
                return;
            }
            let _ = writeln!(
                out,
                "  <path d=\"{}\" fill=\"none\" {stroke}/>",
                path_d(points)
            );
        }
        ElementKind::Shape { kind, start, end } => {
            let (min, max) = normalize(*start, *end);
            match kind {
                ShapeKind::Rect => {
                    let _ = writeln!(
                        out,
                        "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" {stroke}/>",
                        num(min.x),
                        num(min.y),
                        num(max.x - min.x),
                        num(max.y - min.y),
                    );
                }
                ShapeKind::Ellipse => {
                    let _ = writeln!(
                        out,
                        "  <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" {stroke}/>",
                        num((min.x + max.x) / 2.0),
                        num((min.y + max.y) / 2.0),
                        num((max.x - min.x) / 2.0),
                        num((max.y - min.y) / 2.0),
                    );
                }
                ShapeKind::Diamond => {
                    let (cx, cy) = ((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
                    let _ = writeln!(
                        out,
                        "  <polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"none\" {stroke}/>",
                        num(cx),
                        num(min.y),
                        num(max.x),
                        num(cy),
                        num(cx),
                        num(max.y),
                        num(min.x),
                        num(cy),
                    );
                }
                ShapeKind::Line => write_line(out, *start, *end, &stroke),
                ShapeKind::Arrow => {
                    write_line(out, *start, *end, &stroke);
                    let [w1, w2] = arrowhead_wings(*start, *end, e.style.stroke_width);
                    let _ = writeln!(
                        out,
                        "  <polyline points=\"{},{} {},{} {},{}\" fill=\"none\" {stroke}/>",
                        num(w1.x),
                        num(w1.y),
                        num(end.x),
                        num(end.y),
                        num(w2.x),
                        num(w2.y),
                    );
                }
            }
        }
        ElementKind::Text { pos, content, size } => {
            let _ = writeln!(
                out,
                "  <text x=\"{}\" y=\"{}\" font-size=\"{}\" font-family=\"sans-serif\" fill=\"{}\"{}>{}</text>",
                num(pos.x),
                num(pos.y + size),
                num(*size),
                hex(e.style.color),
                fill_opacity(e.style.color),
                escape(content),
            );
        }
    }
}

fn write_line(out: &mut String, a: Point, b: Point, stroke: &str) {
    let _ = writeln!(
        out,
        "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {stroke}/>",
        num(a.x),
        num(a.y),
        num(b.x),
        num(b.y),
    );
}

fn path_d(points: &[Point]) -> String {
    let mut d = format!("M {} {}", num(points[0].x), num(points[0].y));
    for p in &points[1..] {
        let _ = write!(d, " L {} {}", num(p.x), num(p.y));
    }
    d
}

fn stroke_attrs(style: &Style) -> String {
    let mut s = format!(
        "stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"",
        hex(style.color),
        num(style.stroke_width),
    );
    if style.color[3] < 255 {
        let _ = write!(
            s,
            " stroke-opacity=\"{}\"",
            num(style.color[3] as f32 / 255.0)
        );
    }
    s
}

fn fill_opacity(color: [u8; 4]) -> String {
    if color[3] < 255 {
        format!(" fill-opacity=\"{}\"", num(color[3] as f32 / 255.0))
    } else {
        String::new()
    }
}

fn hex(c: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Formats without a trailing ".0" so whole numbers stay compact.
fn num(v: f32) -> String {
    if v.fract() == 0.0 && v.abs() < 1e7 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn normalize(a: Point, b: Point) -> (Point, Point) {
    (
        Point::new(a.x.min(b.x), a.y.min(b.y)),
        Point::new(a.x.max(b.x), a.y.max(b.y)),
    )
}

fn bounds(scene: &Scene) -> (Point, Point) {
    let mut min = Point::new(f32::INFINITY, f32::INFINITY);
    let mut max = Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut grow = |p: Point, margin: f32| {
        min.x = min.x.min(p.x - margin);
        min.y = min.y.min(p.y - margin);
        max.x = max.x.max(p.x + margin);
        max.y = max.y.max(p.y + margin);
    };
    for e in scene.elements() {
        let m = e.style.stroke_width;
        match &e.kind {
            ElementKind::Freehand { points } => points.iter().for_each(|p| grow(*p, m)),
            ElementKind::Shape { start, end, .. } => {
                grow(*start, m);
                grow(*end, m);
            }
            ElementKind::Text { pos, content, size } => {
                grow(*pos, 0.0);
                let width = content.chars().count() as f32 * size * 0.6;
                grow(Point::new(pos.x + width, pos.y + size * 1.2), 0.0);
            }
        }
    }
    if scene.elements().is_empty() {
        (Point::new(0.0, 0.0), Point::new(1.0, 1.0))
    } else {
        (min, max)
    }
}
