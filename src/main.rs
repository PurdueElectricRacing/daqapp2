use crate::app::Settings;
use crate::can::can_messages::CanMessage;
use std::sync::mpsc::channel;

mod app;
mod can;
mod config;
mod shortcuts;
mod ui;
mod widgets;
mod workspace;

use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let (can_sender, can_receiver) = channel::<CanMessage>();
    let _settings = Settings::load(app::SETTINGS_PATH);

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
        Box::new(|cc| Ok(Box::new(app::DAQApp::new(can_receiver, can_sender, cc)))),
    )
}
