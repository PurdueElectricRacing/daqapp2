use crate::send;

use std::{thread, time::Duration};

pub fn start_send_thread(
    ui_to_send_rx: std::sync::mpsc::Receiver<send::messages::ToSendThread>,
    send_to_ui_tx: std::sync::mpsc::Sender<send::messages::FromSendThreadToUi>,
    send_to_can_tx: std::sync::mpsc::Sender<send::messages::FromSendThreadToCan>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {})
}
