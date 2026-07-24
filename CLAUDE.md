# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

olydraw is a cross-platform (macOS/Windows/Linux) screen-overlay annotation tool: a single resident Rust binary showing a transparent always-on-top egui window toggled by a global hotkey. No Electron/webview — lightness is a hard requirement.

## Commands

```sh
cargo test                             # all suites
cargo test --test scene                # one suite (tests/*.rs: scene, hit_testing, freehand, svg_export, png_export, prefs, ipc)
cargo test --test scene undo_and_redo_round_trip_add   # one test
BLESS=1 cargo test --test png_export   # regenerate golden images in tests/golden/
cargo clippy --all-targets -- -D warnings   # CI-enforced
cargo fmt --check                           # CI-enforced
cargo run                              # start overlay; `cargo run -- toggle` toggles a running instance
cargo run -- --hidden                  # start with no overlay shown (tray/background only; launch-at-login use)
```

CI runs test/clippy/fmt on macOS, Windows, and Ubuntu (`.github/workflows/ci.yml`). Rust edition 2024, stable toolchain (`rust-toolchain.toml`).

## Architecture

Two strictly separated layers inside one crate:

- **Headless core** (`scene/`, `tools/`, `export/`, `prefs.rs`, `ipc.rs`): no window, GPU, or egui-context dependency. All automated tests (`tests/*.rs`) target only this layer through its public API. New logic belongs here whenever possible, test-first.
- **App shell** (`main.rs`, `app.rs`, `ui.rs`, `render.rs`, `laser.rs`, `capture.rs`, `hotkey.rs`, `tray.rs`, `platform_macos.rs`): egui/eframe glue and OS-specific code. Not unit-tested; verified via `docs/manual-testing.md` per OS.

Load-bearing invariants:

- **Single mutation choke point**: tools and UI never edit elements directly — every change goes through `Scene`'s command API (`add`/`remove`/`translate`/`clear`), which is what makes undo/redo (`scene/history.rs`, capped at 100, one entry per user gesture) correct. A multi-step gesture (eraser drag, text re-edit) must still produce exactly one history entry.
- **Two rasterizers on purpose**: on-screen drawing uses egui's painter (`render.rs`), while PNG/SVG export re-renders the same geometry via tiny-skia/ab_glyph (`export/png.rs`, `export/svg.rs`) so export stays headless and deterministic for tests. Shape geometry changes (e.g. arrowhead math in `export/mod.rs`, outlines in `scene/hit.rs`) must be kept consistent across both paths.
- **Hit tolerance is spec**: `stroke_width / 2 + 4px` around unfilled outlines (`scene/hit.rs`); tests encode it.
- **Single instance via IPC**: startup binds a per-user local socket (`socket_name()` in `lib.rs`); a second invocation (or `olydraw toggle`) forwards a Toggle command and exits. This is also the GNOME Wayland hotkey fallback, since Wayland forbids app-defined global hotkeys.
- **Repaint discipline**: the overlay must idle at ~0% CPU. Continuous repaints are requested only while a drag or the laser trail is active. Don't add per-frame work that runs unconditionally.
- **Overlay lifecycle**: the window is hidden, never closed, on toggle (instant re-show; scene survives hide but not restart). On every show, `app.rs::move_to_active_monitor` repositions to the monitor containing the cursor (macOS-only today via `platform_macos::cursor_monitor`; other OSes fall back to primary) and records `capture_point` so composite export captures the right monitor. Composite export hides the overlay, waits `COMPOSITE_DELAY`, captures via xcap, then re-shows.
- **Prefs forward-compat**: every `Prefs` field carries `#[serde(default)]`; corrupt/missing/unknown-key TOML must load as defaults, never error (`prefs.rs`, enforced by tests).
- **No headless daemon mode**: olydraw only runs as a per-user GUI process with desktop/display access — `--hidden` suppresses the initial window, it doesn't detach from the session. Launch-at-login is done via OS-level login items (`packaging/macos/com.olydraw.app.plist.template`, `packaging/linux/olydraw-autostart.desktop.template`, `packaging/windows/register-autostart.ps1`), not a service manager; see README "Launch at login".

Platform-specific code is `cfg`-gated at module level (`platform_macos.rs`, `tray.rs` macOS/Windows only). macOS window elevation (above fullscreen apps, all Spaces) and monitor lookup use `objc2-app-kit`; coordinates there convert between Cocoa bottom-left-origin and the top-left-origin logical points egui uses.

Dependencies are deliberately trimmed (`image` PNG-only, eframe glow backend). Check with the user before adding a crate.

## Docs

`docs/stack.md`, `docs/architecture/overview.md`, `docs/adr/`, `docs/manual-testing.md` (per-OS checklist). Update these alongside architectural changes; keyboard shortcuts and user-facing behavior live in `README.md`.
