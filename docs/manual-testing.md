# Manual Test Checklist

Automated tests cover the headless core. Everything below touches OS windowing and must be verified per platform (macOS / Windows / Linux X11 / Linux Wayland).

## Launch & toggle
- [ ] `olydraw` starts hidden-cost resident overlay; window covers the monitor, transparent, purple armed-border visible
- [ ] Global hotkey (Ctrl/Cmd+Shift+D) hides/shows instantly; apps underneath keep running (play a video)
- [ ] `olydraw toggle` from a shell toggles the running instance; second `olydraw` launch toggles instead of duplicating
- [ ] Kill -9 the process, relaunch — starts cleanly (stale socket recovery)
- [ ] Tray icon (macOS/Windows): show/hide, clear, quit all work
- [ ] Esc priority: commits text → cancels drag → deselects → hides
- [ ] `olydraw --hidden` (or `OLYDRAW_START_HIDDEN=1`) starts with no overlay visible; tray/hotkey/`toggle` still bring it up
- [ ] Launch-at-login setup (see README "Launch at login") starts olydraw hidden after a fresh login/boot on each OS

## Drawing
- [ ] Every tool draws correctly; keyboard shortcuts V/1 R/2 D/3 O/4 A/5 L/6 P/7 T/8 E/0 K switch tools (not while typing text)
- [ ] Shift constrains: square, circle, 45° lines/arrows
- [ ] Freehand smooth at fast strokes; single click = dot
- [ ] Text: click to type, Esc/click-away commits, empty text discarded, click existing text with T re-edits
- [ ] Eraser drag fades elements then removes them; single undo restores the whole drag
- [ ] Select: click, marquee (intersecting), drag-move multiple, Delete removes
- [ ] Undo/redo across all ops incl. clear; toolbar buttons match shortcuts
- [ ] Laser: fading red trail follows cursor, disappears ~0.7 s, CPU drops back to ~0 after

## Export
- [ ] PNG (annotation) transparent background, SVG opens in a browser, clipboard PNG pastes into another app
- [ ] Composite PNG: overlay vanishes from the capture, annotations aligned with screen content on a HiDPI/Retina display (scale factor correct)
- [ ] macOS: first composite prompts for Screen Recording; denial shows error toast, app keeps working
- [ ] Files land in ~/Pictures/olydraw or the configured directory

## Prefs & settings
- [ ] Color, stroke width, palette, export dir, hotkey persist across restart
- [ ] Hotkey rebinding via Settings applies immediately; invalid string shows error, old binding kept
- [ ] Corrupt prefs.toml → app starts with defaults

## Performance
- [ ] Idle (overlay visible, no input): ~0% CPU in Activity Monitor / Task Manager
- [ ] Drawing at 4K stays smooth
- [ ] Overlay shows above fullscreen apps (macOS Spaces)

## Wayland-specific
- [ ] System shortcut bound to `olydraw toggle` works (GNOME + KDE)
- [ ] Composite export triggers the portal permission dialog and captures correctly
