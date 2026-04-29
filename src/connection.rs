#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum ConnectionSource {
    Serial(String),
    Udp(u16),
    Simulated(bool, Option<std::path::PathBuf>), // true for connected, false for disconnected, path to dbc file for sim
    Loopback,
}

impl ConnectionSource {
    pub fn display_name(&self) -> String {
        match self {
            ConnectionSource::Serial(path) => format!("Serial: {}", path),
            ConnectionSource::Udp(port) => format!("UDP: {}", port),
            ConnectionSource::Simulated(connected, _) => {
                if *connected {
                    "Simulated (connected)".into()
                } else {
                    "Simulated (disconnected)".into()
                }
            }
            ConnectionSource::Loopback => "Loopback".into(),
        }
    }
}
