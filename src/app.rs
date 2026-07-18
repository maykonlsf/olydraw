use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::{Color32, Pos2, Sense, Vec2};

use crate::hotkey::HotkeyManager;
use crate::ipc::IpcCommand;
use crate::laser::LaserTrail;
use crate::prefs::{self, Prefs};
use crate::render;
use crate::scene::{ElementId, ElementKind, Point, Rect, Scene, ShapeKind, Style};
use crate::tools::freehand;
use crate::ui;

const FREEHAND_EPSILON: f32 = 1.5;
const DEFAULT_TEXT_SIZE: f32 = 24.0;
const PREFS_SAVE_DEBOUNCE: Duration = Duration::from_millis(500);
/// Delay between hiding the overlay and grabbing the screenshot, so the
/// compositor has removed the window from screen.
const COMPOSITE_DELAY: Duration = Duration::from_millis(350);

/// Commands arriving from outside the egui loop (IPC, global hotkey, tray).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCmd {
    Toggle,
    Clear,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Select,
    Rect,
    Diamond,
    Ellipse,
    Arrow,
    Line,
    Freehand,
    Text,
    Eraser,
    Laser,
}

pub enum DragOp {
    Shape {
        kind: ShapeKind,
        start: Pos2,
        current: Pos2,
    },
    Freehand {
        points: Vec<Point>,
    },
    Marquee {
        start: Pos2,
        current: Pos2,
    },
    MoveSelection {
        start: Pos2,
        current: Pos2,
    },
    Erase {
        last: Pos2,
        pending: Vec<ElementId>,
    },
}

pub struct TextEditState {
    pub pos: Pos2,
    pub buffer: String,
    pub size: f32,
    /// Style captured when editing began (re-edit keeps original color).
    pub style: Style,
}

pub struct OlyApp {
    pub scene: Scene,
    pub tool: ToolKind,
    pub selected: Vec<ElementId>,
    pub drag: Option<DragOp>,
    pub text_edit: Option<TextEditState>,
    pub laser: LaserTrail,
    pub prefs: Prefs,
    prefs_dir: Option<PathBuf>,
    prefs_dirty: Option<Instant>,
    pub show_export: bool,
    pub show_settings: bool,
    pub hotkey_input: String,
    pub status: Option<(String, Instant)>,
    pending_cmds: Arc<Mutex<Vec<AppCmd>>>,
    pending_composite: Option<Instant>,
    hotkey: Option<HotkeyManager>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    _tray: Option<crate::tray::Tray>,
    visible: bool,
    sized: bool,
    /// A global point inside the monitor the overlay was last shown on,
    /// used to capture the right screen for composite export.
    capture_point: Option<(i32, i32)>,
}

impl OlyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, ipc_rx: Receiver<IpcCommand>) -> Self {
        let prefs_dir = directories::ProjectDirs::from("com", "olydraw", "olydraw")
            .map(|d| d.config_dir().to_path_buf());
        let prefs = prefs_dir
            .as_deref()
            .map(prefs::load_from)
            .unwrap_or_default();

        let pending_cmds: Arc<Mutex<Vec<AppCmd>>> = Arc::default();
        let push_cmd = {
            let pending = pending_cmds.clone();
            let ctx = cc.egui_ctx.clone();
            move |cmd: AppCmd| {
                pending.lock().unwrap().push(cmd);
                // Wake the (possibly hidden) event loop.
                ctx.request_repaint();
            }
        };

        {
            let push = push_cmd.clone();
            std::thread::spawn(move || {
                while let Ok(cmd) = ipc_rx.recv() {
                    match cmd {
                        IpcCommand::Toggle => push(AppCmd::Toggle),
                    }
                }
            });
        }

        let mut status = None;
        let hotkey = {
            let push = push_cmd.clone();
            match HotkeyManager::new(move || push(AppCmd::Toggle)) {
                Ok(mut manager) => match manager.rebind(&prefs.hotkey) {
                    Ok(()) => Some(manager),
                    Err(e) => {
                        status = Some((
                            format!("hotkey '{}' not registered: {e}", prefs.hotkey),
                            Instant::now(),
                        ));
                        Some(manager)
                    }
                },
                Err(e) => {
                    status = Some((format!("global hotkey unavailable: {e}"), Instant::now()));
                    None
                }
            }
        };

        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let tray = {
            let push = push_cmd.clone();
            crate::tray::build(move |cmd| {
                push(match cmd {
                    crate::tray::TrayCmd::Toggle => AppCmd::Toggle,
                    crate::tray::TrayCmd::Clear => AppCmd::Clear,
                    crate::tray::TrayCmd::Quit => AppCmd::Quit,
                })
            })
            .ok()
        };

        #[cfg(target_os = "macos")]
        let _ = crate::platform_macos::elevate_window(cc);

        let hotkey_input = prefs.hotkey.clone();
        Self {
            scene: Scene::new(),
            tool: ToolKind::Freehand,
            selected: Vec::new(),
            drag: None,
            text_edit: None,
            laser: LaserTrail::default(),
            prefs,
            prefs_dir,
            prefs_dirty: None,
            show_export: false,
            show_settings: false,
            hotkey_input,
            status,
            pending_cmds,
            pending_composite: None,
            hotkey,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            _tray: tray,
            visible: true,
            sized: false,
            capture_point: None,
        }
    }

    /// Positions and sizes the overlay to fill the monitor containing the
    /// cursor. Falls back to the current monitor at the primary origin when
    /// the cursor's monitor can't be determined (non-macOS for now).
    fn move_to_active_monitor(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "macos")]
        if let Some(m) = crate::platform_macos::cursor_monitor() {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(m.x, m.y)));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2::new(
                m.width, m.height,
            )));
            self.capture_point =
                Some(((m.x + m.width / 2.0) as i32, (m.y + m.height / 2.0) as i32));
            self.sized = true;
            return;
        }
        if let Some(size) = ctx.input(|i| i.viewport().monitor_size) {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::ZERO));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            self.sized = true;
        }
    }

    /// Applies a new hotkey binding string, persisting it on success.
    pub fn apply_hotkey(&mut self) {
        let binding = self.hotkey_input.trim().to_string();
        let Some(manager) = &mut self.hotkey else {
            self.set_status("global hotkey unavailable on this system");
            return;
        };
        match manager.rebind(&binding) {
            Ok(()) => {
                self.prefs.hotkey = binding.clone();
                self.mark_prefs_dirty();
                self.set_status(format!("hotkey set to {binding}"));
            }
            Err(e) => self.set_status(format!("invalid hotkey '{binding}': {e}")),
        }
    }

    /// Hides the overlay, waits for the compositor, then captures + composites.
    pub fn schedule_composite(&mut self, ctx: &egui::Context) {
        self.hide_overlay(ctx);
        self.pending_composite = Some(Instant::now());
        ctx.request_repaint_after(COMPOSITE_DELAY + Duration::from_millis(50));
    }

    fn finish_composite(&mut self, ctx: &egui::Context) {
        let result = crate::capture::composite_with_screen(&self.scene, self.capture_point)
            .and_then(|img| {
                crate::export::save_image(&img, &crate::export::export_dir(&self.prefs)?)
            });
        // Bring the overlay back before reporting.
        if !self.visible {
            self.toggle_visibility(ctx);
        }
        match result {
            Ok(path) => self.set_status(format!("saved {path}")),
            Err(e) => self.set_status(format!("composite export failed: {e}")),
        }
    }

    pub fn style(&self) -> Style {
        Style {
            color: self.prefs.color,
            stroke_width: self.prefs.stroke_width,
        }
    }

    pub fn mark_prefs_dirty(&mut self) {
        self.prefs_dirty = Some(Instant::now());
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), Instant::now()));
    }

    pub fn set_tool(&mut self, tool: ToolKind) {
        if self.tool == tool {
            return;
        }
        self.commit_text_edit();
        self.commit_drag();
        if tool != ToolKind::Select {
            self.selected.clear();
        }
        if tool != ToolKind::Laser {
            self.laser.clear();
        }
        self.tool = tool;
    }

    pub fn hide_overlay(&mut self, ctx: &egui::Context) {
        if self.visible {
            self.toggle_visibility(ctx);
        }
    }

    fn toggle_visibility(&mut self, ctx: &egui::Context) {
        self.visible = !self.visible;
        if self.visible {
            self.move_to_active_monitor(ctx);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        } else {
            self.commit_text_edit();
            self.commit_drag();
            self.laser.clear();
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    // ----- element commit helpers -----

    pub fn commit_text_edit(&mut self) {
        let Some(edit) = self.text_edit.take() else {
            return;
        };
        if edit.buffer.trim().is_empty() {
            return;
        }
        self.scene.add(
            ElementKind::Text {
                pos: Point::new(edit.pos.x, edit.pos.y),
                content: edit.buffer,
                size: edit.size,
            },
            edit.style,
        );
    }

    fn commit_drag(&mut self) {
        let Some(op) = self.drag.take() else { return };
        match op {
            DragOp::Shape {
                kind,
                start,
                current,
            } => {
                if (current - start).length() >= 2.0 {
                    let (a, b) = (to_point(start), to_point(current));
                    self.scene.add(
                        ElementKind::Shape {
                            kind,
                            start: a,
                            end: b,
                        },
                        self.style(),
                    );
                }
            }
            DragOp::Freehand { points } => {
                if !points.is_empty() {
                    let points = freehand::decimate(&points, FREEHAND_EPSILON);
                    self.scene
                        .add(ElementKind::Freehand { points }, self.style());
                }
            }
            DragOp::Marquee { start, current } => {
                self.selected = self
                    .scene
                    .hit_rect(Rect::new(to_point(start), to_point(current)).ordered());
            }
            DragOp::MoveSelection { start, current } => {
                let delta = current - start;
                if delta.length() > 0.0 {
                    self.scene.translate(&self.selected, delta.x, delta.y);
                }
            }
            DragOp::Erase { pending, .. } => {
                if !pending.is_empty() {
                    self.scene.remove(&pending);
                }
            }
        }
    }

    fn cancel_drag(&mut self) {
        self.drag = None;
    }

    pub fn delete_selection(&mut self) {
        if !self.selected.is_empty() {
            self.scene.remove(&self.selected);
            self.selected.clear();
        }
    }

    pub fn clear_canvas(&mut self) {
        self.commit_text_edit();
        self.selected.clear();
        self.scene.clear();
    }

    // ----- input -----

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let typing = self.text_edit.is_some() || ctx.egui_wants_keyboard_input();

        let (esc, cmd_z, cmd_shift_z, cmd_e) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Escape),
                i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z),
                i.modifiers.command && i.key_pressed(egui::Key::E),
            )
        });

        if esc {
            if self.text_edit.is_some() {
                self.commit_text_edit();
            } else if self.drag.is_some() {
                self.cancel_drag();
            } else if !self.selected.is_empty() {
                self.selected.clear();
            } else if self.show_export {
                self.show_export = false;
            } else {
                self.toggle_visibility(ctx);
            }
            return;
        }
        if cmd_shift_z {
            self.scene.redo();
            return;
        }
        if cmd_z {
            self.scene.undo();
            return;
        }
        if cmd_e {
            self.show_export = !self.show_export;
            return;
        }
        if typing {
            return;
        }

        let pressed = |ctx: &egui::Context, k: egui::Key| ctx.input(|i| i.key_pressed(k));
        use egui::Key as K;
        let tools = [
            (K::V, ToolKind::Select),
            (K::Num1, ToolKind::Select),
            (K::R, ToolKind::Rect),
            (K::Num2, ToolKind::Rect),
            (K::D, ToolKind::Diamond),
            (K::Num3, ToolKind::Diamond),
            (K::O, ToolKind::Ellipse),
            (K::Num4, ToolKind::Ellipse),
            (K::A, ToolKind::Arrow),
            (K::Num5, ToolKind::Arrow),
            (K::L, ToolKind::Line),
            (K::Num6, ToolKind::Line),
            (K::P, ToolKind::Freehand),
            (K::Num7, ToolKind::Freehand),
            (K::T, ToolKind::Text),
            (K::Num8, ToolKind::Text),
            (K::E, ToolKind::Eraser),
            (K::Num0, ToolKind::Eraser),
            (K::K, ToolKind::Laser),
        ];
        for (key, tool) in tools {
            if pressed(ctx, key) {
                self.set_tool(tool);
                return;
            }
        }

        if pressed(ctx, K::Delete) || pressed(ctx, K::Backspace) {
            self.delete_selection();
        }
        if pressed(ctx, K::OpenBracket) {
            self.prefs.stroke_width = (self.prefs.stroke_width - 1.0).max(1.0);
            self.mark_prefs_dirty();
        }
        if pressed(ctx, K::CloseBracket) {
            self.prefs.stroke_width = (self.prefs.stroke_width + 1.0).min(24.0);
            self.mark_prefs_dirty();
        }
        if pressed(ctx, K::C) {
            let palette = &self.prefs.palette;
            if !palette.is_empty() {
                let idx = palette.iter().position(|c| *c == self.prefs.color);
                let next = idx.map(|i| (i + 1) % palette.len()).unwrap_or(0);
                self.prefs.color = palette[next];
                self.mark_prefs_dirty();
            }
        }
    }

    fn handle_canvas(&mut self, response: &egui::Response) {
        let shift = response.ctx.input(|i| i.modifiers.shift);
        let pointer = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos());
        // Intermediate pointer positions from this frame. Pen tablets (and
        // fast mice) report more often than we render; sampling only the
        // final position per frame would cut the corners of fast strokes.
        let moves: Vec<Pos2> = response.ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::PointerMoved(p) => Some(*p),
                    _ => None,
                })
                .collect()
        });

        if self.tool == ToolKind::Laser {
            // Trail only while the button is held, like drawing.
            if response.is_pointer_button_down_on()
                && let Some(pos) = response.interact_pointer_pos()
            {
                if moves.is_empty() {
                    self.laser.push(pos);
                } else {
                    for p in moves {
                        self.laser.push(p);
                    }
                }
            } else {
                self.laser.break_stroke();
            }
            return;
        }

        if response.drag_started()
            && let Some(pos) = pointer
        {
            self.start_drag(pos);
        }
        if response.dragged()
            && let Some(pos) = pointer
        {
            if matches!(self.drag, Some(DragOp::Freehand { .. })) {
                for p in moves {
                    self.update_drag(p, shift);
                }
            }
            self.update_drag(pos, shift);
        }
        if response.drag_stopped() {
            self.commit_drag();
        }
        if response.clicked()
            && let Some(pos) = pointer
        {
            self.handle_click(pos);
        }
    }

    fn start_drag(&mut self, pos: Pos2) {
        self.commit_text_edit();
        let shape = |kind| DragOp::Shape {
            kind,
            start: pos,
            current: pos,
        };
        self.drag = Some(match self.tool {
            ToolKind::Rect => shape(ShapeKind::Rect),
            ToolKind::Diamond => shape(ShapeKind::Diamond),
            ToolKind::Ellipse => shape(ShapeKind::Ellipse),
            ToolKind::Arrow => shape(ShapeKind::Arrow),
            ToolKind::Line => shape(ShapeKind::Line),
            ToolKind::Freehand => DragOp::Freehand {
                points: vec![to_point(pos)],
            },
            ToolKind::Eraser => DragOp::Erase {
                last: pos,
                pending: self.scene.hit_top(to_point(pos)).into_iter().collect(),
            },
            ToolKind::Select => {
                if let Some(id) = self.scene.hit_top(to_point(pos)) {
                    if !self.selected.contains(&id) {
                        self.selected = vec![id];
                    }
                    DragOp::MoveSelection {
                        start: pos,
                        current: pos,
                    }
                } else {
                    self.selected.clear();
                    DragOp::Marquee {
                        start: pos,
                        current: pos,
                    }
                }
            }
            ToolKind::Text | ToolKind::Laser => return,
        });
    }

    fn update_drag(&mut self, pos: Pos2, shift: bool) {
        let mut extra_hits: Vec<ElementId> = Vec::new();
        match &mut self.drag {
            Some(DragOp::Shape {
                kind,
                start,
                current,
            }) => {
                *current = if shift {
                    constrain(*kind, *start, pos)
                } else {
                    pos
                };
            }
            Some(DragOp::Marquee { current, .. }) | Some(DragOp::MoveSelection { current, .. }) => {
                *current = pos
            }
            Some(DragOp::Freehand { points }) => points.push(to_point(pos)),
            Some(DragOp::Erase { last, pending }) => {
                extra_hits = self.scene.hit_segment(to_point(*last), to_point(pos));
                *last = pos;
                for id in &extra_hits {
                    if !pending.contains(id) {
                        pending.push(*id);
                    }
                }
            }
            None => {}
        }
        let _ = extra_hits;
    }

    fn handle_click(&mut self, pos: Pos2) {
        match self.tool {
            ToolKind::Text => {
                self.commit_text_edit();
                // Clicking an existing text re-edits it.
                if let Some(id) = self.scene.hit_top(to_point(pos)) {
                    let existing = self
                        .scene
                        .elements()
                        .iter()
                        .find(|e| e.id == id)
                        .filter(|e| matches!(e.kind, ElementKind::Text { .. }))
                        .cloned();
                    if let Some(e) = existing
                        && let ElementKind::Text {
                            pos: tp,
                            content,
                            size,
                        } = e.kind
                    {
                        self.scene.remove(&[id]);
                        self.text_edit = Some(TextEditState {
                            pos: Pos2::new(tp.x, tp.y),
                            buffer: content,
                            size,
                            style: e.style,
                        });
                        return;
                    }
                }
                self.text_edit = Some(TextEditState {
                    pos,
                    buffer: String::new(),
                    size: DEFAULT_TEXT_SIZE,
                    style: self.style(),
                });
            }
            ToolKind::Select => {
                self.commit_text_edit();
                match self.scene.hit_top(to_point(pos)) {
                    Some(id) => self.selected = vec![id],
                    None => self.selected.clear(),
                }
            }
            ToolKind::Freehand => {
                self.commit_text_edit();
                self.scene.add(
                    ElementKind::Freehand {
                        points: vec![to_point(pos)],
                    },
                    self.style(),
                );
            }
            ToolKind::Eraser => {
                self.commit_text_edit();
                if let Some(id) = self.scene.hit_top(to_point(pos)) {
                    self.scene.remove(&[id]);
                }
            }
            _ => self.commit_text_edit(),
        }
    }

    // ----- painting -----

    fn paint(&mut self, painter: &egui::Painter, screen: egui::Rect) {
        // Subtle border showing the overlay is armed.
        painter.rect_stroke(
            screen.shrink(1.0),
            0.0,
            egui::Stroke::new(2.0, Color32::from_rgba_unmultiplied(120, 100, 255, 90)),
            egui::StrokeKind::Inside,
        );

        let (move_offset, erase_pending) = match &self.drag {
            Some(DragOp::MoveSelection { start, current }) => (*current - *start, None),
            Some(DragOp::Erase { pending, .. }) => (Vec2::ZERO, Some(pending)),
            _ => (Vec2::ZERO, None),
        };

        for e in self.scene.elements() {
            let offset = if self.selected.contains(&e.id) {
                move_offset
            } else {
                Vec2::ZERO
            };
            let alpha = if erase_pending.is_some_and(|p| p.contains(&e.id)) {
                0.25
            } else {
                1.0
            };
            render::paint_element(painter, e, offset, alpha);
        }

        for id in &self.selected {
            if let Some(e) = self.scene.elements().iter().find(|e| e.id == *id) {
                render::paint_selection_box(painter, e, move_offset);
            }
        }

        // Live preview of in-progress drawing.
        match &self.drag {
            Some(DragOp::Shape {
                kind,
                start,
                current,
            }) => {
                let stroke = egui::Stroke::new(
                    self.prefs.stroke_width,
                    render::color32(self.prefs.color, 1.0),
                );
                render::paint_shape(
                    painter,
                    *kind,
                    to_point(*start),
                    to_point(*current),
                    stroke,
                    Vec2::ZERO,
                );
            }
            Some(DragOp::Freehand { points }) => {
                let pts: Vec<Pos2> = points.iter().map(|p| Pos2::new(p.x, p.y)).collect();
                painter.add(egui::Shape::line(
                    pts,
                    egui::Stroke::new(
                        self.prefs.stroke_width,
                        render::color32(self.prefs.color, 1.0),
                    ),
                ));
            }
            Some(DragOp::Marquee { start, current }) => {
                let rect = egui::Rect::from_two_pos(*start, *current);
                painter.rect(
                    rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(64, 128, 255, 20),
                    egui::Stroke::new(1.0, Color32::from_rgb(64, 128, 255)),
                    egui::StrokeKind::Inside,
                );
            }
            _ => {}
        }

        self.laser.prune();
        if !self.laser.is_empty() {
            self.laser.paint(painter);
        }
    }

    fn autosave_prefs(&mut self) {
        let Some(mark) = self.prefs_dirty else { return };
        if mark.elapsed() < PREFS_SAVE_DEBOUNCE {
            return;
        }
        self.prefs_dirty = None;
        if let Some(dir) = &self.prefs_dir
            && let Err(e) = prefs::save_to(dir, &self.prefs)
        {
            self.set_status(format!("prefs save failed: {e}"));
        }
    }
}

impl eframe::App for OlyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    // Runs even while the window is hidden (woken by request_repaint from
    // the IPC thread), so toggle commands always land.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Size to the cursor's monitor on the first frames after startup.
        if !self.sized {
            self.move_to_active_monitor(ctx);
        }

        let cmds = std::mem::take(&mut *self.pending_cmds.lock().unwrap());
        for cmd in cmds {
            match cmd {
                AppCmd::Toggle => self.toggle_visibility(ctx),
                AppCmd::Clear => self.clear_canvas(),
                AppCmd::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }

        if let Some(at) = self.pending_composite {
            if at.elapsed() >= COMPOSITE_DELAY {
                self.pending_composite = None;
                self.finish_composite(ctx);
            } else {
                ctx.request_repaint_after(COMPOSITE_DELAY - at.elapsed());
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.visible {
            return;
        }

        self.handle_shortcuts(&ctx);

        let screen = ui.max_rect();
        let response = ui.allocate_rect(screen, Sense::click_and_drag());
        self.handle_canvas(&response);
        let painter = ui.painter_at(screen);
        self.paint(&painter, screen);

        ui::toolbar(self, &ctx);
        ui::text_editor(self, &ctx);
        if self.show_export {
            ui::export_dialog(self, &ctx);
        }
        if self.show_settings {
            ui::settings_dialog(self, &ctx);
        }
        ui::status_toast(self, &ctx);

        self.autosave_prefs();

        // Only animate while the laser trail is fading; stay idle otherwise.
        if !self.laser.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if self.prefs_dirty.is_some() {
            ctx.request_repaint_after(PREFS_SAVE_DEBOUNCE);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(dir) = &self.prefs_dir {
            let _ = prefs::save_to(dir, &self.prefs);
        }
    }
}

fn to_point(p: Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// Shift-key constraint: squares/circles for box shapes, 45° snapping for
/// lines and arrows.
fn constrain(kind: ShapeKind, start: Pos2, pos: Pos2) -> Pos2 {
    let d = pos - start;
    match kind {
        ShapeKind::Rect | ShapeKind::Ellipse | ShapeKind::Diamond => {
            let side = d.x.abs().max(d.y.abs());
            Pos2::new(start.x + side * d.x.signum(), start.y + side * d.y.signum())
        }
        ShapeKind::Line | ShapeKind::Arrow => {
            let angle = d.y.atan2(d.x);
            let snapped =
                (angle / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
            let len = d.length();
            Pos2::new(start.x + len * snapped.cos(), start.y + len * snapped.sin())
        }
    }
}

trait RectExt {
    fn ordered(self) -> Rect;
}

impl RectExt for Rect {
    fn ordered(self) -> Rect {
        Rect::new(
            Point::new(self.min.x.min(self.max.x), self.min.y.min(self.max.y)),
            Point::new(self.min.x.max(self.max.x), self.min.y.max(self.max.y)),
        )
    }
}
