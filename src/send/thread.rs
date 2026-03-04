use crate::send::{self, messages};

use std::{thread, time::Duration};



pub fn start_send_thread(
    send_sender: std::sync::mpsc::Sender<send::messages::FromSendThread>,
	send_receiver: std::sync::mpsc::Receiver<send::messages::ToSendThread>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {



	})
}