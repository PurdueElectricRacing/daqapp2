mod action;
mod app;
mod can;
mod connection;
mod settings;
mod shortcuts;
mod theme;
mod ui;
mod util;
mod widgets;
mod workspace;

use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let (can_sender, can_receiver) = std::sync::mpsc::channel::<can::can_messages::CanMessage>();
    let (ui_sender, ui_receiver) = std::sync::mpsc::channel::<ui::ui_messages::UiMessage>();
    let settings = settings::Settings::load();

    let _can_thread = can::thread::start_can_thread(
        can_sender,
        ui_receiver,
        settings.selected_source.clone(),
        settings.dbc_path.clone(),
    );

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(egui::viewport::WindowLevel::Normal),
        ..Default::default()
    };

    eframe::run_native(
        "DaqApp2",
        options,
        Box::new(|cc| {
            Ok(Box::new(app::DAQApp::new(
                can_receiver,
                ui_sender,
                settings,
                cc,
            )))
        }),
    )
}
