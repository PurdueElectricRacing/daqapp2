use crate::can::CanDriver;
use serialport::SerialPort;
use slcan::sync::CanSocket;
use slcan::{CanFrame, NominalBitRate, OperatingMode, ReadError};
use std::{io, time::Duration};

pub struct SerialDriver {
    socket: Option<CanSocket<Box<dyn SerialPort>>>,
    path: String,
    baud_rate: u32,
}

impl SerialDriver {
    pub fn new(path: String, baud_rate: u32) -> Self {
        Self {
            socket: None,
            path,
            baud_rate,
        }
    }

    fn try_connect(&mut self) -> io::Result<()> {
        let port = serialport::new(&self.path, self.baud_rate)
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open: {}", e)))?;

        let mut socket = CanSocket::new(port);
        socket
            .set_operating_mode(OperatingMode::Normal)
            .map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Failed to set mode: {:?}", e))
            })?;
        socket.open(NominalBitRate::Rate500Kbit).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to open SLCAN: {:?}", e),
            )
        })?;

        self.socket = Some(socket);
        Ok(())
    }
}

impl CanDriver for SerialDriver {
    fn read_frame(&mut self) -> io::Result<CanFrame> {
        if self.socket.is_none() {
            self.try_connect()?;
        }

        if let Some(ref mut socket) = self.socket {
            match socket.read() {
                Ok(frame) => Ok(frame),
                Err(ReadError::Io(e)) => {
                    if e.kind() != io::ErrorKind::WouldBlock && e.kind() != io::ErrorKind::TimedOut
                    {
                        self.socket = None;
                    }
                    Err(e)
                }
                Err(e) => Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("SLCAN Error: {:?}", e),
                )),
            }
        } else {
            Err(io::Error::new(io::ErrorKind::NotConnected, "Not connected"))
        }
    }

    fn write_frame(&mut self, _frame: &CanFrame) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Write not supported yet",
        ))
    }

    fn is_connected(&self) -> bool {
        self.socket.is_some()
    }
}
