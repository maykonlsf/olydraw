//! macOS-only window elevation: keep the overlay above fullscreen apps and
//! visible on every Space.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

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
