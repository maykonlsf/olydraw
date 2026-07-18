use ab_glyph::{Font, FontRef, ScaleFont};
use tiny_skia::{
    LineCap, LineJoin, Paint, PathBuilder, Pixmap, PremultipliedColorU8, Stroke, Transform,
};

use crate::scene::{Element, ElementKind, Point, Scene, ShapeKind};

use super::arrowhead_wings;

/// Rasterizes the scene onto a transparent canvas of the given size and
/// returns encoded PNG bytes. Coordinates map 1:1 to pixels.
pub fn scene_to_png(scene: &Scene, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let pixmap = rasterize(scene, width, height, 1.0)?;
    pixmap.encode_png().map_err(|e| e.to_string())
}

/// Rasterizes the scene at `scale` (logical → physical pixels) onto a
/// transparent canvas of `width`×`height` physical pixels, returned as
/// straight-alpha RGBA (for compositing onto a screenshot).
pub fn scene_to_rgba_scaled(
    scene: &Scene,
    width: u32,
    height: u32,
    scale: f32,
) -> Result<image::RgbaImage, String> {
    let pixmap = rasterize(scene, width, height, scale)?;
    let mut out = image::RgbaImage::new(width, height);
    for (px, dst) in pixmap.pixels().iter().zip(out.pixels_mut()) {
        let c = px.demultiply();
        dst.0 = [c.red(), c.green(), c.blue(), c.alpha()];
    }
    Ok(out)
}

fn rasterize(scene: &Scene, width: u32, height: u32, scale: f32) -> Result<Pixmap, String> {
    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| "invalid canvas size".to_string())?;
    for e in scene.elements() {
        draw_element(&mut pixmap, e, scale)?;
    }
    Ok(pixmap)
}

fn draw_element(pixmap: &mut Pixmap, e: &Element, scale: f32) -> Result<(), String> {
    match &e.kind {
        ElementKind::Text { pos, content, size } => {
            draw_text(pixmap, *pos, content, *size, e, scale)
        }
        _ => {
            let Some(path) = element_path(e) else {
                return Ok(());
            };
            let mut paint = Paint::default();
            let [r, g, b, a] = e.style.color;
            paint.set_color_rgba8(r, g, b, a);
            paint.anti_alias = true;
            let stroke = Stroke {
                width: e.style.stroke_width,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            pixmap.stroke_path(
                &path,
                &paint,
                &stroke,
                Transform::from_scale(scale, scale),
                None,
            );
            Ok(())
        }
    }
}

fn element_path(e: &Element) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    match &e.kind {
        ElementKind::Freehand { points } => {
            let first = points.first()?;
            pb.move_to(first.x, first.y);
            for p in &points[1..] {
                pb.line_to(p.x, p.y);
            }
            // A tap renders as a dot rather than an empty path.
            if points.len() == 1 {
                pb.line_to(first.x + 0.1, first.y);
            }
        }
        ElementKind::Shape { kind, start, end } => {
            let (min, max) = normalize(*start, *end);
            match kind {
                ShapeKind::Rect => {
                    let rect = tiny_skia::Rect::from_ltrb(min.x, min.y, max.x, max.y)?;
                    pb.push_rect(rect);
                }
                ShapeKind::Ellipse => {
                    let rect = tiny_skia::Rect::from_ltrb(min.x, min.y, max.x, max.y)?;
                    pb.push_oval(rect);
                }
                ShapeKind::Diamond => {
                    let (cx, cy) = ((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
                    pb.move_to(cx, min.y);
                    pb.line_to(max.x, cy);
                    pb.line_to(cx, max.y);
                    pb.line_to(min.x, cy);
                    pb.close();
                }
                ShapeKind::Line => {
                    pb.move_to(start.x, start.y);
                    pb.line_to(end.x, end.y);
                }
                ShapeKind::Arrow => {
                    pb.move_to(start.x, start.y);
                    pb.line_to(end.x, end.y);
                    let [w1, w2] = arrowhead_wings(*start, *end, e.style.stroke_width);
                    pb.move_to(w1.x, w1.y);
                    pb.line_to(end.x, end.y);
                    pb.line_to(w2.x, w2.y);
                }
            }
        }
        ElementKind::Text { .. } => return None,
    }
    pb.finish()
}

fn draw_text(
    pixmap: &mut Pixmap,
    pos: Point,
    content: &str,
    size: f32,
    e: &Element,
    scale: f32,
) -> Result<(), String> {
    let pos = Point::new(pos.x * scale, pos.y * scale);
    let size = size * scale;
    let font_bytes = default_font_bytes();
    let font = FontRef::try_from_slice(&font_bytes).map_err(|e| e.to_string())?;
    let scaled = font.as_scaled(ab_glyph::PxScale::from(size));
    let [r, g, b, a] = e.style.color;
    let width = pixmap.width() as i32;
    let height = pixmap.height() as i32;

    let mut baseline_y = pos.y + scaled.ascent();
    for line in content.lines() {
        let mut x = pos.x;
        for ch in line.chars() {
            let glyph_id = scaled.glyph_id(ch);
            let glyph = glyph_id.with_scale_and_position(
                ab_glyph::PxScale::from(size),
                ab_glyph::point(x, baseline_y),
            );
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                let data = pixmap.pixels_mut();
                outlined.draw(|gx, gy, coverage| {
                    let px = bounds.min.x as i32 + gx as i32;
                    let py = bounds.min.y as i32 + gy as i32;
                    if px < 0 || py < 0 || px >= width || py >= height {
                        return;
                    }
                    let alpha = (coverage * a as f32) as u8;
                    if alpha == 0 {
                        return;
                    }
                    let mul = |c: u8| ((c as u16 * alpha as u16) / 255) as u8;
                    if let Some(c) = PremultipliedColorU8::from_rgba(mul(r), mul(g), mul(b), alpha)
                    {
                        data[(py * width + px) as usize] = c;
                    }
                });
            }
            x += scaled.h_advance(glyph_id);
        }
        baseline_y += scaled.height();
    }
    Ok(())
}

/// Reuses egui's embedded default proportional font so the binary carries
/// only one copy.
fn default_font_bytes() -> Vec<u8> {
    let defs = egui::FontDefinitions::default();
    let data = defs
        .font_data
        .get("Ubuntu-Light")
        .or_else(|| defs.font_data.values().next())
        .expect("egui default fonts present");
    data.font.to_vec()
}

fn normalize(a: Point, b: Point) -> (Point, Point) {
    (
        Point::new(a.x.min(b.x), a.y.min(b.y)),
        Point::new(a.x.max(b.x), a.y.max(b.y)),
    )
}
