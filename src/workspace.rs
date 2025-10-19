use crate::{ui, widgets};
use eframe::egui;

pub fn show(app: &mut crate::app::DAQApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.tile_tree.is_empty() {
            ui.vertical_centered(|ui| {
                ui.label("No widgets in workspace yet.");
                ui.label("CMD+S to toggle the sidebar.");
                ui.label("Use the sidebar to spawn widgets.");
            });
        } else {
            let mut behavior = WorkspaceTileBehavior {
                ui_sender: &app.ui_sender,
            };
            app.tile_tree.ui(&mut behavior, ui);
        }
    });
}

struct WorkspaceTileBehavior<'a> {
    ui_sender: &'a std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
}

impl egui_tiles::Behavior<widgets::Widget> for WorkspaceTileBehavior<'_> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        widget: &mut widgets::Widget,
    ) -> egui_tiles::UiResponse {
        widget.show(ui, self.ui_sender)
    }

    fn tab_title_for_pane(&mut self, widget: &widgets::Widget) -> egui::WidgetText {
        widget.title().into()
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }

    fn is_tab_closable(
        &self,
        _tiles: &egui_tiles::Tiles<widgets::Widget>,
        _tile_id: egui_tiles::TileId,
    ) -> bool {
        true
    }
}
