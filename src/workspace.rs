use crate::app::DAQApp;
use crate::widgets::{AppAction, Widget};
use eframe::egui;
use std::{collections::VecDeque, path::PathBuf};

pub fn show(app: &mut DAQApp, ctx: &egui::Context) {
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
                .inner_margin(10.0),
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
                    action_queue: &mut app.action_queue,
                    dbc_path: app.dbc_path.as_ref(),
                };
                app.tile_tree.ui(&mut behavior, ui);

                // Drain and process all pending actions
                while let Some(action) = app.action_queue.pop_front() {
                    app.handle_action(action);
                }
            }
        });
}

struct WorkspaceTileBehavior<'a> {
    action_queue: &'a mut VecDeque<AppAction>,
    dbc_path: Option<&'a PathBuf>,
}

impl egui_tiles::Behavior<Widget> for WorkspaceTileBehavior<'_> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        widget: &mut Widget,
    ) -> egui_tiles::UiResponse {
        widget.show(ui, self.action_queue, self.dbc_path)
    }

    fn tab_title_for_pane(&mut self, widget: &Widget) -> egui::WidgetText {
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
        _tiles: &egui_tiles::Tiles<Widget>,
        _tile_id: egui_tiles::TileId,
    ) -> bool {
        true
    }
}
