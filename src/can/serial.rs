use serialport::SerialPort;
use slcan::sync::CanSocket;
use slcan::{CanFrame, NominalBitRate, OperatingMode, ReadError};
use std::{io, time::Duration};

pub struct SerialDriver {
    socket: CanSocket<Box<dyn SerialPort>>,
}

impl SerialDriver {
    pub fn new(path: String, baud_rate: u32) -> io::Result<Self> {
        let port = serialport::new(&path, baud_rate)
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open: {}", e)))?;

        let mut socket = CanSocket::new(port);
        socket.set_operating_mode(OperatingMode::Normal).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to set mode: {:?}", e))
        })?;
        socket.open(NominalBitRate::Rate500Kbit).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to open SLCAN: {:?}", e),
            )
        })?;

        Ok(Self { socket })
    }

    pub fn read_frame(&mut self) -> io::Result<CanFrame> {
        match self.socket.read() {
            Ok(frame) => Ok(frame),
            Err(ReadError::Io(e)) => Err(e),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("SLCAN Error: {:?}", e),
            )),
        }
    }
}
