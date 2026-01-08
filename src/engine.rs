use gonhanh_core::engine::{Engine as CoreEngine, Result as CoreResult};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::sync::Arc;
use crate::settings::Settings;

// Public alias for Result to be used by hook
pub use gonhanh_core::engine::Result as ImeResult;

pub struct EngineWrapper {
    core: CoreEngine,
    settings: Settings,
}

impl EngineWrapper {
    pub fn new() -> Self {
        let settings = Settings::load();
        let mut core = CoreEngine::new();
        
        // Apply settings to core
        core.set_method(settings.method as u8);
        core.set_enabled(settings.enabled);
        core.set_modern_tone(settings.modern_tone);
        core.set_auto_capitalize(settings.auto_capitalize);
        core.set_skip_w_shortcut(!settings.w_as_u_at_start);
        core.set_bracket_shortcut(settings.bracket_as_uo);
        core.set_english_auto_restore(settings.auto_restore_english);

        Self { core, settings }
    }

    pub fn process_key(&mut self, keycode: u16, _shift: bool, capslock: bool) -> CoreResult {
        // Core handles the logic. We just pass the mapped key.
        // If keycode mapping failed (e.g. keycode == 0), caller shouldn't call this or we handle it.
        // We assume valid keycode here.
        self.core.on_key(keycode, capslock, false) 
        // Note: 'ctrl' param is false because we handle shortcuts locally or pass them?
        // Core takes (key, caps, ctrl).
        // If we want core to handle Ctrl shortcuts/bypass, we should pass correct Ctrl state.
        // For now pass false (assuming no Ctrl combos handled by IME core logic).
    }

    pub fn update_settings(&mut self, new_settings: Settings) {
        self.settings = new_settings.clone();
        self.core.set_method(new_settings.method as u8);
        self.core.set_enabled(new_settings.enabled);
        self.core.set_modern_tone(new_settings.modern_tone);
        self.core.set_auto_capitalize(new_settings.auto_capitalize);
        self.core.set_skip_w_shortcut(!new_settings.w_as_u_at_start);
        self.core.set_bracket_shortcut(new_settings.bracket_as_uo);
        self.core.set_english_auto_restore(new_settings.auto_restore_english);
        self.settings.save();
    }

    pub fn toggle_enabled(&mut self) {
        self.settings.enabled = !self.settings.enabled;
        self.core.set_enabled(self.settings.enabled);
        // Don't save settings on quick toggle? Or do we?
        // Usually quick toggle is temporary. But let's save for persistence.
        self.settings.save();
    }

    pub fn get_settings(&self) -> Settings {
        self.settings.clone()
    }
}

lazy_static! {
    pub static ref ENGINE: Arc<Mutex<EngineWrapper>> = Arc::new(Mutex::new(EngineWrapper::new()));
}
