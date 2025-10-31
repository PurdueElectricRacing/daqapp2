use eframe::egui;

#[derive(Debug, PartialEq)]
pub enum ShortcutAction {
    ToggleSidebar,
    CloseActiveWidget,
    IncreaseScale,
    DecreaseScale,
}

pub struct ShortcutHandler;

impl ShortcutHandler {
    pub fn check_shortcuts(ctx: &egui::Context) -> Vec<ShortcutAction> {
        let mut actions = Vec::new();

        // Get input state
        let input = ctx.input(|i| i.clone());

        // CMD+S = toggle sidebar
        if input.modifiers.command_only() && input.key_pressed(egui::Key::S) {
            actions.push(ShortcutAction::ToggleSidebar);
        }

        // CMD+W = close window
        if input.modifiers.command_only() && input.key_pressed(egui::Key::W) {
            actions.push(ShortcutAction::CloseActiveWidget);
        }

        // CMD+Plus = increase scale
        if input.modifiers.command_only() && input.key_pressed(egui::Key::Equals) {
            actions.push(ShortcutAction::IncreaseScale);
        }

        // CMD+Minus = decrease scale
        if input.modifiers.command_only() && input.key_pressed(egui::Key::Minus) {
            actions.push(ShortcutAction::DecreaseScale);
        }

        actions
    }
}
