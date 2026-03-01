use crate::{action, app};
use eframe::egui;

pub fn show(app: &mut app::DAQApp, ctx: &egui::Context) {
    if !app.show_command_palette { return; }

    let cmd_list = [
        ("Spawn CAN Table", action::WidgetType::ViewerTable),
        ("Spawn CAN List", action::WidgetType::ViewerList),
        ("Spawn Bootloader", action::WidgetType::Bootloader),
        ("Spawn Log Parser", action::WidgetType::LogParser),
    ];

    // filtering Logic
    let filtered_options: Vec<_> = cmd_list
        .iter()
        .filter(|(label, _)| {
            if app.palette_search.is_empty() {
                true
            } else {
                label.to_lowercase().contains(&app.palette_search.to_lowercase())
            }
        })
        .collect();

    if !filtered_options.is_empty() {
        app.palette_index = app.palette_index.min(filtered_options.len() - 1);
    }

    // handle keyboard navigation (maybe needs to be smarter?)
    ctx.input_mut(|i| {
        if i.key_pressed(egui::Key::ArrowDown) && !filtered_options.is_empty() {
            app.palette_index = (app.palette_index + 1) % filtered_options.len();
        }
        if i.key_pressed(egui::Key::ArrowUp) && !filtered_options.is_empty() {
            app.palette_index = (app.palette_index + filtered_options.len() - 1) % filtered_options.len();
        }
        if i.key_pressed(egui::Key::Enter) && !filtered_options.is_empty() {
            app.action_queue.push(action::AppAction::SpawnWidget(filtered_options[app.palette_index].1.clone()));
            app.show_command_palette = false;
        }
        if i.key_pressed(egui::Key::Escape) {
            app.show_command_palette = false;
        }
    });

    // render UI
    let style = ctx.style();
    egui::Window::new("Command Palette")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false).resizable(false).title_bar(false)
        .frame(egui::Frame::window(&style).inner_margin(12.0))
        .fixed_size([300.0, 400.0])
        .show(ctx, |ui| {
            // Search Input
            ui.horizontal(|ui| {
                ui.label("🔍");
                let response = ui.text_edit_singleline(&mut app.palette_search);
                
                response.request_focus();  // focus the search box when palette is open

                if response.changed() {
                    app.palette_index = 0; // reset selection to top when search changes
                }
            });
            ui.separator();

            // Results List
            ui.vertical_centered_justified(|ui| {
                if filtered_options.is_empty() {
                    ui.label("No commands found");
                } else {
                    for (i, (label, widget_type)) in filtered_options.iter().enumerate() {
                        let is_selected = i == app.palette_index;
                        let selection = ui.visuals().selection;

                        // render style
                        let button = egui::Button::new(*label)
                            .fill(if is_selected { selection.bg_fill } else { egui::Color32::TRANSPARENT })
                            .stroke(if is_selected { selection.stroke } else { egui::Stroke::NONE });

                        let response = ui.add(button);
                        if response.clicked() {
                            app.action_queue.push(action::AppAction::SpawnWidget((*widget_type).clone()));
                            app.show_command_palette = false;
                        }
                    }
                }
            });
        });
}