use eframe::egui::{self, Color32};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeColors {
    pub background: String,
    pub panel_bg: String,
    pub text: String,
    pub accent: String,
    pub button: Option<String>,
    pub button_hover: Option<String>,
    pub button_text: Option<String>,
}

impl ThemeColors {
    pub fn parse_hex(hex: &str) -> Color32 {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                Color32::from_rgb(r, g, b)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
                Color32::from_rgba_unmultiplied(r, g, b, a)
            }
            _ => Color32::from_rgb(255, 0, 255), // fallback magenta for invalid
        }
    }

    pub fn to_egui_style(&self) -> egui::Style {
        let mut style = egui::Style::default();

        let bg = Self::parse_hex(&self.background);
        let panel = Self::parse_hex(&self.panel_bg);
        let text = Self::parse_hex(&self.text);
        let accent = Self::parse_hex(&self.accent);
        let button = Self::parse_hex(&self.button.clone().unwrap_or_else(|| self.accent.clone()));
        let button_hover = Self::parse_hex(
            &self
                .button_hover
                .clone()
                .unwrap_or_else(|| self.accent.clone()),
        );
        let button_text = Self::parse_hex(
            &self
                .button_text
                .clone()
                .unwrap_or_else(|| self.text.clone()),
        );

        // --- Base ---
        style.visuals.window_fill = bg;
        style.visuals.panel_fill = panel;
        style.visuals.faint_bg_color = panel;
        style.visuals.override_text_color = Some(text);
        style.visuals.selection.bg_fill = bg;

        // --- Global button visuals ---
        style.visuals.widgets.inactive.weak_bg_fill = button;
        style.visuals.widgets.hovered.weak_bg_fill = button_hover;
        style.visuals.widgets.active.weak_bg_fill = button_hover;

        style.visuals.text_edit_bg_color = Some(button);

        style.visuals.widgets.inactive.fg_stroke.color = button_text;
        style.visuals.widgets.hovered.fg_stroke.color = button_text;
        style.visuals.widgets.active.fg_stroke.color = button_text;

        // Optional: slightly darker borders for contrast
        style.visuals.widgets.noninteractive.bg_stroke.color = accent.linear_multiply(0.3);

        style
    }

    pub fn load_from_file(path: &str) -> Option<Self> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|data| toml::from_str::<Self>(&data).ok())
    }
}
