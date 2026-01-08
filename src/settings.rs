use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputMethod {
    Telex = 0,
    Vni = 1,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub enabled: bool,
    pub method: InputMethod,
    pub modern_tone: bool,
    pub auto_capitalize: bool,
    pub spell_check: bool,
    pub is_first_run: bool,
    #[serde(default = "default_true")]
    pub w_as_u_at_start: bool,
    #[serde(default = "default_true")]
    pub bracket_as_uo: bool,
    #[serde(default)]
    pub run_with_system: bool,
    #[serde(default)]
    pub auto_switch_mode: bool,
    #[serde(default)]
    pub auto_restore_english: bool,
}

fn default_true() -> bool { true }

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            method: InputMethod::Telex,
            modern_tone: false,
            auto_capitalize: true,
            spell_check: false,
            is_first_run: true,
            w_as_u_at_start: true,
            bracket_as_uo: true,
            run_with_system: false,
            auto_switch_mode: false,
            auto_restore_english: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        if let Some(path) = Self::get_config_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(settings) = serde_json::from_str(&content) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::get_config_path() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, content);
            }
        }
    }

    fn get_config_path() -> Option<PathBuf> {
        if let Some(dirs) = directories::ProjectDirs::from("org", "gonhanh", "GoNhanh") {
            return Some(dirs.config_dir().join("settings.json"));
        }
        None
    }
}
