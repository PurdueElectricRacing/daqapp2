use crate::{can, ui};

pub fn start_can_thread(
    can_sender: std::sync::mpsc::Sender<can::can_messages::CanMessage>,
    ui_receiver: std::sync::mpsc::Receiver<ui::ui_messages::UiMessage>,
) -> std::thread::JoinHandle<()> {


    // TODO: start thread
    // connect to CAN vs SLCAN/serial
    // read from CAN and send to can_sender
    // UI receiver -> set up can can_decode::Parser based on the path to the DBC

    std::thread::spawn(move || {
        todo!()
    })
}
