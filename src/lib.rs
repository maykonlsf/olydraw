pub mod app;
pub mod capture;
pub mod export;
pub mod hotkey;
pub mod ipc;
pub mod laser;
pub mod prefs;
pub mod render;
pub mod scene;
pub mod tools;
pub mod ui;

#[cfg(target_os = "macos")]
pub mod platform_macos;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub mod tray;

/// Per-user IPC socket name (single-instance guard + `olydraw toggle`).
pub fn socket_name() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "default".to_string());
    format!("olydraw-{user}")
}
