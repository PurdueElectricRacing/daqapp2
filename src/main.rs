mod action;
mod app;
mod can;
mod connection;
mod messages;
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

    let (can_to_ui_tx, can_to_ui_rx) = std::sync::mpsc::channel::<messages::MsgFromCan>();
    let (ui_to_can_tx, ui_to_can_rx) = std::sync::mpsc::channel::<messages::MsgFromUi>();

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
