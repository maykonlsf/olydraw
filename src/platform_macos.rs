//! macOS-only window elevation: keep the overlay above fullscreen apps and
//! visible on every Space.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// A monitor's bounds in global logical points, top-left origin (the
/// coordinate space egui viewport commands and xcap points use).
pub struct MonitorRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The monitor currently containing the mouse cursor. Falls back to the
/// primary screen if the cursor is on no screen (e.g. mid-reconfiguration).
/// Returns None off the main thread.
pub fn cursor_monitor() -> Option<MonitorRect> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSScreen};

    let mtm = MainThreadMarker::new()?;
    // Cocoa global coords: bottom-left origin, y grows upward.
    let mouse = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);
    let primary_height = screens.iter().next()?.frame().size.height;
    let screen = screens
        .iter()
        .find(|s| {
            let f = s.frame();
            mouse.x >= f.origin.x
                && mouse.x < f.origin.x + f.size.width
                && mouse.y >= f.origin.y
                && mouse.y < f.origin.y + f.size.height
        })
        .or_else(|| screens.iter().next())?;
    let f = screen.frame();
    Some(MonitorRect {
        x: f.origin.x as f32,
        y: (primary_height - (f.origin.y + f.size.height)) as f32,
        width: f.size.width as f32,
        height: f.size.height as f32,
    })
}

pub fn elevate_window(cc: &eframe::CreationContext<'_>) -> Result<(), String> {
    use objc2_app_kit::{NSPopUpMenuWindowLevel, NSView, NSWindowCollectionBehavior};

    let handle = cc.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return Err("not an AppKit window".to_string());
    };
    // SAFETY: eframe hands us a live NSView pointer for the window it created;
    // we only call main-thread-safe setters during app creation (main thread).
    unsafe {
        let view = &*appkit.ns_view.as_ptr().cast::<NSView>();
        let window = view
            .window()
            .ok_or_else(|| "view has no window".to_string())?;
        window.setLevel(NSPopUpMenuWindowLevel);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary,
        );
    }
    Ok(())
}
