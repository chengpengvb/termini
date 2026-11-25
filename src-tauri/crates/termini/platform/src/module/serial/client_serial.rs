use anyhow::anyhow;
use std::sync::Arc;
use summer::tracing::info;
use tokio::time::Duration;
use crate::module::common::util::task_hub::TaskGroup;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use crate::module::common::client_terminal::EventHandler;
use crate::model::resp::event::resp_event_serial::RespEventSerial;
use crate::module::common::context::context_serial::ContextSerial;
use protocol_serial::{SerialConfig, SerialFlowControl, SerialParity, SerialSession};

const READER_JOIN_WAIT: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub struct ClientSerial {
    tasks: TaskGroup,
    session: Arc<AsyncMutex<Option<Arc<SerialSession>>>>,
    close_tx: Arc<parking_lot::Mutex<Option<oneshot::Sender<()>>>>,
}

impl ClientSerial {
    pub fn new() -> Self {
        Self { session: Arc::new(AsyncMutex::new(None)), close_tx: Arc::new(parking_lot::Mutex::new(None)), tasks: TaskGroup::new() }
    }

    pub async fn open(&self, context: Arc<ContextSerial>, handler: EventHandler<RespEventSerial>) -> anyhow::Result<()> {
        {
            let guard = self.session.lock().await;
            if guard.is_some() {
                return Err(anyhow!("serial already opened"));
            }
        }
        let serial_config = Self::build_serial_config(&context)?;
        let port_name = serial_config.port.clone();
        let opened = SerialSession::open(&serial_config);
        let (session, data_rx) = match opened {
            Ok(pair) => pair,
            Err(error) => {
                let message = format!("Failed to open serial {}: {}", port_name, error);
                let _ = handler(RespEventSerial::OnError(message.clone())).await;
                return Err(anyhow!(message));
            }
        };

        let (close_tx, close_rx) = oneshot::channel::<()>();
        *self.close_tx.lock() = Some(close_tx);
        if let Some(reader) = session.take_reader() {
            self.tasks.adopt_os(reader);
        }
        {
            let mut guard = self.session.lock().await;
            *guard = Some(Arc::new(session));
        }
        Self::spawn_dispatch(&self.tasks, handler, data_rx, close_rx);
        info!("[serial] 串口打开 [{}] 成功.", port_name);
        Ok(())
    }

    pub async fn write(&self, data: String) -> anyhow::Result<()> {
        let guard = self.session.lock().await;
        let session = guard.as_ref().ok_or_else(|| anyhow!("Session not found or already closed"))?;
        session.write(data.as_bytes())?;
        Ok(())
    }

    pub async fn close(&self) -> anyhow::Result<()> {
        let session = {
            let mut guard = self.session.lock().await;
            guard.take()
        };
        if let Some(session) = session {
            session.close();
        }
        if let Some(close_tx) = self.close_tx.lock().take() {
            let _ = close_tx.send(());
        }
        self.tasks.shutdown_within(READER_JOIN_WAIT).await;
        Ok(())
    }

    fn build_serial_config(context: &ContextSerial) -> anyhow::Result<SerialConfig> {
        let config = context.config.read();
        let parity = SerialParity::from_str_loose(config.parity.as_str())?;
        let flow_control = SerialFlowControl::from_str_loose(config.flow_control.as_str())?;
        Ok(SerialConfig {
            port: config.port_name.clone(),
            baud_rate: u32::try_from(config.baud_rate).map_err(|_| anyhow!("invalid baud rate: {}", config.baud_rate))?,
            data_bits: u8::try_from(config.data_bits).map_err(|_| anyhow!("invalid data bits: {}", config.data_bits))?,
            stop_bits: u8::try_from(config.stop_bits).map_err(|_| anyhow!("invalid stop bits: {}", config.stop_bits))?,
            parity,
            flow_control,
        })
    }

    fn spawn_dispatch(tasks: &TaskGroup, handler: EventHandler<RespEventSerial>, mut data_rx: mpsc::UnboundedReceiver<Vec<u8>>, mut close_rx: oneshot::Receiver<()>) {
        tasks.spawn("serial-dispatch", async move {
            let _ = handler(RespEventSerial::OnOpen).await;
            loop {
                tokio::select! {
                    _ = &mut close_rx => {
                        let _ = handler(RespEventSerial::OnClose).await;
                        break;
                    }
                    maybe_data = data_rx.recv() => {
                        let Some(data) = maybe_data else {
                            let _ = handler(RespEventSerial::OnClose).await;
                            break;
                        };
                        let _ = handler(RespEventSerial::OnData(data)).await;
                    }
                }
            }
        });
    }
}
