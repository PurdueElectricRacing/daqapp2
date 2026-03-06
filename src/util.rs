pub fn get_available_serial_ports() -> Vec<serialport::SerialPortInfo> {
    match serialport::available_ports() {
        Ok(ports) => ports
            .into_iter()
            .filter(|p| {
                let name = p.port_name.to_lowercase();
                if cfg!(target_os = "windows") {
                    name.starts_with("com")
                } else {
                    name.starts_with("/dev/tty.usbmodem") || name.starts_with("/dev/ttyacm")
                }
            })
            .collect(),
        Err(err) => {
            log::error!("Error listing serial ports: {}", err);
            Vec::new()
        }
    }
}

pub fn msg_id_as_u32(msg_id: &can_dbc::MessageId) -> u32 {
    match msg_id {
        can_dbc::MessageId::Standard(id) => *id as u32,
        can_dbc::MessageId::Extended(id) => *id,
    }
}
