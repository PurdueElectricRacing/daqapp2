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
        let mut state = can::state::State::new(can_sender, ui_receiver);

        loop { 
            for msg in state.ui_receiver.try_iter() {
                match msg {
                    ui::ui_messages::UiMessage::DbcSelected(path) => {
                        match can_decode::Parser::from_dbc_file(&path) {
                            Ok(parser) => {
                                state.parser = Some(parser);
                                println!("Loaded DBC from {:?}", path);
                            }
                            Err(e) => {
                                eprintln!("Failed to load DBC from {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }

            todo!();
        }
    })
}
