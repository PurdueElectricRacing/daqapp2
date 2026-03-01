use crate::{action, app, can, widgets};
use eframe::egui;

pub fn show(app: &mut app::DAQApp, ctx: &egui::Context) {
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
                    can_messages: &app.can_messages,
                    action_queue: &mut app.action_queue,
                    parser: app.parser.as_ref(),
                };
                app.tile_tree.ui(&mut behavior, ui);
            }
        });
}

struct WorkspaceTileBehavior<'a> {
    can_messages: &'a [can::message::ParsedMessage],
    action_queue: &'a mut Vec<action::AppAction>,
    parser: Option<&'a app::ParserInfo>,
}

impl egui_tiles::Behavior<widgets::Widget> for WorkspaceTileBehavior<'_> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        widget: &mut widgets::Widget,
    ) -> egui_tiles::UiResponse {
        widget.show(ui, self.can_messages, self.action_queue, self.parser)
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
