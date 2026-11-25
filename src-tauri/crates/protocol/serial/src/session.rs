use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::Duration;
use crate::error::SerialError;
use std::thread::JoinHandle as OsJoinHandle;
use crate::config::{SerialConfig, SerialFlowControl, SerialParity};
use serial2::{CharSize, FlowControl, Parity, SerialPort, StopBits};

const READ_BUF_SIZE: usize = 4096;
const READ_TIMEOUT: Duration = Duration::from_millis(50);

pub struct SerialSession {
    port: Arc<SerialPort>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    reader: std::sync::Mutex<Option<OsJoinHandle<()>>>,
}

impl SerialSession {
    pub fn open(config: &SerialConfig) -> Result<(Self, mpsc::UnboundedReceiver<Vec<u8>>), SerialError> {
        config.validate()?;
        let char_size = CharSize::try_from(config.data_bits).map_err(|_| SerialError::InvalidConfig(format!("invalid data bits: {}", config.data_bits)))?;
        let stop_bits = StopBits::try_from(config.stop_bits).map_err(|_| SerialError::InvalidConfig(format!("invalid stop bits: {}", config.stop_bits)))?;
        let parity = match config.parity {
            SerialParity::None => Parity::None,
            SerialParity::Odd => Parity::Odd,
            SerialParity::Even => Parity::Even,
        };
        let flow_control = match config.flow_control {
            SerialFlowControl::None => FlowControl::None,
            SerialFlowControl::Software => FlowControl::XonXoff,
            SerialFlowControl::Hardware => FlowControl::RtsCts,
        };
        let baud_rate = config.baud_rate;

        let mut port = SerialPort::open(&config.port, move |mut settings: serial2::Settings| {
            settings.set_baud_rate(baud_rate)?;
            settings.set_char_size(char_size);
            settings.set_stop_bits(stop_bits);
            settings.set_parity(parity);
            settings.set_flow_control(flow_control);
            Ok(settings)
        })
        .map_err(|error| SerialError::Unavailable(format!("{}: {}", config.port, error)))?;
        port.set_read_timeout(READ_TIMEOUT)?;

        let port = Arc::new(port);
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (tx, rx) = mpsc::unbounded_channel();

        let read_port = port.clone();
        let read_stop = stop.clone();
        let reader = std::thread::spawn(move || {
            let mut buf = [0u8; READ_BUF_SIZE];
            while !read_stop.load(std::sync::atomic::Ordering::Relaxed) {
                match read_port.read(&mut buf) {
                    Ok(0) => continue,
                    Ok(size) => {
                        if tx.send(buf[..size].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::TimedOut => continue,
                    Err(_) => break,
                }
            }
        });

        Ok((SerialSession { port, stop, reader: std::sync::Mutex::new(Some(reader)) }, rx))
    }

    pub fn write(&self, data: &[u8]) -> Result<(), SerialError> {
        let mut offset = 0;
        while offset < data.len() {
            let written = self.port.write(&data[offset..])?;
            if written == 0 {
                return Err(SerialError::Io(std::io::Error::new(std::io::ErrorKind::WriteZero, "serial port wrote 0 bytes")));
            }
            offset += written;
        }
        Ok(())
    }

    pub fn close(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn take_reader(&self) -> Option<OsJoinHandle<()>> {
        match self.reader.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        }
    }
}

impl Drop for SerialSession {
    fn drop(&mut self) {
        self.close();
    }
}
