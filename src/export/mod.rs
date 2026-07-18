pub mod png;
pub mod svg;

use std::path::PathBuf;

use crate::prefs::Prefs;
use crate::scene::Point;

/// Destination directory for exports: the configured one, else
/// `~/Pictures/olydraw`, else `~/olydraw`.
pub fn export_dir(prefs: &Prefs) -> Result<PathBuf, String> {
    if let Some(dir) = &prefs.export_dir {
        return Ok(PathBuf::from(dir));
    }
    directories::UserDirs::new()
        .and_then(|d| {
            d.picture_dir()
                .map(|p| p.join("olydraw"))
                .or_else(|| Some(d.home_dir().join("olydraw")))
        })
        .ok_or_else(|| "no home directory found".to_string())
}

/// Writes bytes to a timestamped file in `dir`, returning the path.
pub fn write_bytes(dir: &std::path::Path, ext: &str, bytes: &[u8]) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let path = dir.join(format!("olydraw-{ts}.{ext}"));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

/// Saves an RGBA image as a timestamped PNG in `dir`, returning the path.
pub fn save_image(img: &image::RgbaImage, dir: &std::path::Path) -> Result<String, String> {
    let mut bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .map_err(|e| e.to_string())?;
    write_bytes(dir, "png", &bytes)
}

/// The two back-swept arrowhead wing tips for an arrow ending at `end`.
pub fn arrowhead_wings(start: Point, end: Point, stroke_width: f32) -> [Point; 2] {
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (ux, uy) = (dx / len, dy / len);
    let size = (stroke_width * 4.0).max(10.0).min(len * 0.5);
    let angle = 30f32.to_radians();
    let (sin, cos) = (angle.sin(), angle.cos());
    let wing = |s: f32| {
        Point::new(
            end.x - size * (ux * cos - s * uy * sin),
            end.y - size * (uy * cos + s * ux * sin),
        )
    };
    [wing(1.0), wing(-1.0)]
}
