mod app;
mod can;
mod config;
mod shortcuts;
mod ui;
mod widgets;
mod workspace;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let (can_sender, can_receiver) = std::sync::mpsc::channel::<can::can_messages::CanMessage>();
    let (ui_sender, ui_receiver) = std::sync::mpsc::channel::<ui::ui_messages::UiMessage>();

    let _can_thread = can::thread::start_can_thread(can_sender, ui_receiver);

    eframe::run_native(
        "DaqApp2",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(app::DAQApp::new(can_receiver, ui_sender, cc)))),
    )
}
