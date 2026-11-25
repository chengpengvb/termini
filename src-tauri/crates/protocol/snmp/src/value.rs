use std::fmt;
use std::str::FromStr;
use crate::error::SnmpError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Oid(pub Vec<u32>);

impl Oid {
    pub fn starts_with(&self, prefix: &Oid) -> bool {
        self.0.len() >= prefix.0.len() && self.0[..prefix.0.len()] == prefix.0[..]
    }
}

impl FromStr for Oid {
    type Err = SnmpError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let trimmed = raw.trim().trim_start_matches('.');
        if trimmed.is_empty() {
            return Err(SnmpError::InvalidOid(raw.to_string()));
        }
        let mut arcs = Vec::new();
        for part in trimmed.split('.') {
            let arc = part.parse::<u32>().map_err(|_| SnmpError::InvalidOid(raw.to_string()))?;
            arcs.push(arc);
        }
        if arcs.len() < 2 || arcs[0] > 2 || (arcs[0] < 2 && arcs[1] >= 40) {
            return Err(SnmpError::InvalidOid(raw.to_string()));
        }
        Ok(Oid(arcs))
    }
}

impl fmt::Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self.0.iter().map(|arc| arc.to_string()).collect::<Vec<_>>().join(".");
        write!(f, "{}", text)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SnmpValue {
    Integer(i64),
    OctetString(Vec<u8>),
    Null,
    ObjectIdentifier(Oid),
    IpAddress([u8; 4]),
    Counter32(u32),
    Gauge32(u32),
    TimeTicks(u32),
    Opaque(Vec<u8>),
    Counter64(u64),
    NoSuchObject,
    NoSuchInstance,
    EndOfMibView,
}

impl SnmpValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            SnmpValue::Integer(value) => Some(*value as f64),
            SnmpValue::Counter32(value) => Some(*value as f64),
            SnmpValue::Gauge32(value) => Some(*value as f64),
            SnmpValue::TimeTicks(value) => Some(*value as f64),
            SnmpValue::Counter64(value) => Some(*value as f64),
            _ => None,
        }
    }

    pub fn is_exception(&self) -> bool {
        matches!(self, SnmpValue::NoSuchObject | SnmpValue::NoSuchInstance | SnmpValue::EndOfMibView)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            SnmpValue::Integer(_) => "INTEGER",
            SnmpValue::OctetString(_) => "STRING",
            SnmpValue::Null => "NULL",
            SnmpValue::ObjectIdentifier(_) => "OID",
            SnmpValue::IpAddress(_) => "IpAddress",
            SnmpValue::Counter32(_) => "Counter32",
            SnmpValue::Gauge32(_) => "Gauge32",
            SnmpValue::TimeTicks(_) => "TimeTicks",
            SnmpValue::Opaque(_) => "Opaque",
            SnmpValue::Counter64(_) => "Counter64",
            SnmpValue::NoSuchObject => "noSuchObject",
            SnmpValue::NoSuchInstance => "noSuchInstance",
            SnmpValue::EndOfMibView => "endOfMibView",
        }
    }
}

impl fmt::Display for SnmpValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnmpValue::Integer(value) => write!(f, "{}", value),
            SnmpValue::OctetString(bytes) => match std::str::from_utf8(bytes) {
                Ok(text) => write!(f, "{}", text),
                Err(_) => {
                    for byte in bytes {
                        write!(f, "{:02X} ", byte)?;
                    }
                    Ok(())
                }
            },
            SnmpValue::Null => write!(f, "NULL"),
            SnmpValue::ObjectIdentifier(oid) => write!(f, "{}", oid),
            SnmpValue::IpAddress(addr) => write!(f, "{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3]),
            SnmpValue::Counter32(value) => write!(f, "{}", value),
            SnmpValue::Gauge32(value) => write!(f, "{}", value),
            SnmpValue::TimeTicks(value) => write!(f, "{}", value),
            SnmpValue::Opaque(bytes) => {
                for byte in bytes {
                    write!(f, "{:02X} ", byte)?;
                }
                Ok(())
            }
            SnmpValue::Counter64(value) => write!(f, "{}", value),
            SnmpValue::NoSuchObject => write!(f, "noSuchObject"),
            SnmpValue::NoSuchInstance => write!(f, "noSuchInstance"),
            SnmpValue::EndOfMibView => write!(f, "endOfMibView"),
        }
    }
}
