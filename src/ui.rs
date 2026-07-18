use std::time::Duration;

use egui::{Align2, Color32, FontId, Pos2, RichText, Stroke, Vec2};

use crate::app::{OlyApp, ToolKind};
use crate::export;

const STATUS_LIFETIME: Duration = Duration::from_secs(4);

pub fn toolbar(app: &mut OlyApp, ctx: &egui::Context) {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("toolbar"))
        .default_pos(Pos2::new(screen.center().x - 320.0, 12.0))
        .movable(true)
        .show(ctx, |ui| {
            egui::Frame::window(&ctx.global_style())
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        tool_buttons(app, ui);
                        ui.separator();
                        width_buttons(app, ui);
                        ui.separator();
                        color_buttons(app, ui);
                        ui.separator();
                        action_buttons(app, ui, ctx);
                    });
                });
        });
}

fn tool_buttons(app: &mut OlyApp, ui: &mut egui::Ui) {
    let tools: [(ToolKind, &str, &str); 10] = [
        (ToolKind::Select, "⬉", "Select — V or 1"),
        (ToolKind::Rect, "▭", "Rectangle — R or 2"),
        (ToolKind::Diamond, "◇", "Diamond — D or 3"),
        (ToolKind::Ellipse, "○", "Ellipse — O or 4"),
        (ToolKind::Arrow, "➔", "Arrow — A or 5"),
        (ToolKind::Line, "―", "Line — L or 6"),
        (ToolKind::Freehand, "✏", "Draw — P or 7"),
        (ToolKind::Text, "T", "Text — T or 8"),
        (ToolKind::Eraser, "⌫", "Eraser — E or 0"),
        (ToolKind::Laser, "☄", "Laser pointer — K"),
    ];
    for (tool, icon, tip) in tools {
        let selected = app.tool == tool;
        let button = ui.selectable_label(selected, RichText::new(icon).size(16.0));
        if button.on_hover_text(tip).clicked() {
            app.set_tool(tool);
        }
    }
}

fn width_buttons(app: &mut OlyApp, ui: &mut egui::Ui) {
    for (label, width, tip) in [("S", 2.0), ("M", 4.0), ("L", 7.0)]
        .map(|(l, w)| (l, w, format!("Stroke width {w} — [ and ] adjust")))
    {
        let selected = (app.prefs.stroke_width - width).abs() < 0.5;
        if ui
            .selectable_label(selected, label)
            .on_hover_text(tip)
            .clicked()
        {
            app.prefs.stroke_width = width;
            app.mark_prefs_dirty();
        }
    }
}

fn color_buttons(app: &mut OlyApp, ui: &mut egui::Ui) {
    let palette = app.prefs.palette.clone();
    for c in palette {
        let color = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
        let selected = app.prefs.color == c;
        let size = Vec2::splat(18.0);
        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
        let stroke = if selected {
            Stroke::new(2.0, ui.visuals().strong_text_color())
        } else {
            Stroke::new(1.0, ui.visuals().weak_text_color())
        };
        ui.painter().rect(
            rect.shrink(2.0),
            3.0,
            color,
            stroke,
            egui::StrokeKind::Inside,
        );
        if resp.on_hover_text("Color — C cycles palette").clicked() {
            app.prefs.color = c;
            app.mark_prefs_dirty();
        }
    }
    let mut custom = Color32::from_rgba_unmultiplied(
        app.prefs.color[0],
        app.prefs.color[1],
        app.prefs.color[2],
        app.prefs.color[3],
    );
    if ui.color_edit_button_srgba(&mut custom).changed() {
        app.prefs.color = custom.to_srgba_unmultiplied();
        app.mark_prefs_dirty();
    }
}

fn action_buttons(app: &mut OlyApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    if ui.button("↶").on_hover_text("Undo — Ctrl/Cmd+Z").clicked() {
        app.scene.undo();
    }
    if ui
        .button("↷")
        .on_hover_text("Redo — Ctrl/Cmd+Shift+Z")
        .clicked()
    {
        app.scene.redo();
    }
    if ui
        .button("🗑")
        .on_hover_text("Clear canvas (undoable)")
        .clicked()
    {
        app.clear_canvas();
    }
    if ui
        .button("💾")
        .on_hover_text("Export — Ctrl/Cmd+E")
        .clicked()
    {
        app.show_export = true;
    }
    if ui.button("⚙").on_hover_text("Settings").clicked() {
        app.show_settings = true;
    }
    if ui.button("✕").on_hover_text("Hide overlay — Esc").clicked() {
        app.hide_overlay(ctx);
    }
    if ui.button("⏻").on_hover_text("Quit olydraw").clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

pub fn text_editor(app: &mut OlyApp, ctx: &egui::Context) {
    let Some(edit) = &mut app.text_edit else {
        return;
    };
    let color = Color32::from_rgba_unmultiplied(
        edit.style.color[0],
        edit.style.color[1],
        edit.style.color[2],
        edit.style.color[3],
    );
    egui::Area::new(egui::Id::new("text-editor"))
        .fixed_pos(edit.pos)
        .show(ctx, |ui| {
            let response = ui.add(
                egui::TextEdit::multiline(&mut edit.buffer)
                    .font(FontId::proportional(edit.size))
                    .text_color(color)
                    .frame(egui::Frame::NONE)
                    .desired_width(600.0)
                    .desired_rows(1)
                    .hint_text("Type… (Esc to commit)"),
            );
            response.request_focus();
        });
}

pub fn export_dialog(app: &mut OlyApp, ctx: &egui::Context) {
    let mut open = app.show_export;
    let empty = app.scene.elements().is_empty();
    let size = ctx.content_rect().size();
    let mut action: Option<&str> = None;

    egui::Window::new("Export")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            if empty {
                ui.label("Nothing to export yet — draw something first.");
                return;
            }
            ui.label("Annotation only:");
            ui.horizontal(|ui| {
                if ui.button("Save PNG").clicked() {
                    action = Some("png");
                }
                if ui.button("Save SVG").clicked() {
                    action = Some("svg");
                }
                if ui.button("Copy PNG").clicked() {
                    action = Some("clipboard");
                }
            });
            ui.add_space(6.0);
            ui.label("Screen + annotation:");
            if ui.button("Save composite PNG").clicked() {
                action = Some("composite");
            }
        });

    match action {
        Some("png") => {
            let result = export::png::scene_to_png(&app.scene, size.x as u32, size.y as u32)
                .and_then(|bytes| write_export(app, "png", &bytes));
            report(app, result);
            app.show_export = false;
        }
        Some("svg") => {
            let svg = export::svg::scene_to_svg(&app.scene);
            let result = write_export(app, "svg", svg.as_bytes());
            report(app, result);
            app.show_export = false;
        }
        Some("clipboard") => {
            let result = export::png::scene_to_png(&app.scene, size.x as u32, size.y as u32)
                .and_then(|bytes| copy_png_to_clipboard(&bytes));
            report(app, result.map(|()| "copied PNG to clipboard".to_string()));
            app.show_export = false;
        }
        Some("composite") => {
            app.show_export = false;
            app.schedule_composite(ctx);
        }
        _ => app.show_export = open,
    }
}

pub fn settings_dialog(app: &mut OlyApp, ctx: &egui::Context) {
    let mut open = app.show_settings;
    let mut apply_hotkey = false;
    let mut export_dir = app.prefs.export_dir.clone().unwrap_or_default();
    let mut export_dir_changed = false;
    let mut pick_dir = false;

    egui::Window::new("Settings")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label("Global hotkey (e.g. CmdOrCtrl+Shift+D):");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut app.hotkey_input);
                if ui.button("Apply").clicked() {
                    apply_hotkey = true;
                }
            });
            ui.add_space(6.0);
            ui.label("Export directory (empty = ~/Pictures/olydraw):");
            ui.horizontal(|ui| {
                if ui.text_edit_singleline(&mut export_dir).changed() {
                    export_dir_changed = true;
                }
                if ui.button("Browse…").clicked() {
                    pick_dir = true;
                }
            });
        });

    if pick_dir && let Some(dir) = rfd::FileDialog::new().pick_folder() {
        export_dir = dir.display().to_string();
        export_dir_changed = true;
    }
    if export_dir_changed {
        app.prefs.export_dir = if export_dir.trim().is_empty() {
            None
        } else {
            Some(export_dir)
        };
        app.mark_prefs_dirty();
    }
    if apply_hotkey {
        app.apply_hotkey();
    }
    app.show_settings = open;
}

fn report(app: &mut OlyApp, result: Result<String, String>) {
    match result {
        Ok(msg) => app.set_status(msg),
        Err(e) => app.set_status(format!("export failed: {e}")),
    }
}

fn write_export(app: &OlyApp, ext: &str, bytes: &[u8]) -> Result<String, String> {
    let dir = export::export_dir(&app.prefs)?;
    export::write_bytes(&dir, ext, bytes).map(|path| format!("saved {path}"))
}

fn copy_png_to_clipboard(png_bytes: &[u8]) -> Result<(), String> {
    let img = image::load_from_memory(png_bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_image(arboard::ImageData {
            width: w,
            height: h,
            bytes: img.into_raw().into(),
        })
        .map_err(|e| e.to_string())
}

pub fn status_toast(app: &mut OlyApp, ctx: &egui::Context) {
    let Some((msg, at)) = &app.status else { return };
    if at.elapsed() > STATUS_LIFETIME {
        app.status = None;
        return;
    }
    let msg = msg.clone();
    egui::Area::new(egui::Id::new("status-toast"))
        .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -24.0))
        .show(ctx, |ui| {
            egui::Frame::window(&ctx.global_style())
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(msg);
                });
        });
    ctx.request_repaint_after(Duration::from_millis(500));
}
