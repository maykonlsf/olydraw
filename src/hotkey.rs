use std::sync::{Arc, Mutex};

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

/// Which app behavior a global hotkey binding triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    Toggle,
    Passthrough,
}

/// Owns the OS-level hotkey registrations. Not available on Wayland — there
/// users bind system shortcuts to `olydraw toggle` / `olydraw passthrough`
/// instead.
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    bindings: Arc<Mutex<Vec<(HotKey, HotkeyAction)>>>,
}

impl HotkeyManager {
    /// Creates the manager and installs `on_press` as the global handler,
    /// invoked with whichever action's binding fired.
    /// Must be called on the main thread.
    pub fn new(on_press: impl Fn(HotkeyAction) + Send + Sync + 'static) -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        let bindings: Arc<Mutex<Vec<(HotKey, HotkeyAction)>>> = Arc::default();
        let handler_bindings = bindings.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state() != HotKeyState::Pressed {
                return;
            }
            let bindings = handler_bindings.lock().unwrap();
            if let Some((_, action)) = bindings.iter().find(|(hk, _)| hk.id() == event.id()) {
                on_press(*action);
            }
        }));
        Ok(Self { manager, bindings })
    }

    /// Replaces the binding for `action` with `binding` (e.g.
    /// "CmdOrCtrl+Shift+D"), unregistering the action's previous binding
    /// first, if any.
    pub fn rebind(&mut self, action: HotkeyAction, binding: &str) -> Result<(), String> {
        let hotkey: HotKey = binding.parse().map_err(|e| format!("{e}"))?;
        let mut bindings = self.bindings.lock().unwrap();
        if let Some(pos) = bindings.iter().position(|(_, a)| *a == action) {
            let (old, _) = bindings.remove(pos);
            let _ = self.manager.unregister(old);
        }
        self.manager.register(hotkey).map_err(|e| e.to_string())?;
        bindings.push((hotkey, action));
        Ok(())
    }
}
