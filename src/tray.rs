//! System tray icon and menu (macOS and Windows; on Linux control the app
//! with `olydraw toggle` instead).

use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCmd {
    Toggle,
    Clear,
    Quit,
}

pub struct Tray {
    _icon: TrayIcon,
}

/// Builds the tray icon and installs `on_cmd` as the menu handler.
/// Must be called on the main thread.
pub fn build(on_cmd: impl Fn(TrayCmd) + Send + Sync + 'static) -> Result<Tray, String> {
    let menu = Menu::new();
    let toggle = MenuItem::new("Show/hide overlay", true, None);
    let clear = MenuItem::new("Clear canvas", true, None);
    let quit = MenuItem::new("Quit olydraw", true, None);
    menu.append_items(&[&toggle, &clear, &quit])
        .map_err(|e| e.to_string())?;

    let (toggle_id, clear_id, quit_id) =
        (toggle.id().clone(), clear.id().clone(), quit.id().clone());
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let cmd = if *event.id() == toggle_id {
            TrayCmd::Toggle
        } else if *event.id() == clear_id {
            TrayCmd::Clear
        } else if *event.id() == quit_id {
            TrayCmd::Quit
        } else {
            return;
        };
        on_cmd(cmd);
    }));

    let icon =
        tray_icon::Icon::from_rgba(icon_rgba(), ICON_SIZE, ICON_SIZE).map_err(|e| e.to_string())?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_icon_as_template(true)
        .with_tooltip("olydraw")
        .build()
        .map_err(|e| e.to_string())?;
    Ok(Tray { _icon: tray })
}

const ICON_SIZE: u32 = 32;

/// A simple pen-nib glyph: filled ring, template-style (alpha only).
fn icon_rgba() -> Vec<u8> {
    let mut data = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    let c = ICON_SIZE as f32 / 2.0 - 0.5;
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let d = (dx * dx + dy * dy).sqrt();
            let ring = (d < 13.0 && d > 8.0) || d < 3.5;
            if ring {
                let i = ((y * ICON_SIZE + x) * 4) as usize;
                data[i..i + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    data
}
