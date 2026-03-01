use crate::{action, app};
use eframe::egui;

pub fn show(app: &mut app::DAQApp, ctx: &egui::Context) {
    if !app.show_command_palette {
        return;
    }

    // 1. Centralized Options Definition
    let options = [
        ("Spawn CAN Table", action::WidgetType::ViewerTable),
        ("Spawn CAN List", action::WidgetType::ViewerList),
        (
            "Spawn Scope",
            action::WidgetType::Scope {
                msg_id: 0,
                msg_name: "".into(),
                signal_name: "".into(),
            },
        ),
        ("Spawn Bootloader", action::WidgetType::Bootloader),
        ("Spawn Log Parser", action::WidgetType::LogParser),
    ];

    let id = egui::Id::new("command_palette_selection");
    let mut index = ctx.data_mut(|d| d.get_temp::<usize>(id).unwrap_or(0));

    // 2. Simplified Keyboard Logic (Consuming events to prevent side effects)
    ctx.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
            index = (index + 1) % options.len();
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
            index = (index + options.len() - 1) % options.len();
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
            let (_, widget_type) = &options[index];
            app.action_queue
                .push(action::AppAction::SpawnWidget(widget_type.clone()));
            app.show_command_palette = false;
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
            app.show_command_palette = false;
        }
    });

    // Save selection for next frame
    ctx.data_mut(|d| d.insert_temp(id, index));

    // 3. Clean UI Rendering
    egui::Window::new("Command Palette")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .fixed_size([300.0, 400.0])
        .show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading("Command Palette");
                ui.separator();

                for (i, (label, widget_type)) in options.iter().enumerate() {
                    let is_selected = i == index;

                    // Concise button styling that respects themes
                    let btn = egui::Button::new(*label)
                        .fill(if is_selected {
                            ui.visuals().widgets.hovered.bg_fill
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .stroke(if is_selected {
                            ui.visuals().widgets.hovered.bg_stroke
                        } else {
                            egui::Stroke::NONE
                        });

                    if ui.add(btn).clicked() {
                        app.action_queue
                            .push(action::AppAction::SpawnWidget(widget_type.clone()));
                        app.show_command_palette = false;
                    }
                }
            });
        });
}
