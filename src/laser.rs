use std::time::{Duration, Instant};

use egui::{Color32, Painter, Pos2, Stroke};

const TRAIL_LIFETIME: Duration = Duration::from_millis(700);

/// Ephemeral laser-pointer trail. Never enters the scene or undo history.
#[derive(Default)]
pub struct LaserTrail {
    points: Vec<(Pos2, Instant)>,
}

impl LaserTrail {
    pub fn push(&mut self, pos: Pos2) {
        self.points.push((pos, Instant::now()));
    }

    pub fn prune(&mut self) {
        let now = Instant::now();
        self.points.retain(|(_, t)| now - *t < TRAIL_LIFETIME);
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn paint(&self, painter: &Painter) {
        let now = Instant::now();
        let life = TRAIL_LIFETIME.as_secs_f32();
        for w in self.points.windows(2) {
            let (a, ta) = w[0];
            let (b, _) = w[1];
            let age = (now - ta).as_secs_f32();
            let fade = (1.0 - age / life).clamp(0.0, 1.0);
            let glow = Color32::from_rgba_unmultiplied(255, 60, 60, (90.0 * fade) as u8);
            let core = Color32::from_rgba_unmultiplied(255, 80, 80, (230.0 * fade) as u8);
            painter.line_segment([a, b], Stroke::new(9.0 * fade + 2.0, glow));
            painter.line_segment([a, b], Stroke::new(3.5 * fade + 1.0, core));
        }
        if let Some((head, _)) = self.points.last() {
            painter.circle_filled(*head, 5.0, Color32::from_rgb(255, 70, 70));
            painter.circle_filled(*head, 2.5, Color32::from_rgb(255, 220, 220));
        }
    }
}
