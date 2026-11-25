use crate::ber::{
    BerReader, BerWriter, TAG_COUNTER32, TAG_COUNTER64, TAG_END_OF_MIB_VIEW, TAG_GAUGE32, TAG_INTEGER, TAG_IP_ADDRESS, TAG_NO_SUCH_INSTANCE, TAG_NO_SUCH_OBJECT, TAG_NULL, TAG_OCTET_STRING, TAG_OID,
    TAG_OPAQUE, TAG_SEQUENCE, TAG_TIME_TICKS,
};
use crate::error::SnmpError;
use crate::value::{Oid, SnmpValue};

pub const SNMP_VERSION_1: i64 = 0;
pub const SNMP_VERSION_2C: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PduType {
    GetRequest,
    GetNextRequest,
    Response,
    GetBulkRequest,
}

impl PduType {
    pub fn tag(&self) -> u8 {
        match self {
            PduType::GetRequest => 0xA0,
            PduType::GetNextRequest => 0xA1,
            PduType::Response => 0xA2,
            PduType::GetBulkRequest => 0xA5,
        }
    }

    pub fn from_tag(tag: u8) -> Result<Self, SnmpError> {
        match tag {
            0xA0 => Ok(PduType::GetRequest),
            0xA1 => Ok(PduType::GetNextRequest),
            0xA2 => Ok(PduType::Response),
            0xA5 => Ok(PduType::GetBulkRequest),
            _ => Err(SnmpError::Decode(format!("unknown PDU tag: 0x{:02X}", tag))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VarBind {
    pub oid: Oid,
    pub value: SnmpValue,
}

#[derive(Debug, Clone)]
pub struct Pdu {
    pub pdu_type: PduType,
    pub request_id: i32,
    pub error_status: i32,
    pub error_index: i32,
    pub varbinds: Vec<VarBind>,
}

pub fn encode_message(version: i64, community: &str, pdu: &Pdu) -> Vec<u8> {
    let mut varbind_list = BerWriter::new();
    for varbind in &pdu.varbinds {
        let mut entry = BerWriter::new();
        entry.write_oid(&varbind.oid);
        write_value(&mut entry, &varbind.value);
        varbind_list.write_wrapped(TAG_SEQUENCE, entry);
    }

    let mut pdu_writer = BerWriter::new();
    pdu_writer.write_integer(pdu.request_id as i64);
    pdu_writer.write_integer(pdu.error_status as i64);
    pdu_writer.write_integer(pdu.error_index as i64);
    pdu_writer.write_wrapped(TAG_SEQUENCE, varbind_list);

    let mut message = BerWriter::new();
    message.write_integer(version);
    message.write_octet_string(community.as_bytes());
    message.write_wrapped(pdu.pdu_type.tag(), pdu_writer);

    let mut root = BerWriter::new();
    root.write_wrapped(TAG_SEQUENCE, message);
    root.into_bytes()
}

pub fn decode_message(data: &[u8]) -> Result<(i64, String, Pdu), SnmpError> {
    let mut root = BerReader::new(data);
    let message_content = root.expect_tlv(TAG_SEQUENCE)?;
    let mut message = BerReader::new(message_content);

    let version = message.read_integer()?;
    let community_bytes = message.expect_tlv(TAG_OCTET_STRING)?;
    let community = String::from_utf8_lossy(community_bytes).to_string();

    let (pdu_tag, pdu_content) = message.read_tlv()?;
    let pdu_type = PduType::from_tag(pdu_tag)?;
    let mut pdu_reader = BerReader::new(pdu_content);

    let request_id = pdu_reader.read_integer()? as i32;
    let error_status = pdu_reader.read_integer()? as i32;
    let error_index = pdu_reader.read_integer()? as i32;

    let varbind_list = pdu_reader.expect_tlv(TAG_SEQUENCE)?;
    let mut list_reader = BerReader::new(varbind_list);
    let mut varbinds = Vec::new();
    while list_reader.has_remaining() {
        let entry_content = list_reader.expect_tlv(TAG_SEQUENCE)?;
        let mut entry = BerReader::new(entry_content);
        let oid_content = entry.expect_tlv(TAG_OID)?;
        let oid = BerReader::decode_oid(oid_content)?;
        let (value_tag, value_content) = entry.read_tlv()?;
        let value = read_value(value_tag, value_content)?;
        varbinds.push(VarBind { oid, value });
    }

    Ok((version, community, Pdu { pdu_type, request_id, error_status, error_index, varbinds }))
}

fn write_value(writer: &mut BerWriter, value: &SnmpValue) {
    match value {
        SnmpValue::Integer(inner) => writer.write_integer(*inner),
        SnmpValue::OctetString(bytes) => writer.write_octet_string(bytes),
        SnmpValue::Null => writer.write_null(),
        SnmpValue::ObjectIdentifier(oid) => writer.write_oid(oid),
        SnmpValue::IpAddress(addr) => writer.write_tlv(TAG_IP_ADDRESS, addr),
        SnmpValue::Counter32(inner) => writer.write_unsigned(TAG_COUNTER32, *inner as u64),
        SnmpValue::Gauge32(inner) => writer.write_unsigned(TAG_GAUGE32, *inner as u64),
        SnmpValue::TimeTicks(inner) => writer.write_unsigned(TAG_TIME_TICKS, *inner as u64),
        SnmpValue::Opaque(bytes) => writer.write_tlv(TAG_OPAQUE, bytes),
        SnmpValue::Counter64(inner) => writer.write_unsigned(TAG_COUNTER64, *inner),
        SnmpValue::NoSuchObject => writer.write_tlv(TAG_NO_SUCH_OBJECT, &[]),
        SnmpValue::NoSuchInstance => writer.write_tlv(TAG_NO_SUCH_INSTANCE, &[]),
        SnmpValue::EndOfMibView => writer.write_tlv(TAG_END_OF_MIB_VIEW, &[]),
    }
}

fn read_value(tag: u8, content: &[u8]) -> Result<SnmpValue, SnmpError> {
    match tag {
        TAG_INTEGER => Ok(SnmpValue::Integer(BerReader::decode_signed(content))),
        TAG_OCTET_STRING => Ok(SnmpValue::OctetString(content.to_vec())),
        TAG_NULL => Ok(SnmpValue::Null),
        TAG_OID => Ok(SnmpValue::ObjectIdentifier(BerReader::decode_oid(content)?)),
        TAG_IP_ADDRESS => {
            if content.len() != 4 {
                return Err(SnmpError::Decode(format!("invalid IpAddress length: {}", content.len())));
            }
            Ok(SnmpValue::IpAddress([content[0], content[1], content[2], content[3]]))
        }
        TAG_COUNTER32 => Ok(SnmpValue::Counter32(BerReader::decode_unsigned(content) as u32)),
        TAG_GAUGE32 => Ok(SnmpValue::Gauge32(BerReader::decode_unsigned(content) as u32)),
        TAG_TIME_TICKS => Ok(SnmpValue::TimeTicks(BerReader::decode_unsigned(content) as u32)),
        TAG_OPAQUE => Ok(SnmpValue::Opaque(content.to_vec())),
        TAG_COUNTER64 => Ok(SnmpValue::Counter64(BerReader::decode_unsigned(content))),
        TAG_NO_SUCH_OBJECT => Ok(SnmpValue::NoSuchObject),
        TAG_NO_SUCH_INSTANCE => Ok(SnmpValue::NoSuchInstance),
        TAG_END_OF_MIB_VIEW => Ok(SnmpValue::EndOfMibView),
        _ => Err(SnmpError::Decode(format!("unknown value tag: 0x{:02X}", tag))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn message_roundtrip() {
        let pdu = Pdu {
            pdu_type: PduType::GetRequest,
            request_id: 1234,
            error_status: 0,
            error_index: 0,
            varbinds: vec![VarBind { oid: Oid::from_str("1.3.6.1.2.1.1.1.0").unwrap(), value: SnmpValue::Null }],
        };
        let bytes = encode_message(SNMP_VERSION_2C, "public", &pdu);
        let (version, community, decoded) = decode_message(&bytes).unwrap();
        assert_eq!(version, SNMP_VERSION_2C);
        assert_eq!(community, "public");
        assert_eq!(decoded.pdu_type, PduType::GetRequest);
        assert_eq!(decoded.request_id, 1234);
        assert_eq!(decoded.varbinds.len(), 1);
        assert_eq!(decoded.varbinds[0].oid.to_string(), "1.3.6.1.2.1.1.1.0");
    }

    #[test]
    fn known_get_request_bytes() {
        let pdu = Pdu {
            pdu_type: PduType::GetRequest,
            request_id: 0x1B,
            error_status: 0,
            error_index: 0,
            varbinds: vec![VarBind { oid: Oid::from_str("1.3.6.1.2.1.1.5.0").unwrap(), value: SnmpValue::Null }],
        };
        let bytes = encode_message(SNMP_VERSION_1, "public", &pdu);
        let expected: Vec<u8> = vec![
            0x30, 0x26, 0x02, 0x01, 0x00, 0x04, 0x06, 0x70, 0x75, 0x62, 0x6C, 0x69, 0x63, 0xA0, 0x19, 0x02, 0x01, 0x1B, 0x02, 0x01, 0x00, 0x02, 0x01, 0x00, 0x30, 0x0E, 0x30, 0x0C, 0x06, 0x08, 0x2B,
            0x06, 0x01, 0x02, 0x01, 0x01, 0x05, 0x00, 0x05, 0x00,
        ];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn response_with_values_roundtrip() {
        let pdu = Pdu {
            pdu_type: PduType::Response,
            request_id: 7,
            error_status: 0,
            error_index: 0,
            varbinds: vec![
                VarBind { oid: Oid::from_str("1.3.6.1.2.1.2.2.1.10.1").unwrap(), value: SnmpValue::Counter32(4294967295) },
                VarBind { oid: Oid::from_str("1.3.6.1.2.1.31.1.1.1.6.1").unwrap(), value: SnmpValue::Counter64(18446744073709551615) },
                VarBind { oid: Oid::from_str("1.3.6.1.2.1.1.5.0").unwrap(), value: SnmpValue::OctetString(b"server-01".to_vec()) },
                VarBind { oid: Oid::from_str("1.3.6.1.2.1.4.20.1.1.10.0.0.1").unwrap(), value: SnmpValue::IpAddress([10, 0, 0, 1]) },
                VarBind { oid: Oid::from_str("1.3.6.1.2.1.1.9.1.2.1").unwrap(), value: SnmpValue::EndOfMibView },
            ],
        };
        let bytes = encode_message(SNMP_VERSION_2C, "private", &pdu);
        let (_, _, decoded) = decode_message(&bytes).unwrap();
        assert_eq!(decoded.varbinds.len(), 5);
        assert_eq!(decoded.varbinds[0].value, SnmpValue::Counter32(4294967295));
        assert_eq!(decoded.varbinds[1].value, SnmpValue::Counter64(18446744073709551615));
        assert_eq!(decoded.varbinds[2].value, SnmpValue::OctetString(b"server-01".to_vec()));
        assert_eq!(decoded.varbinds[3].value, SnmpValue::IpAddress([10, 0, 0, 1]));
        assert_eq!(decoded.varbinds[4].value, SnmpValue::EndOfMibView);
    }
}
