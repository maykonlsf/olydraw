# Tech Stack

- **Language:** Rust (edition 2024, stable toolchain)
- **UI/windowing:** `eframe`/`egui` 0.35, glow (OpenGL) backend, transparent always-on-top borderless viewport
- **Global hotkey:** `global-hotkey` (macOS/Windows/X11; Wayland uses the IPC CLI instead)
- **Tray:** `tray-icon` (macOS/Windows only)
- **Screen capture:** `xcap` (composite export)
- **Raster export:** `tiny-skia` (+ `ab_glyph` for text, reusing egui's embedded font)
- **Vector export:** hand-rolled SVG serialization (no dependency)
- **IPC / single instance:** `interprocess` local sockets — fs path in temp dir on Unix, named pipe on Windows
- **Prefs:** `serde` + `toml` + `directories` (`~/Library/Application Support/com.olydraw.olydraw` etc.)
- **Clipboard:** `arboard`; **dialogs:** `rfd`
- **macOS specifics:** `objc2-app-kit` (window level, all-Spaces collection behavior)

CI: GitHub Actions matrix (macOS/Windows/Ubuntu) — fmt, clippy `-D warnings`, tests.
