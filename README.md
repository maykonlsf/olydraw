# olydraw

Lightweight screen-overlay drawing tool for meetings and presentations. Draw Excalidraw-style annotations on top of your live screen, highlight with a laser pointer, and export the result — all from a single small native binary (Rust + egui, no Electron).

## Install / run

```sh
cargo build --release
./target/release/olydraw          # starts the overlay (single instance)
./target/release/olydraw toggle   # shows/hides the running instance
```

The app stays resident; toggle it with the global hotkey (default **Ctrl/Cmd+Shift+D**, configurable in ⚙ Settings) or the tray icon (macOS/Windows).

## Tools & shortcuts

| Key | Tool |
|-----|------|
| `V` / `1` | Select (click, drag-move, marquee) |
| `R` / `2` | Rectangle |
| `D` / `3` | Diamond |
| `O` / `4` | Ellipse |
| `A` / `5` | Arrow |
| `L` / `6` | Line |
| `P` / `7` | Freehand |
| `T` / `8` | Text |
| `E` / `0` | Eraser |
| `K` | Laser pointer |

Actions: `Ctrl/Cmd+Z` undo · `Ctrl/Cmd+Shift+Z` redo · `Ctrl/Cmd+E` export · `Delete` remove selection · `C` cycle palette colors · `[` `]` stroke width · `Shift`-drag constrains (square/circle/45°) · `Esc` cancel → deselect → hide overlay.

Export (Ctrl/Cmd+E): annotation-only PNG/SVG, copy PNG to clipboard, or screen+annotation composite PNG. Files land in `~/Pictures/olydraw` (configurable). Preferences (color, stroke width, palette, hotkey, export dir) persist across sessions.

## Platform notes

- **macOS**: composite export needs the one-time Screen Recording permission. The overlay joins all Spaces and shows over fullscreen apps.
- **Windows / Linux X11**: full support.
- **Linux Wayland**: no app-defined global hotkeys — bind a system shortcut to `olydraw toggle` instead (e.g. GNOME Settings → Keyboard → Custom Shortcuts). The overlay runs as a regular borderless window; always-on-top behavior depends on your compositor. Screenshot capture goes through the XDG portal (system prompt).

## Development

```sh
cargo test          # core test suites (scene, hit-testing, export, prefs, IPC)
cargo clippy --all-targets -- -D warnings
BLESS=1 cargo test --test png_export   # regenerate golden images
```

See `docs/` for architecture and the manual test checklist.
