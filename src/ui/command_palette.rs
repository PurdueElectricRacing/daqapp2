use crate::{action, app};
use eframe::egui;

pub fn show(app: &mut app::DAQApp, ctx: &egui::Context) {
    // 1. Data Definitions
    let master_options = [
        ("Spawn CAN Table", action::WidgetType::ViewerTable),
        ("Spawn CAN List", action::WidgetType::ViewerList),
        ("Spawn Scope", action::WidgetType::Scope { msg_id: 0, msg_name: "".into(), signal_name: "".into() }),
        ("Spawn Bootloader", action::WidgetType::Bootloader),
        ("Spawn Log Parser", action::WidgetType::LogParser),
    ];

    // IDs for persistent UI state
    let index_id = egui::Id::new("command_palette_index");
    let search_id = egui::Id::new("command_palette_search");

    // Load state from previous frame
    let mut index = ctx.data(|d| d.get_temp::<usize>(index_id).unwrap_or(0));
    let mut search_query = ctx.data(|d| d.get_temp::<String>(search_id).unwrap_or_default());

    // Exit early if not showing
    if !app.show_command_palette { 
        if !search_query.is_empty() || index != 0 {
            ctx.data_mut(|d| {
                d.insert_temp(index_id, 0);
                d.insert_temp(search_id, String::new());
            });
        }
        return; 
    }

    // 2. Filtering Logic
    let filtered_options: Vec<_> = master_options
        .iter()
        .filter(|(label, _)| {
            if search_query.is_empty() {
                true
            } else {
                label.to_lowercase().contains(&search_query.to_lowercase())
            }
        })
        .collect();

    // Clamp index to filtered list size
    if !filtered_options.is_empty() {
        index = index.min(filtered_options.len() - 1);
    }

    // 3. Keyboard Input
    ctx.input_mut(|i| {
        if i.key_pressed(egui::Key::ArrowDown) && !filtered_options.is_empty() {
            index = (index + 1) % filtered_options.len();
        }
        if i.key_pressed(egui::Key::ArrowUp) && !filtered_options.is_empty() {
            index = (index + filtered_options.len() - 1) % filtered_options.len();
        }
        if i.key_pressed(egui::Key::Enter) && !filtered_options.is_empty() {
            app.action_queue.push(action::AppAction::SpawnWidget(filtered_options[index].1.clone()));
            app.show_command_palette = false;
        }
        if i.key_pressed(egui::Key::Escape) {
            app.show_command_palette = false;
        }
    });

    // 4. Clean Themed UI
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
                let response = ui.text_edit_singleline(&mut search_query);
                
                // Keep focus in the search box while open
                response.request_focus();

                if response.changed() {
                    index = 0; // Reset selection on type
                }
            });
            ui.separator();

            // Results List
            ui.vertical_centered_justified(|ui| {
                if filtered_options.is_empty() {
                    ui.label("No commands found...");
                } else {
                    for (i, (label, widget_type)) in filtered_options.iter().enumerate() {
                        let is_selected = i == index;
                        let selection = ui.visuals().selection;

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

    // Save state for next frame
    ctx.data_mut(|d| {
        d.insert_temp(index_id, index);
        d.insert_temp(search_id, search_query);
    });
}
