mod app;
mod can;
mod config;
mod shortcuts;
mod ui;
mod widgets;
mod workspace;

use eframe::icon_data::from_png_bytes;
use eframe::egui;



fn main() -> eframe::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let (can_sender, can_receiver) = std::sync::mpsc::channel::<can::can_messages::CanMessage>();
    let (ui_sender, ui_receiver) = std::sync::mpsc::channel::<ui::ui_messages::UiMessage>();

    let _can_thread = can::thread::start_can_thread(can_sender, ui_receiver);
    
    let icon = from_png_bytes(include_bytes!("../images/PER_Logo.png"))
        .expect("Failed to load app icon");
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_icon(icon),
        ..Default::default()
    };


    eframe::run_native(
        "DaqApp2",
        options,
        Box::new(|cc| Ok(Box::new(app::DAQApp::new(can_receiver, ui_sender, cc)))),
    )

}
