# ADR-0001: egui/eframe UI with a vector-first scene model

Date: 2026-07-18 · Status: accepted

## Context

olydraw needs an always-on-top transparent overlay with an Excalidraw-like toolset on macOS, Windows, and Linux, explicitly without Electron or other heavy runtimes, and with near-zero idle resource usage.

Alternatives considered:
1. `winit` + CPU rendering (`softbuffer`/`tiny-skia`) — smallest binary, but all UI widgets (toolbar, color picker, text editing) hand-rolled.
2. **`eframe`/`egui` (glow backend)** — widgets, text editing, and canvas painting out of the box; ~10 MB binary; repaint-on-input keeps idle CPU ≈ 0.
3. `winit` + `wgpu` + `lyon` — best rendering ceiling, most code, heavy GPU dependency; overkill for stroke annotation.

## Decision

Use eframe/egui (option 2) with a vector-first scene: `Vec<Element>` where `Element` is Freehand/Shape/Text plus style, mutated only through invertible commands.

Exports are rendered by a second, headless rasterizer (`tiny-skia` + `ab_glyph`) instead of egui, so the core compiles and tests without a window or GPU.

## Consequences

- Undo/redo, whole-element erasing, marquee selection, and SVG export fall out of the scene model naturally.
- Toolbar/settings/text UI cost little code; UI style is egui's, not fully custom.
- Two renderers must be kept visually consistent (shared geometry helpers; golden-image tests guard the export side).
- egui's API moves quickly; upgrades need care (0.35 already diverged from older egui idioms).
