mod app;
mod can;
mod shortcuts;
mod ui;
mod widgets;
mod workspace;

fn main() -> eframe::Result<()> {
    let (can_sender, can_receiver) = std::sync::mpsc::channel::<can::can_messages::CanMessage>();
    let (ui_sender, ui_receiver) = std::sync::mpsc::channel::<ui::ui_messages::UiMessage>();

    let _can_thread = can::thread::start_can_thread(can_sender, ui_receiver);

    eframe::run_native(
        "DaqApp2",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(app::DAQApp::new(can_receiver, ui_sender)))),
    )
}
