use std::sync::Arc;
use tokio::io::WriteHalf;
use tokio::io::AsyncWriteExt;
use crate::module::rdp::protocol::gdi::gfx;
use crate::module::rdp::protocol::core::mcs;
use crate::module::rdp::protocol::core::pdu;
use crate::module::rdp::protocol::codec::avc;
use crate::module::rdp::protocol::core::info;
use crate::module::rdp::protocol::core::nego;
use crate::module::rdp::protocol::core::rdpgfx;
use crate::module::rdp::protocol::core::errinfo;
use crate::module::rdp::protocol::gdi::gfx::GfxSurfaces;
use crate::module::common::client_terminal::EventHandler;
use crate::module::rdp::protocol::codec::bulk::BulkContext;
use crate::module::rdp::protocol::core::crypt::RdpSecurity;
use crate::module::rdp::protocol::core::stream::ReadCursor;
use crate::model::resp::event::resp_event_rdp::RespEventRdp;
use crate::module::rdp::protocol::core::rdpsnd::RdpsndState;
use crate::module::rdp::protocol::core::error::RdpProtocolError;
use crate::module::rdp::protocol::gdi::framebuffer::Framebuffer;
use crate::module::rdp::client_session_factory::RdpSessionFactory;
use crate::module::rdp::protocol::codec::zgfx::{self, ZgfxContext};
use crate::module::rdp::protocol::core::fastpath::FastPathAssembler;
use crate::module::rdp::client_rdp_session::{RdpChannel, RdpSettings};
use crate::module::rdp::protocol::core::channel::{self, ChannelTable};
use crate::module::rdp::protocol::core::drdynvc::{self, DynamicChannels};
use crate::module::rdp::protocol::core::transport::{self, RdpFrame, RdpStream};
use crate::module::rdp::protocol::core::msgchannel::{self, MessageChannelState};

const HOST: &str = "192.168.72.129";
const USER: &str = "cp";
const WIN8_HOST: &str = "192.168.72.130";
const WIN8_USER: &str = "chengpeng";
fn rdp_test_password() -> String {
    std::env::var("TERMINI_TEST_RDP_PASSWORD").expect("TERMINI_TEST_RDP_PASSWORD not set")
}
const GFX_HEADER: u32 = 8;
const CMD_WIRE_1: u16 = 0x0001;
const CMD_WIRE_2: u16 = 0x0002;
const CMD_RESET: u16 = 0x000e;
const CMD_CAPS_CONFIRM: u16 = 0x0013;
const AVC_DISABLED: u32 = 0x0000_0020;
const CAPVERSION_10: u32 = 0x000A_0000;

struct Probe {
    channel: RdpChannel,
    channels: ChannelTable,
    security: RdpSecurity,
    dynamic: DynamicChannels,
    message: MessageChannelState,
    bulk: BulkContext,
    assembler: FastPathAssembler,
    framebuffer: Framebuffer,
    surfaces: GfxSurfaces,
    gfx_zgfx: ZgfxContext,
    sound: RdpsndState,
    gfx_pdus: usize,
    gfx_process_errors: usize,
    avc_packets: usize,
    avc_pack_ok: usize,
    avc_pack_err: usize,
    last_codec: u16,
    caps_version: u32,
    caps_flags: u32,
    disable_avc: bool,
    error_info: Option<u32>,
    deactivate: bool,
}

fn hex_head(data: &[u8], count: usize) -> String {
    data.iter().take(count).map(|byte| format!("{:02x}", byte)).collect::<Vec<_>>().join(" ")
}

fn annexb(data: &[u8]) -> bool {
    data.starts_with(&[0, 0, 0, 1]) || data.starts_with(&[0, 0, 1])
}

fn caps_advertise(disable_avc: bool) -> Vec<u8> {
    let mut data = rdpgfx::build_caps_advertise();
    if !disable_avc {
        return data;
    }
    let mut offset = GFX_HEADER as usize + 2;
    while offset + 12 <= data.len() {
        let version = u32::from_le_bytes(data[offset..offset + 4].try_into().expect("version"));
        let caps_len = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().expect("caps_len")) as usize;
        if version >= CAPVERSION_10 {
            let flags = u32::from_le_bytes(data[offset + 8..offset + 12].try_into().expect("flags"));
            data[offset + 8..offset + 12].copy_from_slice(&(flags | AVC_DISABLED).to_le_bytes());
        }
        offset += 8 + caps_len;
    }
    data
}

fn inspect_avc(probe: &mut Probe, codec: u16, data: &[u8]) {
    probe.avc_packets += 1;
    probe.last_codec = codec;
    let packed = if codec == gfx::CODEC_AVC420 {
        match avc::parse_avc420(data) {
            Ok(stream) => eprintln!("avc420 rects={} bits={} annexb={} head=[{}]", stream.rects.len(), stream.bitstream.len(), annexb(stream.bitstream), hex_head(stream.bitstream, 16)),
            Err(error) => eprintln!("avc420 parse err={}", error.message),
        }
        avc::pack_avc420_passthrough(data)
    } else {
        match avc::parse_avc444(data) {
            Ok((lc, first, second)) => {
                let chroma = match &second {
                    Some(stream) => stream.bitstream.len(),
                    None => 0,
                };
                eprintln!(
                    "avc444 v2={} lc={} luma_rects={} luma_bits={} luma_annexb={} chroma_bits={} luma_head=[{}]",
                    codec == gfx::CODEC_AVC444V2,
                    lc,
                    first.rects.len(),
                    first.bitstream.len(),
                    annexb(first.bitstream),
                    chroma,
                    hex_head(first.bitstream, 16)
                );
            }
            Err(error) => eprintln!("avc444 parse err={}", error.message),
        }
        avc::pack_avc444_passthrough(codec == gfx::CODEC_AVC444V2, data)
    };
    match packed {
        Ok(payload) => {
            probe.avc_pack_ok += 1;
            eprintln!("avc pack ok codec=0x{:04x} out={} flag=0x{:02x}", codec, payload.len(), payload[0]);
        }
        Err(error) => {
            probe.avc_pack_err += 1;
            eprintln!("avc pack err codec=0x{:04x} {}", codec, error.message);
        }
    }
}

fn dump_wire(probe: &mut Probe, wire: u8, body: &mut ReadCursor<'_>) -> Result<(), RdpProtocolError> {
    let surface = body.read_u16_le()?;
    let codec = body.read_u16_le()?;
    if wire == 1 {
        let _ = body.read_u8()?;
        let left = body.read_u16_le()?;
        let top = body.read_u16_le()?;
        let right = body.read_u16_le()?;
        let bottom = body.read_u16_le()?;
        let size = body.read_u32_le()?;
        eprintln!("gfx wire1 surface={} codec=0x{:04x} dest=({},{})-({},{}) data={}", surface, codec, left, top, right, bottom, size);
        let data = body.read_slice(size as usize)?;
        if codec == gfx::CODEC_AVC420 || codec == gfx::CODEC_AVC444 || codec == gfx::CODEC_AVC444V2 {
            inspect_avc(probe, codec, data);
        }
    } else {
        body.skip(4)?;
        let _ = body.read_u8()?;
        let size = body.read_u32_le()?;
        eprintln!("gfx wire2 surface={} codec=0x{:04x} data={}", surface, codec, size);
        let data = body.read_slice(size as usize)?;
        if codec == gfx::CODEC_AVC420 || codec == gfx::CODEC_AVC444 || codec == gfx::CODEC_AVC444V2 {
            inspect_avc(probe, codec, data);
        }
    }
    Ok(())
}

fn dump_gfx(probe: &mut Probe, plain: &[u8]) -> Result<(), RdpProtocolError> {
    let mut cursor = ReadCursor::new(plain);
    while cursor.remaining() >= GFX_HEADER as usize {
        let command = cursor.read_u16_le()?;
        cursor.skip(2)?;
        let length = cursor.read_u32_le()?;
        if length < GFX_HEADER {
            return Err(RdpProtocolError::protocol(format!("gfx dump short length={}", length)));
        }
        let mut body = cursor.sub_cursor(length as usize - GFX_HEADER as usize)?;
        match command {
            CMD_CAPS_CONFIRM => {
                probe.caps_version = body.read_u32_le()?;
                let _caps_len = body.read_u32_le()?;
                probe.caps_flags = body.read_u32_le()?;
                eprintln!("gfx caps-confirm version=0x{:08x} flags=0x{:08x} avc_disabled={}", probe.caps_version, probe.caps_flags, probe.caps_flags & AVC_DISABLED != 0);
            }
            CMD_WIRE_1 => dump_wire(probe, 1, &mut body)?,
            CMD_WIRE_2 => dump_wire(probe, 2, &mut body)?,
            CMD_RESET => eprintln!("gfx reset {}x{} monitors={}", body.read_u32_le()?, body.read_u32_le()?, body.read_u32_le()?),
            other => eprintln!("gfx cmd=0x{:04x} len={}", other, length),
        }
    }
    Ok(())
}

impl Probe {
    async fn write_channel(&mut self, writer: &mut WriteHalf<RdpStream>, channel_id: u16, body: &[u8]) -> Result<(), RdpProtocolError> {
        let wrapped = self.security.wrap(0, body)?;
        let bytes = mcs::build_send_data_request(self.channel.user_id, channel_id, &wrapped)?;
        writer.write_all(&bytes).await.map_err(|error| RdpProtocolError::protocol(format!("write failed: {}", error)))?;
        writer.flush().await.map_err(|error| RdpProtocolError::protocol(format!("flush failed: {}", error)))?;
        Ok(())
    }

    async fn write_static(&mut self, writer: &mut WriteHalf<RdpStream>, channel_id: u16, body: &[u8]) -> Result<(), RdpProtocolError> {
        for packet in self.channels.split(channel_id, body)? {
            self.write_channel(writer, channel_id, &packet).await?;
        }
        Ok(())
    }

    async fn write_dynamic(&mut self, writer: &mut WriteHalf<RdpStream>, name: &str, payload: &[u8]) -> Result<(), RdpProtocolError> {
        let dynamic_id = self.dynamic.channel_id(name).ok_or_else(|| RdpProtocolError::protocol(format!("dynamic channel {} missing", name)))?;
        let channel_id = self.channels.channel_id(channel::DRDYNVC_CHANNEL_NAME).ok_or_else(|| RdpProtocolError::protocol("drdynvc missing".to_string()))?;
        self.write_static(writer, channel_id, &drdynvc::build_data(dynamic_id, payload)?).await
    }

    async fn handle_fastpath(&mut self, flags: u8, payload: &[u8]) -> Result<(), RdpProtocolError> {
        let payload = self.security.unwrap_fastpath(flags, payload)?;
        let _ = self.assembler.push(&payload, &mut self.bulk)?;
        Ok(())
    }

    async fn handle_x224(&mut self, writer: &mut WriteHalf<RdpStream>, payload: &[u8]) -> Result<(), RdpProtocolError> {
        let indication = mcs::parse_send_data_indication(payload)?;
        let payload = self.security.unwrap(&indication.payload)?;
        if indication.channel_id == self.channel.global_channel_id {
            let mut cursor = ReadCursor::new(&payload);
            let header = pdu::read_share_control_header(&mut cursor)?;
            eprintln!("x224 pdu_type=0x{:02x}", header.pdu_type);
            if header.pdu_type == pdu::PDU_TYPE_DEACTIVATE_ALL {
                self.deactivate = true;
                eprintln!("deactivate-all");
                return Ok(());
            }
            if header.pdu_type != pdu::PDU_TYPE_DATA {
                return Ok(());
            }
            let data = pdu::read_share_data_header(&mut cursor)?;
            let body = pdu::read_data_pdu_body(&mut cursor, &data, &mut self.bulk)?;
            eprintln!("x224 pdu_type2={} body_len={} head=[{}]", data.pdu_type2, body.len(), hex_head(&body, 16));
            if data.pdu_type2 == pdu::PDU_TYPE2_SET_ERROR_INFO {
                let code = u32::from_le_bytes(body[..4].try_into().expect("errorInfo"));
                if code == 0 {
                    eprintln!("set-error-info 0 ignored");
                    return Ok(());
                }
                self.error_info = Some(code);
                return Err(RdpProtocolError::rejected(errinfo::describe(code)));
            }
            return Ok(());
        }
        if self.channel.message_channel_id != 0 && indication.channel_id == self.channel.message_channel_id {
            if let Some(response) = msgchannel::process(&mut self.message, &payload)? {
                return self.write_channel(writer, self.channel.message_channel_id, &response).await;
            }
            return Ok(());
        }
        let chunk = channel::parse_chunk(&payload)?;
        let name = self.channels.channel_name(indication.channel_id).ok_or_else(|| RdpProtocolError::protocol(format!("unknown channel id={}", indication.channel_id)))?;
        let Some(message) = self.channels.reassemble(indication.channel_id, chunk) else {
            return Ok(());
        };
        match name {
            channel::DRDYNVC_CHANNEL_NAME => self.handle_dynamic(writer, indication.channel_id, &message).await,
            channel::RDPSND_CHANNEL_NAME => {
                for response in &crate::module::rdp::protocol::core::rdpsnd::process(&mut self.sound, &message)?.responses {
                    self.write_static(writer, indication.channel_id, response).await?;
                }
                Ok(())
            }
            channel::CLIPRDR_CHANNEL_NAME | channel::RDPDR_CHANNEL_NAME => Ok(()),
            other => Err(RdpProtocolError::protocol(format!("static channel {} unimplemented", other))),
        }
    }

    async fn handle_dynamic(&mut self, writer: &mut WriteHalf<RdpStream>, channel_id: u16, message: &[u8]) -> Result<(), RdpProtocolError> {
        let outcome = drdynvc::process(&mut self.dynamic, message)?;
        for response in &outcome.responses {
            self.write_static(writer, channel_id, response).await?;
        }
        if outcome.created == Some(rdpgfx::CHANNEL_NAME) {
            eprintln!("gfx channel created disable_avc={}", self.disable_avc);
            self.write_dynamic(writer, rdpgfx::CHANNEL_NAME, &caps_advertise(self.disable_avc)).await?;
        }
        let Some(message) = outcome.message else {
            return Ok(());
        };
        if message.name != rdpgfx::CHANNEL_NAME {
            eprintln!("dynvc {} len={}", message.name, message.payload.len());
            return Ok(());
        }
        let plain = zgfx::decompress(&mut self.gfx_zgfx, &message.payload)?;
        eprintln!("gfx zgfx {} -> {} head=[{}]", message.payload.len(), plain.len(), hex_head(&plain, 24));
        if let Err(error) = dump_gfx(self, &plain) {
            eprintln!("gfx dump err={}", error.message);
        }
        match rdpgfx::process(&mut self.surfaces, &mut self.framebuffer, &plain) {
            Ok(outcome) => {
                self.gfx_pdus += 1;
                let patches = self.surfaces.take_avc_patches();
                if !patches.is_empty() {
                    eprintln!("avc patches={} last_codec=0x{:04x}", patches.len(), self.surfaces.last_codec_id);
                }
                for ack in outcome.responses {
                    self.write_dynamic(writer, rdpgfx::CHANNEL_NAME, &ack).await?;
                }
                Ok(())
            }
            Err(error) => {
                self.gfx_process_errors += 1;
                eprintln!("rdpgfx.process err={}", error.message);
                Ok(())
            }
        }
    }
}

fn settings(host: &str, user: &str) -> RdpSettings {
    RdpSettings {
        host: host.to_string(),
        port: 3389,
        domain: String::new(),
        username: user.to_string(),
        password: rdp_test_password(),
        ignore_cert: true,
        console_session: false,
        color_depth: 32,
        keyboard_layout: 0x0409,
        performance_flags: info::default_performance_flags(),
        requested_protocols: nego::PROTOCOL_HYBRID_EX | nego::PROTOCOL_HYBRID | nego::PROTOCOL_SSL,
        connect_timeout_seconds: 15,
        keepalive_interval_seconds: 0,
        desktop_width: 1280,
        desktop_height: 750,
        desktop_dpi: 0,
        dynamic_desktop: true,
        drive_path: None,
        aad_client_id: None,
        aad_tenant: None,
        routing_token: None,
        redirection_password: None,
        redirection_guid: None,
        redirection_session_id: 0,
        auto_reconnect: None,
    }
}

async fn probe_once(host: &str, user: &str, disable_avc: bool) {
    eprintln!("probe host={} disable_avc={}", host, disable_avc);
    let handler: EventHandler<RespEventRdp> = Arc::new(|_| Box::pin(async { Ok(()) }));
    let connected = RdpSessionFactory.connect(&settings(host, user), None, &handler, None).await.expect("rdp connect");
    eprintln!("connected desktop={}x{}", connected.desktop.width, connected.desktop.height);
    let mut assembler = FastPathAssembler::default();
    assembler.set_max_request_size(connected.multifragment_update.unwrap_or_default());
    let mut probe = Probe {
        channel: connected.channel,
        channels: connected.channels,
        security: connected.security,
        dynamic: DynamicChannels::default(),
        message: connected.message,
        bulk: connected.bulk,
        assembler,
        framebuffer: Framebuffer::new(connected.desktop.width, connected.desktop.height),
        surfaces: GfxSurfaces::default(),
        gfx_zgfx: ZgfxContext::default(),
        sound: RdpsndState::default(),
        gfx_pdus: 0,
        gfx_process_errors: 0,
        avc_packets: 0,
        avc_pack_ok: 0,
        avc_pack_err: 0,
        last_codec: 0,
        caps_version: 0,
        caps_flags: 0,
        disable_avc,
        error_info: None,
        deactivate: false,
    };
    let (mut reader, mut writer) = connected.transport.split();
    for index in 0..80 {
        let frame = match tokio::time::timeout(std::time::Duration::from_secs(4), transport::read_frame(&mut reader)).await {
            Ok(Ok(frame)) => frame,
            Ok(Err(error)) => {
                eprintln!("read[{}] {}", index, error.message);
                break;
            }
            Err(_) => {
                eprintln!("read[{}] timeout", index);
                break;
            }
        };
        let result = match &frame {
            RdpFrame::FastPath { flags, payload } => probe.handle_fastpath(*flags, payload).await,
            RdpFrame::X224(payload) => probe.handle_x224(&mut writer, payload).await,
        };
        if let Err(error) = result {
            eprintln!("frame[{}] {}", index, error.message);
            break;
        }
    }
    eprintln!(
        "summary host={} disable_avc={} gfx_pdus={} gfx_err={} avc_packets={} pack_ok={} pack_err={} last_codec=0x{:04x} caps=0x{:08x} flags=0x{:08x} confirm_avc_disabled={} deactivate={} error_info={:?}",
        host,
        disable_avc,
        probe.gfx_pdus,
        probe.gfx_process_errors,
        probe.avc_packets,
        probe.avc_pack_ok,
        probe.avc_pack_err,
        probe.last_codec,
        probe.caps_version,
        probe.caps_flags,
        probe.caps_flags & AVC_DISABLED != 0,
        probe.deactivate,
        probe.error_info
    );
}

#[tokio::test]
async fn connect_win11_avc_probe() {
    let data_dir = std::env::temp_dir().join(format!("termini-rdp-test-{}", HOST));
    std::fs::create_dir_all(&data_dir).expect("create temp data dir");
    std::env::set_var("TERMINI_DATA_DIR", &data_dir);
    probe_once(HOST, USER, false).await;
    probe_once(HOST, USER, true).await;
}

#[tokio::test]
async fn connect_win8_drop_probe() {
    let data_dir = std::env::temp_dir().join(format!("termini-rdp-test-{}", WIN8_HOST));
    std::fs::create_dir_all(&data_dir).expect("create temp data dir");
    std::env::set_var("TERMINI_DATA_DIR", &data_dir);
    probe_once(WIN8_HOST, WIN8_USER, false).await;
    probe_once(WIN8_HOST, WIN8_USER, true).await;
}
