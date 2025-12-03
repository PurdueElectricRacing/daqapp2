use crate::{can, ui, widgets};
use eframe::egui;

pub fn show(app: &mut crate::app::DAQApp, ctx: &egui::Context) {
    let rounding = if cfg!(target_os = "macos") {
        egui::CornerRadius {
            nw: 0,
            ne: 12,
            sw: 0,
            se: 12,
        }
    } else {
        egui::CornerRadius::ZERO
    };

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(ctx.style().visuals.window_fill())
                .corner_radius(rounding)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            if app.tile_tree.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.label("No widgets in workspace yet.");
                    ui.label("CMD+S to toggle the sidebar.");
                    ui.label("Use the sidebar to spawn widgets.");
                });
            } else {
                let mut behavior = WorkspaceTileBehavior {
                    can_receiver: &app.can_receiver,
                    ui_sender: &app.ui_sender,
                    pending_scope_spawns: &mut app.pending_scope_spawns,
                };
                app.tile_tree.ui(&mut behavior, ui);

                // Spawn all pending scopes in the queue
                for (msg_id, msg_name, signal_name) in std::mem::take(&mut app.pending_scope_spawns)
                {
                    app.spawn_scope(msg_id, msg_name, signal_name);
                }
            }
        });
}

struct WorkspaceTileBehavior<'a> {
    can_receiver: &'a std::sync::mpsc::Receiver<can::can_messages::CanMessage>,
    ui_sender: &'a std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
    pending_scope_spawns: &'a mut Vec<(u32, String, String)>,
}

impl egui_tiles::Behavior<widgets::Widget> for WorkspaceTileBehavior<'_> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        widget: &mut widgets::Widget,
    ) -> egui_tiles::UiResponse {
        widget.show(
            ui,
            self.can_receiver,
            self.ui_sender,
            self.pending_scope_spawns,
        )
    }

    fn tab_title_for_pane(&mut self, widget: &widgets::Widget) -> egui::WidgetText {
        widget.title().into()
    }

    fn tab_bar_color(&self, visuals: &egui::Visuals) -> egui::Color32 {
        visuals.window_fill
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
