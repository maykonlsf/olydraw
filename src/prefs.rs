use std::path::Path;

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "prefs.toml";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub color: [u8; 4],
    pub stroke_width: f32,
    pub palette: Vec<[u8; 4]>,
    pub hotkey: String,
    pub export_dir: Option<String>,
    pub export_mode: ExportMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportMode {
    AnnotationOnly,
    ScreenComposite,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            color: [30, 30, 30, 255],
            stroke_width: 2.0,
            palette: vec![
                [30, 30, 30, 255],   // near-black
                [224, 49, 49, 255],  // red
                [47, 158, 68, 255],  // green
                [25, 113, 194, 255], // blue
                [245, 159, 0, 255],  // orange
                [255, 255, 255, 255],
            ],
            hotkey: "Ctrl+Shift+D".to_string(),
            export_dir: None,
            export_mode: ExportMode::AnnotationOnly,
        }
    }
}

/// Loads prefs from `dir/prefs.toml`. Missing or corrupt file returns defaults.
pub fn load_from(dir: &Path) -> Prefs {
    let Ok(text) = std::fs::read_to_string(dir.join(FILE_NAME)) else {
        return Prefs::default();
    };
    toml::from_str(&text).unwrap_or_default()
}

/// Saves prefs to `dir/prefs.toml`, creating the directory if needed.
pub fn save_to(dir: &Path, prefs: &Prefs) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let text = toml::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(FILE_NAME), text).map_err(|e| e.to_string())
}
