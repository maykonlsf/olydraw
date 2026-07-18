use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

/// Owns the OS-level hotkey registration. Not available on Wayland — there
/// users bind a system shortcut to `olydraw toggle` instead.
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
}

impl HotkeyManager {
    /// Creates the manager and installs `on_press` as the global handler.
    /// Must be called on the main thread.
    pub fn new(on_press: impl Fn() + Send + Sync + 'static) -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state() == HotKeyState::Pressed {
                on_press();
            }
        }));
        Ok(Self {
            manager,
            current: None,
        })
    }

    /// Replaces the active binding with `binding` (e.g. "CmdOrCtrl+Shift+D").
    pub fn rebind(&mut self, binding: &str) -> Result<(), String> {
        let hotkey: HotKey = binding.parse().map_err(|e| format!("{e}"))?;
        if let Some(old) = self.current.take() {
            let _ = self.manager.unregister(old);
        }
        self.manager.register(hotkey).map_err(|e| e.to_string())?;
        self.current = Some(hotkey);
        Ok(())
    }
}
