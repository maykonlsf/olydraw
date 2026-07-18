# Architecture Overview

Single long-lived process; the global hotkey / `olydraw toggle` CLI hides and shows an always-on-top transparent fullscreen window — no cold start on trigger.

```
┌──────────────────────────────────────────────┐
│ UI (egui)          src/ui.rs, src/app.rs     │
│ toolbar · text editor · export/settings      │
│ dialogs · canvas input state machine         │
├──────────────────────────────────────────────┤
│ Core (headless, fully unit-tested)           │
│ scene model + undo    src/scene/             │
│ freehand decimation   src/tools/freehand.rs  │
│ PNG/SVG export        src/export/            │
│ prefs                 src/prefs.rs           │
│ IPC single-instance   src/ipc.rs             │
├──────────────────────────────────────────────┤
│ Platform glue                                │
│ hotkey  src/hotkey.rs   tray  src/tray.rs    │
│ capture src/capture.rs  macOS window level   │
│                         src/platform_macos.rs│
└──────────────────────────────────────────────┘
```

## Key decisions

- **Vector-first scene** (`Scene` = ordered `Vec<Element>`): undo/redo is an invertible command stack (cap 100), the eraser deletes whole elements via geometric hit-testing, and SVG export is direct serialization. See ADR-0001.
- **Tools emit scene commands** — only `Scene` mutates state, so every mutation is undoable and the core stays testable without a window.
- **Two rasterizers, one geometry**: egui paints the live canvas; `tiny-skia` renders exports headlessly (tests never need a GPU). Shared helpers (e.g. arrowhead geometry) keep them consistent.
- **External commands** (IPC socket, global hotkey, tray menu) all funnel into one `AppCmd` queue drained in `App::logic`, which eframe runs even while the window is hidden (woken by `request_repaint` from the source thread).
- **Composite export** hides the window, waits ~350 ms for the compositor, captures via `xcap` at physical resolution, rasterizes the scene at the monitor scale factor, alpha-blends, saves, and re-shows.
- **Idle cost ≈ 0**: egui repaints only on input; continuous repaint is requested only while the laser trail is fading.
- **Single instance**: binding the IPC socket doubles as the instance lock; a second launch forwards `toggle` and exits. Stale sockets from killed processes are detected (connect fails) and removed.

## Known limitations (v1)

- Overlay covers the primary monitor (window at origin), not the cursor's monitor.
- Wayland: no layer-shell integration yet — regular borderless window fallback everywhere; no global hotkey (use `olydraw toggle`); no tray on Linux.
- Text re-edit commits as remove+add (two undo steps).
- Drawings survive hide/show but not app restart (no scene file format yet).
