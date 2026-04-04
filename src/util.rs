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

// Linear interpolation helper
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

// HSV → egui::Color32
pub fn hsv_to_color32(h: f64, s: f64, v: f64) -> eframe::egui::Color32 {
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    let r = ((r1 + m) * 255.0) as u8;
    let g = ((g1 + m) * 255.0) as u8;
    let b = ((b1 + m) * 255.0) as u8;

    eframe::egui::Color32::from_rgb(r, g, b)
}

pub mod msg_id {
    const EXTENDED_ID_FLAG: u32 = 0x80000000;
    pub const STANDARD_ID_MASK: u32 = 0x7FF;
    pub const EXTENDED_ID_MASK: u32 = 0x1FFFFFFF;

    // Converts a can_dbc::MessageId to a u32, setting the extended ID flag if it's an extended ID.
    // The extended ID flag is the highest bit (32nd bit) of the u32.
    // Standard IDs (11 bits) will have this bit unset, while extended IDs (29 bits) will have this bit set.
    // Generally use this version when interfacing with `can_decode`.
    pub fn can_dbc_to_u32_with_extid_flag(msg_id: &can_dbc::MessageId) -> u32 {
        match msg_id {
            can_dbc::MessageId::Standard(id) => *id as u32,
            can_dbc::MessageId::Extended(id) => *id | EXTENDED_ID_FLAG,
        }
    }

    // Converts a can_dbc::MessageId to a u32 without setting the extended ID flag.
    // Ex: a 29-bit extended ID will use at most 29 bits while the other version of this function
    // would set the highest bit to indicate it's an extended ID.
    // Generally use this version when showing output to the user or logging.
    pub fn can_dbc_to_u32_without_extid_flag(msg_id: &can_dbc::MessageId) -> u32 {
        match msg_id {
            can_dbc::MessageId::Standard(id) => *id as u32 & STANDARD_ID_MASK,
            can_dbc::MessageId::Extended(id) => *id & EXTENDED_ID_MASK,
        }
    }

    // Converts a slcan::Id to a u32, setting the extended ID flag if it's an extended ID.
    // Similar to the can_dbc version, but for slcan::Id.
    // Likewise, generally use this version when interfacing with `can_decode`.
    pub fn slcan_to_u32_with_extid_flag(id: &slcan::Id) -> u32 {
        match id {
            slcan::Id::Standard(sid) => sid.as_raw() as u32,
            slcan::Id::Extended(eid) => eid.as_raw() | EXTENDED_ID_FLAG,
        }
    }

    // Converts a slcan::Id to a u32 without setting the extended ID flag.
    // Similar to the can_dbc version, but for slcan::Id.
    // Likewise, generally use this version when showing output to the user or logging.
    pub fn slcan_to_u32_without_extid_flag(id: &slcan::Id) -> u32 {
        match id {
            slcan::Id::Standard(sid) => sid.as_raw() as u32 & STANDARD_ID_MASK,
            slcan::Id::Extended(eid) => eid.as_raw() & EXTENDED_ID_MASK,
        }
    }
}

// highest bit in current broadcast from elcon indicates discharge or charge
// highest bit 1 = dischargiing, 0 = charging
pub fn is_discharging_from_current(raw_current: u16) -> bool {
    (raw_current & 0x8000) != 0
}

pub fn extract_current_from_raw(raw_current: u16) -> u16 {
    raw_current & 0x7FFF
}