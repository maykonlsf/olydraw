#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use olydraw::app::OlyApp;
use olydraw::ipc::{self, IpcCommand};

fn main() -> eframe::Result {
    let socket = olydraw::socket_name();

    // `olydraw toggle`: forward to the running instance and exit.
    if std::env::args().nth(1).as_deref() == Some("toggle") {
        if let Err(e) = ipc::send(&socket, IpcCommand::Toggle) {
            eprintln!("olydraw: no running instance ({e})");
            std::process::exit(1);
        }
        return Ok(());
    }

    // Single instance: if one is already running, toggle it instead.
    let (server, ipc_rx) = match ipc::bind(&socket) {
        Ok(bound) => bound,
        Err(e) => {
            eprintln!("olydraw: bind failed: {e}; forwarding toggle");
            let _ = ipc::send(&socket, IpcCommand::Toggle);
            return Ok(());
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_position(egui::Pos2::ZERO)
            .with_inner_size(egui::Vec2::new(1280.0, 800.0))
            .with_app_id("olydraw"),
        ..Default::default()
    };

    let result = eframe::run_native(
        "olydraw",
        options,
        Box::new(move |cc| Ok(Box::new(OlyApp::new(cc, ipc_rx)))),
    );
    drop(server);
    result
}
