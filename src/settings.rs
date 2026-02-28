use crate::theme;

pub const SETTINGS_PATH: &str = "settings.json";

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub dbc_path: Option<std::path::PathBuf>,
    pub selected_serial: Option<String>,
    pub theme: theme::ThemeSelection,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: theme::ThemeSelection::Default,
            dbc_path: None,
            selected_serial: None,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        if let Ok(json) = std::fs::read_to_string(SETTINGS_PATH) {
            serde_json::from_str(&json).unwrap_or_default()
        } else {
            let default = Settings::default();
            default.save();
            default
        }
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(self).expect("Failed to serialize settings");
        std::fs::write(SETTINGS_PATH, json)
            .unwrap_or_else(|e| log::error!("Failed to write {}: {}", SETTINGS_PATH, e));
    }
}
