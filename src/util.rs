pub fn get_avaible_serial_ports() -> Vec<serialport::SerialPortInfo> {
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
            panic!("Failed to get serial ports: {err}");
        }
    }
}
