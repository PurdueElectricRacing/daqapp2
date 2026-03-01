use crate::{connection, theme};

pub const SETTINGS_PATH: &str = "settings.json";
const DEFAULT_UDP_PORT: u16 = 5000;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub dbc_path: Option<std::path::PathBuf>,
    pub selected_source: Option<connection::ConnectionSource>,
    pub udp_port: u16,
    pub theme: theme::ThemeSelection,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dbc_path: None,
            selected_source: None,
            udp_port: DEFAULT_UDP_PORT,
            theme: theme::ThemeSelection::Default,
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
