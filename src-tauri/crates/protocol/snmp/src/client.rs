use tokio::sync::Mutex;
use std::time::Duration;
use tokio::net::UdpSocket;
use crate::value::{Oid, SnmpValue};
use std::sync::atomic::{AtomicI32, Ordering};
use crate::error::{error_status_name, SnmpError, ERROR_STATUS_NO_SUCH_NAME};
use crate::pdu::{decode_message, encode_message, Pdu, PduType, VarBind, SNMP_VERSION_1, SNMP_VERSION_2C};

const RECV_BUF_SIZE: usize = 65535;
const BULK_MAX_REPETITIONS: i32 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnmpVersion {
    V1,
    V2c,
}

impl SnmpVersion {
    pub fn wire(&self) -> i64 {
        match self {
            SnmpVersion::V1 => SNMP_VERSION_1,
            SnmpVersion::V2c => SNMP_VERSION_2C,
        }
    }

    pub fn from_number(value: i32) -> Result<Self, SnmpError> {
        match value {
            1 => Ok(SnmpVersion::V1),
            2 => Ok(SnmpVersion::V2c),
            _ => Err(SnmpError::UnsupportedOperation(format!("unsupported SNMP version: {}", value))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnmpClientConfig {
    pub host: String,
    pub port: u16,
    pub version: SnmpVersion,
    pub community: String,
    pub timeout: Duration,
    pub retries: u32,
}

pub struct SnmpClient {
    socket: UdpSocket,
    config: SnmpClientConfig,
    request_id: AtomicI32,
    request_lock: Mutex<()>,
}

impl SnmpClient {
    pub async fn connect(config: SnmpClientConfig) -> Result<Self, SnmpError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect((config.host.as_str(), config.port)).await?;
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|elapsed| elapsed.subsec_nanos() as i32 & 0x3FFF_FFFF).unwrap_or(1);
        Ok(SnmpClient { socket, config, request_id: AtomicI32::new(seed.max(1)), request_lock: Mutex::new(()) })
    }

    pub async fn get(&self, oids: &[Oid]) -> Result<Vec<VarBind>, SnmpError> {
        let response = self.request(PduType::GetRequest, 0, 0, oids).await?;
        Self::ensure_no_error(&response)?;
        Ok(response.varbinds)
    }

    pub async fn get_next(&self, oids: &[Oid]) -> Result<Vec<VarBind>, SnmpError> {
        let response = self.request(PduType::GetNextRequest, 0, 0, oids).await?;
        Self::ensure_no_error(&response)?;
        Ok(response.varbinds)
    }

    pub async fn get_bulk(&self, non_repeaters: i32, max_repetitions: i32, oids: &[Oid]) -> Result<Vec<VarBind>, SnmpError> {
        if self.config.version != SnmpVersion::V2c {
            return Err(SnmpError::UnsupportedOperation("GETBULK is only supported on v2c".to_string()));
        }
        let response = self.request(PduType::GetBulkRequest, non_repeaters, max_repetitions, oids).await?;
        Self::ensure_no_error(&response)?;
        Ok(response.varbinds)
    }

    pub async fn walk(&self, root: &Oid, max_rows: usize) -> Result<Vec<VarBind>, SnmpError> {
        let mut rows: Vec<VarBind> = Vec::new();
        let mut cursor = root.clone();
        loop {
            let batch = match self.walk_step(&cursor).await {
                Ok(batch) => batch,
                Err(SnmpError::ErrorStatus { status, .. }) if self.config.version == SnmpVersion::V1 && status == ERROR_STATUS_NO_SUCH_NAME => {
                    return Ok(rows);
                }
                Err(error) => return Err(error),
            };
            if batch.is_empty() {
                return Ok(rows);
            }
            for varbind in batch {
                if varbind.value == SnmpValue::EndOfMibView {
                    return Ok(rows);
                }
                if !varbind.oid.starts_with(root) {
                    return Ok(rows);
                }
                if varbind.oid <= cursor {
                    return Ok(rows);
                }
                cursor = varbind.oid.clone();
                rows.push(varbind);
                if rows.len() >= max_rows {
                    return Ok(rows);
                }
            }
        }
    }

    async fn walk_step(&self, cursor: &Oid) -> Result<Vec<VarBind>, SnmpError> {
        let oids = [cursor.clone()];
        match self.config.version {
            SnmpVersion::V2c => self.get_bulk(0, BULK_MAX_REPETITIONS, &oids).await,
            SnmpVersion::V1 => self.get_next(&oids).await,
        }
    }

    async fn request(&self, pdu_type: PduType, error_status: i32, error_index: i32, oids: &[Oid]) -> Result<Pdu, SnmpError> {
        let request_id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let pdu = Pdu { pdu_type, request_id, error_status, error_index, varbinds: oids.iter().map(|oid| VarBind { oid: oid.clone(), value: SnmpValue::Null }).collect() };
        let payload = encode_message(self.config.version.wire(), &self.config.community, &pdu);

        let _guard = self.request_lock.lock().await;
        let mut buf = vec![0u8; RECV_BUF_SIZE];
        for _attempt in 0..=self.config.retries {
            self.socket.send(&payload).await?;
            match tokio::time::timeout(self.config.timeout, self.recv_matching(request_id, &mut buf)).await {
                Ok(result) => return result,
                Err(_elapsed) => continue,
            }
        }
        Err(SnmpError::Timeout)
    }

    async fn recv_matching(&self, request_id: i32, buf: &mut [u8]) -> Result<Pdu, SnmpError> {
        loop {
            let len = self.socket.recv(buf).await?;
            let decoded = match decode_message(&buf[..len]) {
                Ok(decoded) => decoded,
                Err(_) => continue,
            };
            let (_version, _community, pdu) = decoded;
            if pdu.pdu_type == PduType::Response && pdu.request_id == request_id {
                return Ok(pdu);
            }
        }
    }

    fn ensure_no_error(pdu: &Pdu) -> Result<(), SnmpError> {
        if pdu.error_status == 0 {
            return Ok(());
        }
        Err(SnmpError::ErrorStatus { status: pdu.error_status, index: pdu.error_index, name: error_status_name(pdu.error_status) })
    }
}
