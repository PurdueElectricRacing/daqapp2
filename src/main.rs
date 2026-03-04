mod action;
mod app;
mod can;
mod connection;
mod send;
mod settings;
mod shortcuts;
mod theme;
mod ui;
mod util;
mod widgets;
mod workspace;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let (can_to_ui_tx, can_to_ui_rx) = std::sync::mpsc::channel::<can::can_messages::CanMessage>();
    let (ui_to_can_tx, ui_to_can_rx) = std::sync::mpsc::channel::<ui::ui_messages::UiMessage>();
    
    let (ui_to_send_tx, ui_to_send_rx) = std::sync::mpsc::channel::<send::messages::ToSendThread>();
    let (send_to_ui_tx, send_to_ui_rx) = std::sync::mpsc::channel::<send::messages::FromSendThreadToUi>();
    let (send_to_can_tx, send_to_can_rx) = std::sync::mpsc::channel::<send::messages::FromSendThreadToCan>();
    
    let settings = settings::Settings::load();
    if let Some(ref selected_source) = settings.selected_source {
        ui_to_can_tx
            .send(ui::ui_messages::UiMessage::Connect(selected_source.clone()))
            .expect("Failed to send connect message to CAN thread");
    }

    let _can_thread =
        can::thread::start_can_thread(can_to_ui_tx, ui_to_can_rx, settings.selected_source.clone());

    eframe::run_native(
        "DaqApp2",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            Ok(Box::new(app::DAQApp::new(
                can_to_ui_rx,
                ui_to_can_tx,
                settings,
                cc,
            )))
        }),
    )
}
