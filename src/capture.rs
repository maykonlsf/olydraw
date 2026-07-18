use image::RgbaImage;

use crate::export::png::scene_to_rgba_scaled;
use crate::scene::Scene;

/// Captures the monitor containing `at` (global logical point; the overlay's
/// monitor) and composites the scene on top, matching physical pixels via the
/// monitor scale factor. Falls back to the primary monitor when `at` is None.
/// The overlay window must already be hidden when this is called.
pub fn composite_with_screen(scene: &Scene, at: Option<(i32, i32)>) -> Result<RgbaImage, String> {
    let (x, y) = at.unwrap_or((0, 0));
    let monitor = xcap::Monitor::from_point(x, y)
        .or_else(|_| {
            xcap::Monitor::all().and_then(|mut all| {
                if all.is_empty() {
                    Err(xcap::XCapError::new("no monitors found"))
                } else {
                    Ok(all.remove(0))
                }
            })
        })
        .map_err(|e| e.to_string())?;
    let mut screen = monitor.capture_image().map_err(|e| e.to_string())?;
    let scale = monitor.scale_factor().map_err(|e| e.to_string())?;

    let annotation = scene_to_rgba_scaled(scene, screen.width(), screen.height(), scale)?;
    alpha_blend_onto(&mut screen, &annotation);
    Ok(screen)
}

fn alpha_blend_onto(bg: &mut RgbaImage, fg: &RgbaImage) {
    for (bg_px, fg_px) in bg.pixels_mut().zip(fg.pixels()) {
        let a = fg_px.0[3] as u32;
        if a == 0 {
            continue;
        }
        for c in 0..3 {
            let blended = (fg_px.0[c] as u32 * a + bg_px.0[c] as u32 * (255 - a)) / 255;
            bg_px.0[c] = blended as u8;
        }
        bg_px.0[3] = 255;
    }
}
