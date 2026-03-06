mod action;
mod app;
mod can;
mod connection;
mod settings;
mod shortcuts;
mod theme;
mod ui;
mod util;
mod widgets;
mod workspace;

// fn main() -> eframe::Result<()> {
    
//     env_logger::Builder::from_default_env()
//         .filter_level(log::LevelFilter::Info)
//         .init();

//     let (can_sender, can_receiver) = std::sync::mpsc::channel::<can::can_messages::CanMessage>();
//     let (ui_sender, ui_receiver) = std::sync::mpsc::channel::<ui::ui_messages::UiMessage>();

//     let settings = settings::Settings::load();
//     if let Some(ref selected_source) = settings.selected_source {
//         ui_sender
//             .send(ui::ui_messages::UiMessage::Connect(selected_source.clone()))
//             .expect("Failed to send connect message to CAN thread");
//     }

//     let _can_thread =
//         can::thread::start_can_thread(can_sender, ui_receiver, settings.selected_source.clone());

//     eframe::run_native(
//         "DaqApp2",
//         eframe::NativeOptions::default(),
//         Box::new(|cc| {
//             Ok(Box::new(app::DAQApp::new(
//                 can_receiver,
//                 ui_sender,
//                 settings,
//                 cc,
//             )))
//         }),
//     )
// }

// fn main() {
//     use slcan::{CanFrame, Id, Can2Frame};;

//     let id = Id::Standard(slcan::StandardId::new(0x40).unwrap());
//     let data = [0x01, 0x02];
     
//     let mut data8 = [0u8; 8];
//     data8[..data.len()].copy_from_slice(&data);
//     let frame2 = Can2Frame::new_data(id, &data8).expect("Failed to create Can2Frame");
//     let can_frame = CanFrame::Can2(frame2);

//     let mut log = can::thread::Logger::new(None);

//     can::thread::log_frame(&can_frame, &mut log);
// }

fn main() {
    use slcan::{CanFrame, Id, Can2Frame};

    let messages = vec![
        (0x40, [0x01, 0x02]),
        (0x41, [0xAA, 0xBB]),
        (0x42, [0xFF, 0x01]),
        (0x42, [0xFF, 0x01]),
    ];

    let mut log = can::thread::Logger::new(None);

    for (id_val, data) in messages {
        let id = Id::Standard(slcan::StandardId::new(id_val).unwrap());

        let mut data8 = [0u8; 8];
        data8[..data.len()].copy_from_slice(&data);

        let frame2 = Can2Frame::new_data(id, &data8).expect("Failed to create Can2Frame");
        let can_frame = CanFrame::Can2(frame2);

        can::thread::log_frame(&can_frame, &mut log);
    }
}
