use egui::{Align2, Color32, FontId, Painter, Pos2, Stroke, Vec2};

use crate::export::arrowhead_wings;
use crate::scene::{Element, ElementKind, Point, ShapeKind};

pub fn color32(c: [u8; 4], alpha_mul: f32) -> Color32 {
    let a = (c[3] as f32 * alpha_mul) as u8;
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], a)
}

fn pos(p: Point, offset: Vec2) -> Pos2 {
    Pos2::new(p.x + offset.x, p.y + offset.y)
}

/// Paints one scene element. `offset` shifts it (live move preview);
/// `alpha_mul` fades it (eraser preview).
pub fn paint_element(painter: &Painter, e: &Element, offset: Vec2, alpha_mul: f32) {
    let color = color32(e.style.color, alpha_mul);
    let stroke = Stroke::new(e.style.stroke_width, color);
    match &e.kind {
        ElementKind::Freehand { points } => {
            if points.len() == 1 {
                painter.circle_filled(pos(points[0], offset), e.style.stroke_width / 2.0, color);
            } else {
                let pts: Vec<Pos2> = points.iter().map(|p| pos(*p, offset)).collect();
                painter.add(egui::Shape::line(pts, stroke));
            }
        }
        ElementKind::Shape { kind, start, end } => {
            paint_shape(painter, *kind, *start, *end, stroke, offset);
        }
        ElementKind::Text {
            pos: p,
            content,
            size,
        } => {
            painter.text(
                pos(*p, offset),
                Align2::LEFT_TOP,
                content,
                FontId::proportional(*size),
                color,
            );
        }
    }
}

pub fn paint_shape(
    painter: &Painter,
    kind: ShapeKind,
    start: Point,
    end: Point,
    stroke: Stroke,
    offset: Vec2,
) {
    let a = pos(start, offset);
    let b = pos(end, offset);
    let rect = egui::Rect::from_two_pos(a, b);
    match kind {
        ShapeKind::Rect => {
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Middle);
        }
        ShapeKind::Ellipse => {
            painter.add(egui::epaint::EllipseShape {
                center: rect.center(),
                radius: rect.size() / 2.0,
                fill: Color32::TRANSPARENT,
                stroke,
                angle: 0.0,
            });
        }
        ShapeKind::Diamond => {
            let c = rect.center();
            let pts = vec![
                Pos2::new(c.x, rect.top()),
                Pos2::new(rect.right(), c.y),
                Pos2::new(c.x, rect.bottom()),
                Pos2::new(rect.left(), c.y),
            ];
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
        ShapeKind::Line => {
            painter.line_segment([a, b], stroke);
        }
        ShapeKind::Arrow => {
            painter.line_segment([a, b], stroke);
            let [w1, w2] = arrowhead_wings(start, end, stroke.width);
            painter.add(egui::Shape::line(
                vec![pos(w1, offset), b, pos(w2, offset)],
                stroke,
            ));
        }
    }
}

/// Selection highlight: dashed bounding box around an element.
pub fn paint_selection_box(painter: &Painter, e: &Element, offset: Vec2) {
    let bb = element_bounds(e);
    let rect = egui::Rect::from_two_pos(pos(bb.min, offset), pos(bb.max, offset)).expand(6.0);
    let stroke = Stroke::new(1.5, Color32::from_rgb(64, 128, 255));
    painter.add(egui::Shape::dashed_line(
        &[
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
            rect.left_top(),
        ],
        stroke,
        6.0,
        4.0,
    ));
}

pub fn element_bounds(e: &Element) -> crate::scene::Rect {
    use crate::scene::Rect;
    match &e.kind {
        ElementKind::Freehand { points } => {
            let mut min = Point::new(f32::INFINITY, f32::INFINITY);
            let mut max = Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
            for p in points {
                min.x = min.x.min(p.x);
                min.y = min.y.min(p.y);
                max.x = max.x.max(p.x);
                max.y = max.y.max(p.y);
            }
            Rect::new(min, max)
        }
        ElementKind::Shape { start, end, .. } => Rect::new(
            Point::new(start.x.min(end.x), start.y.min(end.y)),
            Point::new(start.x.max(end.x), start.y.max(end.y)),
        ),
        ElementKind::Text { pos, content, size } => {
            let lines: Vec<&str> = content.lines().collect();
            let max_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            Rect::new(
                *pos,
                Point::new(
                    pos.x + max_chars as f32 * size * 0.6,
                    pos.y + lines.len().max(1) as f32 * size * 1.2,
                ),
            )
        }
    }
}
