use std::sync::Arc;
use std::time::Duration;
use sspi::credssp::ClientState;
use crate::module::rdp::auth::auth_gate;
use crate::module::rdp::protocol::core::gcc;
use crate::module::rdp::protocol::core::mcs;
use crate::module::rdp::protocol::core::pdu;
use crate::module::rdp::protocol::codec::png;
use crate::module::rdp::protocol::core::disp;
use crate::module::rdp::protocol::core::info;
use crate::module::rdp::protocol::core::nego;
use crate::module::rdp::protocol::codec::bulk;
use crate::module::rdp::protocol::codec::jpeg;
use crate::module::rdp::protocol::codec::webp;
use crate::module::rdp::protocol::core::input;
use crate::module::rdp::protocol::core::logon;
use crate::module::rdp::protocol::core::rdpgfx;
use crate::module::rdp::protocol::core::rdpsnd;
use crate::module::rdp::protocol::gdi::painter;
use crate::module::rdp::protocol::codec::ncrush;
use crate::module::rdp::protocol::core::pointer;
use crate::module::rdp::protocol::sspi::credssp;
use crate::module::rdp::protocol::core::fastpath;
use crate::module::rdp::protocol::core::security;
use crate::module::rdp::protocol::core::timezone;
use crate::module::rdp::protocol::core::activation;
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf};
use crate::module::rdp::protocol::core::capabilities;
use crate::module::rdp::protocol::codec::color::Palette;
use crate::module::common::client_terminal::EventHandler;
use crate::module::rdp::protocol::codec::bulk::BulkContext;
use crate::model::resp::event::resp_event_rdp::RespEventRdp;
use crate::module::rdp::protocol::codec::avc::OutputEncoder;
use crate::module::rdp::protocol::core::rdpsnd::RdpsndState;
use crate::module::rdp::protocol::gdi::brush_cache::BrushCache;
use crate::module::rdp::protocol::core::error::RdpProtocolError;
use crate::module::rdp::protocol::core::surface::SurfaceCommand;
use crate::module::rdp::protocol::gdi::gfx::{self, GfxSurfaces};
use crate::module::rdp::protocol::gdi::bitmap_cache::BitmapCache;
use crate::module::rdp::client_session_factory::RdpSessionFactory;
use crate::module::rdp::protocol::codec::zgfx::{self, ZgfxContext};
use crate::module::rdp::protocol::core::crypt::{self, RdpSecurity};
use crate::module::rdp::protocol::core::fastpath::FastPathAssembler;
use crate::module::rdp::client_rdp_drive::{self as drive, DriveState};
use crate::module::rdp::client_rdp_session::{RdpChannel, RdpSettings};
use crate::module::rdp::protocol::core::channel::{self, ChannelTable};
use crate::module::rdp::protocol::core::drdynvc::{self, DynamicChannels};
use crate::model::config_rdp::{SECURITY_AUTO, SECURITY_NLA, SECURITY_TLS};
use crate::module::rdp::protocol::core::stream::{ReadCursor, WriteCursor};
use crate::module::rdp::protocol::gdi::glyph_cache::{CachedGlyph, GlyphCache};
use crate::module::rdp::protocol::core::msgchannel::{self, MessageChannelState};
use crate::module::rdp::protocol::core::orders::{DrawKind, DrawOrder, GlyphOrder};
use crate::module::rdp::protocol::core::rdpdr::{self, DeviceIoRequest, RdpdrState};
use crate::module::rdp::protocol::core::update::{self, DecodedUpdate, UpdateState};
use crate::module::rdp::protocol::core::cliprdr::{self, ClipboardState, LocalImage};
use crate::module::rdp::protocol::core::transport::{self, RdpFrame, RdpStream, RdpTransport, TlsUpgradeResult};
use crate::module::rdp::protocol::gdi::framebuffer::{self, Framebuffer, Rect, FRAME_KIND_JPEG, FRAME_KIND_PNG};

const WIN10_HOST: &str = "192.168.72.128";
const WIN10_USER: &str = "chengpeng";
const WIN11_HOST: &str = "192.168.72.129";
const WIN11_USER: &str = "cp";
const WIN8_HOST: &str = "192.168.72.130";
const WIN7_HOST: &str = "192.168.72.131";
const PORT: u16 = 3389;
fn rdp_test_password() -> String {
    std::env::var("TERMINI_TEST_RDP_PASSWORD").expect("TERMINI_TEST_RDP_PASSWORD not set")
}

fn hex_head(data: &[u8], count: usize) -> String {
    data.iter().take(count).map(|byte| format!("{:02x}", byte)).collect::<Vec<_>>().join(" ")
}

fn settings(host: &str, user: &str) -> RdpSettings {
    RdpSettings {
        host: host.to_string(),
        port: PORT,
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

fn silent_handler() -> EventHandler<RespEventRdp> {
    Arc::new(|_| Box::pin(async { Ok(()) }))
}

struct Probe {
    channel: RdpChannel,
    channels: ChannelTable,
    security: RdpSecurity,
    dynamic: DynamicChannels,
    message: MessageChannelState,
    bulk: BulkContext,
    assembler: FastPathAssembler,
    update_state: UpdateState,
    framebuffer: Framebuffer,
    surfaces: GfxSurfaces,
    gfx_zgfx: ZgfxContext,
    gfx_pdus: usize,
    sound: RdpsndState,
    device: RdpdrState,
    color_depth: u16,
    keyboard_layout: u32,
    expecting_wave: bool,
    sound_types: Vec<u8>,
    server_format_tags: Vec<u16>,
    server_version: u16,
    client_volume: u32,
    client_version: u16,
    client_formats: u16,
    wave_count: usize,
    wave2_count: usize,
    training_count: usize,
    play_sound_count: usize,
    dyn_created: Vec<&'static str>,
    disp_caps: Option<u32>,
    demand_desktops: Vec<(u16, u16)>,
}

impl Probe {
    fn fail(path: &str, data: &[u8], error: RdpProtocolError) -> RdpProtocolError {
        RdpProtocolError::protocol(format!("{} len={} head=[{}] err={}", path, data.len(), hex_head(data, 32), error.message))
    }

    async fn write_channel(&mut self, writer: &mut WriteHalf<RdpStream>, channel_id: u16, body: &[u8]) -> Result<(), RdpProtocolError> {
        let wrapped = self.security.wrap(0, body).map_err(|error| Self::fail("security.wrap", body, error))?;
        let bytes = mcs::build_send_data_request(self.channel.user_id, channel_id, &wrapped).map_err(|error| Self::fail("mcs.build", body, error))?;
        writer.write_all(&bytes).await.map_err(|error| RdpProtocolError::protocol(format!("write failed: {}", error)))?;
        writer.flush().await.map_err(|error| RdpProtocolError::protocol(format!("flush failed: {}", error)))?;
        Ok(())
    }

    async fn write_static(&mut self, writer: &mut WriteHalf<RdpStream>, channel_id: u16, body: &[u8]) -> Result<(), RdpProtocolError> {
        let packets = self.channels.split(channel_id, body).map_err(|error| Self::fail("channel.split", body, error))?;
        for packet in packets {
            self.write_channel(writer, channel_id, &packet).await?;
        }
        Ok(())
    }

    fn apply_decoded(&mut self, decoded: DecodedUpdate) -> Result<(), RdpProtocolError> {
        match decoded {
            DecodedUpdate::Bitmap(rectangles) => {
                for rect in rectangles {
                    self.framebuffer.blit_bitmap(&rect)?;
                }
                Ok(())
            }
            DecodedUpdate::Orders(orders) => {
                for order in orders {
                    let Some(draw) = update::consume_order(order, &mut self.update_state)? else {
                        continue;
                    };
                    painter::apply(&mut self.framebuffer, &draw, 32, &self.update_state.palette, &self.update_state.bitmaps, &self.update_state.brushes, &mut self.update_state.glyphs)?;
                }
                Ok(())
            }
            DecodedUpdate::Cursor(_) => Ok(()),
            DecodedUpdate::Surface(commands) => {
                for command in commands {
                    match command {
                        SurfaceCommand::Bits(rect) => self.framebuffer.blit_bitmap(&rect)?,
                        SurfaceCommand::FrameMarker { .. } => {}
                    }
                }
                Ok(())
            }
        }
    }

    async fn handle_fastpath(&mut self, flags: u8, payload: &[u8]) -> Result<(), RdpProtocolError> {
        let payload = self.security.unwrap_fastpath(flags, payload).map_err(|error| Self::fail("fastpath.unwrap", payload, error))?;
        let updates = self.assembler.push(&payload, &mut self.bulk).map_err(|error| Self::fail("fastpath.assemble", &payload, error))?;
        for update in updates {
            let surface = self.framebuffer.surface();
            let Some(decoded) =
                update::decode_fastpath(&update, &mut self.update_state, surface).map_err(|error| Self::fail(&format!("fastpath.decode type={}", update.update_type), &update.data, error))?
            else {
                continue;
            };
            self.apply_decoded(decoded)?;
        }
        Ok(())
    }

    async fn handle_x224(&mut self, writer: &mut WriteHalf<RdpStream>, payload: &[u8]) -> Result<(), RdpProtocolError> {
        let indication = mcs::parse_send_data_indication(payload).map_err(|error| Self::fail("x224.mcs", payload, error))?;
        let payload = self.security.unwrap(&indication.payload).map_err(|error| Self::fail("x224.unwrap", &indication.payload, error))?;
        eprintln!("x224 channel={} len={} head=[{}]", indication.channel_id, payload.len(), hex_head(&payload, 32));
        if indication.channel_id != self.channel.global_channel_id {
            if self.channel.message_channel_id != 0 && indication.channel_id == self.channel.message_channel_id {
                return self.handle_message(writer, &payload).await;
            }
            return self.handle_channel(writer, indication.channel_id, &payload).await;
        }
        let mut cursor = ReadCursor::new(&payload);
        let header = pdu::read_share_control_header(&mut cursor).map_err(|error| Self::fail("x224.share", &payload, error))?;
        eprintln!("x224 pdu_type=0x{:02x} rest={}", header.pdu_type, cursor.remaining());
        if header.pdu_type == pdu::PDU_TYPE_DEMAND_ACTIVE {
            let demand = activation::parse_demand_active(&mut cursor).map_err(|error| Self::fail("x224.demand", &payload, error))?;
            self.demand_desktops.push((demand.capabilities.desktop_width, demand.capabilities.desktop_height));
            eprintln!("demand desktop={}x{}", demand.capabilities.desktop_width, demand.capabilities.desktop_height);
            return self.reactivate(writer, demand).await;
        }
        if header.pdu_type != pdu::PDU_TYPE_DATA {
            return Ok(());
        }
        let data = pdu::read_share_data_header(&mut cursor).map_err(|error| Self::fail("x224.data_header", &payload, error))?;
        let body = pdu::read_data_pdu_body(&mut cursor, &data, &mut self.bulk).map_err(|error| Self::fail("x224.data_body", &payload, error))?;
        eprintln!("x224 pdu_type2={} body_len={} head=[{}]", data.pdu_type2, body.len(), hex_head(&body, 32));
        if data.pdu_type2 == pdu::PDU_TYPE2_PLAY_SOUND {
            let sound = logon::parse_play_sound(&mut ReadCursor::new(&body)).map_err(|error| Self::fail("x224.play_sound", &body, error))?;
            self.play_sound_count += 1;
            eprintln!("play_sound freq={} duration={}", sound.frequency, sound.duration);
            rdpsnd::play_beep(&mut self.sound, sound.frequency, sound.duration).map_err(|error| Self::fail("rdpsnd.play_beep", &body, error))?;
            return Ok(());
        }
        if data.pdu_type2 == pdu::PDU_TYPE2_UPDATE {
            let mut body = ReadCursor::new(&body);
            let update_type = body.read_u16_le().map_err(|error| Self::fail("x224.update_type", body.rest(), error))?;
            if let Some(decoded) =
                update::decode_slowpath(update_type, &mut body, &mut self.update_state).map_err(|error| Self::fail(&format!("x224.decode update_type={}", update_type), body.rest(), error))?
            {
                self.apply_decoded(decoded)?;
            }
        }
        Ok(())
    }

    async fn handle_message(&mut self, writer: &mut WriteHalf<RdpStream>, payload: &[u8]) -> Result<(), RdpProtocolError> {
        let Some(response) = msgchannel::process(&mut self.message, payload).map_err(|error| Self::fail("msgchannel.process", payload, error))? else {
            return Ok(());
        };
        self.write_channel(writer, self.channel.message_channel_id, &response).await
    }

    async fn handle_channel(&mut self, writer: &mut WriteHalf<RdpStream>, channel_id: u16, payload: &[u8]) -> Result<(), RdpProtocolError> {
        let chunk = channel::parse_chunk(payload).map_err(|error| Self::fail("channel.chunk", payload, error))?;
        let name = self.channels.channel_name(channel_id).ok_or_else(|| RdpProtocolError::protocol(format!("unknown channel id={}", channel_id)))?;
        let Some(message) = self.channels.reassemble(channel_id, chunk) else {
            return Ok(());
        };
        eprintln!("channel {} len={} head=[{}]", name, message.len(), hex_head(&message, 32));
        match name {
            channel::DRDYNVC_CHANNEL_NAME => self.handle_dynamic(writer, channel_id, &message).await,
            channel::CLIPRDR_CHANNEL_NAME => Ok(()),
            channel::RDPDR_CHANNEL_NAME => self.handle_device(writer, channel_id, &message).await,
            channel::RDPSND_CHANNEL_NAME => self.handle_sound(writer, channel_id, &message).await,
            other => Err(RdpProtocolError::protocol(format!("static channel {} unimplemented", other))),
        }
    }

    async fn handle_device(&mut self, writer: &mut WriteHalf<RdpStream>, channel_id: u16, message: &[u8]) -> Result<(), RdpProtocolError> {
        let outcome = rdpdr::process(&mut self.device, "termini", None, message).map_err(|error| Self::fail("rdpdr.process", message, error))?;
        for response in &outcome.responses {
            self.write_static(writer, channel_id, response).await?;
        }
        let Some(request) = outcome.request else {
            return Ok(());
        };
        Err(RdpProtocolError::protocol(format!("rdpdr io request without drive: major=0x{:08x}", request.major)))
    }

    async fn handle_sound(&mut self, writer: &mut WriteHalf<RdpStream>, channel_id: u16, message: &[u8]) -> Result<(), RdpProtocolError> {
        if self.expecting_wave {
            self.wave_count += 1;
            self.expecting_wave = false;
            eprintln!("rdpsnd wave-body len={}", message.len());
        } else if !message.is_empty() {
            let msg_type = message[0];
            self.sound_types.push(msg_type);
            match msg_type {
                2 => self.expecting_wave = true,
                6 => self.training_count += 1,
                7 => {
                    let (version, tags) = parse_rdpsnd_server_formats(message);
                    self.server_version = version;
                    self.server_format_tags = tags;
                }
                13 => self.wave2_count += 1,
                _ => {}
            }
            eprintln!("rdpsnd type={} len={}", msg_type, message.len());
        }
        let outcome = rdpsnd::process(&mut self.sound, message).map_err(|error| Self::fail("rdpsnd.process", message, error))?;
        for response in &outcome.responses {
            if response.first() == Some(&7) && response.len() >= 24 {
                self.client_volume = u32::from_le_bytes(response[8..12].try_into().unwrap());
                self.client_formats = u16::from_le_bytes(response[18..20].try_into().unwrap());
                self.client_version = u16::from_le_bytes(response[21..23].try_into().unwrap());
            }
            self.write_static(writer, channel_id, response).await?;
        }
        Ok(())
    }

    async fn handle_dynamic(&mut self, writer: &mut WriteHalf<RdpStream>, channel_id: u16, message: &[u8]) -> Result<(), RdpProtocolError> {
        let outcome = drdynvc::process(&mut self.dynamic, message).map_err(|error| Self::fail("drdynvc.process", message, error))?;
        for response in &outcome.responses {
            self.write_static(writer, channel_id, response).await?;
        }
        if let Some(created) = outcome.created {
            self.dyn_created.push(created);
            eprintln!("dynvc created {}", created);
        }
        if outcome.created == Some(rdpgfx::CHANNEL_NAME) {
            eprintln!("gfx channel created, sending caps advertise");
            let body =
                drdynvc::build_data(self.dynamic.channel_id(rdpgfx::CHANNEL_NAME).expect("gfx id"), &rdpgfx::build_caps_advertise()).map_err(|error| Self::fail("drdynvc.build_data", &[], error))?;
            self.write_static(writer, channel_id, &body).await?;
        }
        let Some(message) = outcome.message else {
            return Ok(());
        };
        eprintln!("dynvc {} len={} head=[{}]", message.name, message.payload.len(), hex_head(&message.payload, 32));
        if message.name == disp::CHANNEL_NAME {
            self.disp_caps = Some(disp::parse_caps(&message.payload).map_err(|error| Self::fail("disp.caps", &message.payload, error))?);
            return Ok(());
        }
        if message.name != rdpgfx::CHANNEL_NAME {
            return Ok(());
        }
        self.handle_gfx(&message.payload)
    }

    async fn reactivate(&mut self, writer: &mut WriteHalf<RdpStream>, demand: activation::DemandActive) -> Result<(), RdpProtocolError> {
        let input_flags = capabilities::negotiate_input_flags(demand.capabilities.input_flags);
        self.channel.share_id = demand.share_id;
        self.channels.set_chunk_size(demand.capabilities.virtual_channel_chunk_size);
        self.security.set_salted_checksum(demand.capabilities.salted_checksum);
        self.assembler.set_max_request_size(demand.capabilities.multifragment_update.unwrap_or_default());
        self.framebuffer = Framebuffer::new(demand.capabilities.desktop_width, demand.capabilities.desktop_height);
        let capabilities = capabilities::ClientCapabilities {
            node_id: self.channel.user_id,
            color_depth: self.color_depth,
            keyboard_layout: self.keyboard_layout,
            desktop_width: demand.capabilities.desktop_width,
            desktop_height: demand.capabilities.desktop_height,
            virtual_channel_chunk_size: self.channels.chunk_size(),
            input_flags,
            order_support: capabilities::negotiate_order_support(&demand.capabilities.order_support),
            large_pointer: demand.capabilities.large_pointer,
            multifragment_update: demand.capabilities.multifragment_update,
            surface_commands: demand.capabilities.surface_commands,
            bitmap_codecs: demand.capabilities.bitmap_codecs,
            frame_acknowledge: Some(2),
        };
        let confirm = activation::build_confirm_active(demand.share_id, self.channel.user_id, &capabilities).map_err(|error| Self::fail("reactivate.confirm", &[], error))?;
        self.write_channel(writer, self.channel.global_channel_id, &confirm).await?;
        let synchronize = activation::build_synchronize(demand.share_id, self.channel.user_id, self.channel.global_channel_id).map_err(|error| Self::fail("reactivate.sync", &[], error))?;
        self.write_channel(writer, self.channel.global_channel_id, &synchronize).await?;
        let cooperate = activation::build_control(demand.share_id, self.channel.user_id, pdu::CTRL_ACTION_COOPERATE).map_err(|error| Self::fail("reactivate.cooperate", &[], error))?;
        self.write_channel(writer, self.channel.global_channel_id, &cooperate).await?;
        let control = activation::build_control(demand.share_id, self.channel.user_id, pdu::CTRL_ACTION_REQUEST_CONTROL).map_err(|error| Self::fail("reactivate.control", &[], error))?;
        self.write_channel(writer, self.channel.global_channel_id, &control).await?;
        let persist = activation::build_persistent_key_list(demand.share_id, self.channel.user_id, &[Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()])
            .map_err(|error| Self::fail("reactivate.persist", &[], error))?;
        self.write_channel(writer, self.channel.global_channel_id, &persist).await?;
        let font_list = activation::build_font_list(demand.share_id, self.channel.user_id).map_err(|error| Self::fail("reactivate.font", &[], error))?;
        self.write_channel(writer, self.channel.global_channel_id, &font_list).await
    }

    fn handle_gfx(&mut self, payload: &[u8]) -> Result<(), RdpProtocolError> {
        let plain = zgfx::decompress(&mut self.gfx_zgfx, payload).map_err(|error| Self::fail("zgfx.decompress", payload, error))?;
        eprintln!("gfx zgfx {} -> {} head=[{}]", payload.len(), plain.len(), hex_head(&plain, 32));
        rdpgfx::process(&mut self.surfaces, &mut self.framebuffer, &plain).map_err(|error| Self::fail("rdpgfx.process", &plain, error))?;
        self.gfx_pdus += 1;
        Ok(())
    }
}

#[test]
fn ntlm_negotiate_token_is_ntlmssp() {
    let mut client = credssp::new_client(WIN10_HOST, "", WIN10_USER, &rdp_test_password(), "termini", vec![0; 32]).expect("build ntlm credssp");
    match credssp::process(&mut client, sspi::credssp::TsRequest::default()).expect("first token") {
        ClientState::ReplyNeeded(request) => {
            let token = request.nego_tokens.expect("nego token");
            assert_eq!(&token[..8], b"NTLMSSP\0");
        }
        ClientState::FinalMessage(_) => panic!("expected first reply"),
    }
}

#[test]
fn kerberos_credssp_client_builds_when_domain_set() {
    credssp::new_client("dc.example.com", "EXAMPLE.COM", WIN10_USER, &rdp_test_password(), "termini", vec![0; 32]).expect("build kerberos credssp");
}

#[test]
fn right_button_down_flags_are_button2() {
    let event = input::button_event(1, true, 10, 20, 0).expect("right down");
    let flags = u16::from_le_bytes([event[1], event[2]]);
    assert_eq!(flags, input::PTR_FLAGS_DOWN | input::PTR_FLAGS_BUTTON2);
    assert_eq!(flags, 0xA000);
}

#[test]
fn negotiate_input_flags_allows_slowpath_when_server_has_no_fastpath() {
    let flags = capabilities::negotiate_input_flags(0);
    assert_eq!(flags, 0x0001);
    assert!(!capabilities::fastpath_input(flags));
    let with_fastpath = capabilities::negotiate_input_flags(capabilities::INPUT_FLAG_FASTPATH_INPUT | capabilities::INPUT_FLAG_FASTPATH_INPUT2);
    assert!(capabilities::fastpath_input(with_fastpath));
}

#[test]
fn slowpath_scancode_and_mouse_match_ms_rdpbcgr() {
    let scancode = input::slowpath_scancode_event(0x1D, false, true).expect("scancode");
    assert_eq!(u32::from_le_bytes(scancode[0..4].try_into().unwrap()), 0);
    assert_eq!(u16::from_le_bytes(scancode[4..6].try_into().unwrap()), 0x0004);
    assert_eq!(u16::from_le_bytes(scancode[6..8].try_into().unwrap()), 0x8100);
    assert_eq!(u16::from_le_bytes(scancode[8..10].try_into().unwrap()), 0x001D);
    assert_eq!(u16::from_le_bytes(scancode[10..12].try_into().unwrap()), 0);
    let mouse = input::slowpath_button_event(1, true, 10, 20, 0).expect("mouse");
    assert_eq!(u16::from_le_bytes(mouse[4..6].try_into().unwrap()), 0x8001);
    assert_eq!(u16::from_le_bytes(mouse[6..8].try_into().unwrap()), input::PTR_FLAGS_DOWN | input::PTR_FLAGS_BUTTON2);
    assert_eq!(u16::from_le_bytes(mouse[8..10].try_into().unwrap()), 10);
    assert_eq!(u16::from_le_bytes(mouse[10..12].try_into().unwrap()), 20);
}

#[test]
fn slowpath_input_pdu_is_one_event_data_pdu() {
    let event = input::slowpath_scancode_event(0x1E, true, false).expect("scancode");
    let pdus = input::build_slowpath_input_pdus(0x0001_0002, 1007, &[event]).expect("pdu");
    assert_eq!(pdus.len(), 1);
    assert_eq!(pdus[0][14], pdu::PDU_TYPE2_INPUT);
    assert_eq!(u16::from_le_bytes(pdus[0][18..20].try_into().unwrap()), 1);
}

#[test]
fn play_sound_parses_duration_then_frequency() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&80u32.to_le_bytes());
    payload.extend_from_slice(&440u32.to_le_bytes());
    let sound = logon::parse_play_sound(&mut ReadCursor::new(&payload)).expect("play sound");
    assert_eq!(sound.duration, 80);
    assert_eq!(sound.frequency, 440);
}

#[test]
fn time_zone_writes_systemtime_and_daylight_bias() {
    assert_eq!(std::mem::size_of::<timezone::TimeZoneInformation>(), 172);
    let mut standard_name = [0u16; 32];
    for (index, unit) in "Pacific Standard Time".encode_utf16().enumerate() {
        standard_name[index] = unit;
    }
    let mut daylight_name = [0u16; 32];
    for (index, unit) in "Pacific Daylight Time".encode_utf16().enumerate() {
        daylight_name[index] = unit;
    }
    let tz = timezone::TimeZoneInformation {
        bias: 480,
        standard_name,
        standard_date: timezone::SystemTime { year: 0, month: 11, day_of_week: 0, day: 1, hour: 2, minute: 0, second: 0, milliseconds: 0 },
        standard_bias: 0,
        daylight_name,
        daylight_date: timezone::SystemTime { year: 0, month: 3, day_of_week: 0, day: 2, hour: 2, minute: 0, second: 0, milliseconds: 0 },
        daylight_bias: -60,
    };
    let mut cursor = WriteCursor::with_capacity(172);
    timezone::write_time_zone(&mut cursor, &tz);
    let bytes = cursor.into_vec();
    assert_eq!(bytes.len(), 172);
    assert_eq!(i32::from_le_bytes(bytes[0..4].try_into().unwrap()), 480);
    assert_eq!(u16::from_le_bytes(bytes[70..72].try_into().unwrap()), 11);
    assert_eq!(u16::from_le_bytes(bytes[74..76].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(bytes[76..78].try_into().unwrap()), 2);
    assert_eq!(u16::from_le_bytes(bytes[154..156].try_into().unwrap()), 3);
    assert_eq!(u16::from_le_bytes(bytes[158..160].try_into().unwrap()), 2);
    assert_eq!(i32::from_le_bytes(bytes[84..88].try_into().unwrap()), 0);
    assert_eq!(i32::from_le_bytes(bytes[168..172].try_into().unwrap()), -60);
    let mut live = WriteCursor::with_capacity(172);
    timezone::write_time_zone_information(&mut live).expect("live timezone");
    assert_eq!(live.into_vec().len(), 172);
    timezone::write_dynamic_time_zone(&mut WriteCursor::with_capacity(8)).expect("live dynamic timezone");
}

#[test]
fn rdp_cert_prompt_skips_when_fingerprint_remembered() {
    let fingerprint = "aa:bb:cc";
    assert_eq!(auth_gate::rdp_cert_prompt(Some("X.509"), Some(fingerprint), fingerprint), None);
    let changed = auth_gate::rdp_cert_prompt(Some("X.509"), Some("11:22"), fingerprint).expect("changed");
    assert_eq!(changed.0, 2);
    assert_eq!(changed.1.as_deref(), Some("11:22"));
    let first = auth_gate::rdp_cert_prompt(None, None, fingerprint).expect("first");
    assert_eq!(first.0, 1);
    assert_eq!(first.1, None);
    let ssh_key = auth_gate::rdp_cert_prompt(Some("ssh-ed25519"), Some(fingerprint), fingerprint).expect("algorithm mismatch");
    assert_eq!(ssh_key.0, 2);
}

#[test]
fn play_sound_beep_pcm_matches_duration() {
    let samples = rdpsnd::beep_pcm(440, 100).expect("beep");
    assert_eq!(samples.len(), 4800);
    assert!(samples.iter().any(|sample| *sample != 0));
    let zero = rdpsnd::play_beep(&mut rdpsnd::RdpsndState::default(), 440, 0);
    zero.expect("zero duration");
    let frequency = rdpsnd::beep_pcm(0, 100).expect_err("zero frequency");
    assert!(frequency.message.contains("频率非法"), "{}", frequency.message);
    let duration = rdpsnd::beep_pcm(440, 10_001).expect_err("long duration");
    assert!(duration.message.contains("时长非法"), "{}", duration.message);
}

fn rdpsnd_format_bytes(tag: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    let avg = u32::from(channels) * u32::from(bits) * rate / 8;
    bytes.extend_from_slice(&avg.to_le_bytes());
    let block = channels * bits / 8;
    bytes.extend_from_slice(&block.to_le_bytes());
    bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}

fn rdpsnd_formats_pdu(formats: &[Vec<u8>]) -> Vec<u8> {
    let extra: usize = formats.iter().map(|format| format.len()).sum();
    let body = 20 + extra;
    let mut message = Vec::new();
    message.push(7);
    message.push(0);
    message.extend_from_slice(&(body as u16).to_le_bytes());
    message.extend_from_slice(&0u32.to_le_bytes());
    message.extend_from_slice(&0u32.to_le_bytes());
    message.extend_from_slice(&0u32.to_le_bytes());
    message.extend_from_slice(&0u16.to_le_bytes());
    message.extend_from_slice(&(formats.len() as u16).to_le_bytes());
    message.push(0);
    message.extend_from_slice(&8u16.to_le_bytes());
    message.push(0);
    for format in formats {
        message.extend_from_slice(format);
    }
    message
}

fn parse_rdpsnd_server_formats(message: &[u8]) -> (u16, Vec<u16>) {
    let mut cursor = ReadCursor::new(message);
    cursor.read_u8().expect("type");
    cursor.read_u8().expect("pad");
    let body = cursor.read_u16_le().expect("body") as usize;
    let mut body = ReadCursor::new(cursor.read_slice(body).expect("body bytes"));
    body.skip(14).expect("flags");
    let count = body.read_u16_le().expect("count");
    body.read_u8().expect("block");
    let version = body.read_u16_le().expect("version");
    body.skip(1).expect("pad2");
    let mut tags = Vec::new();
    for _ in 0..count {
        let tag = body.read_u16_le().expect("tag");
        body.skip(12).expect("format");
        let extra = body.read_u16_le().expect("extra") as usize;
        body.skip(extra).expect("extra bytes");
        tags.push(tag);
    }
    (version, tags)
}

fn rdpsnd_wave2_pdu(timestamp: u16, format_no: u16, block_no: u8, data: &[u8]) -> Vec<u8> {
    let body = 12 + data.len();
    let mut message = Vec::new();
    message.push(13);
    message.push(0);
    message.extend_from_slice(&(body as u16).to_le_bytes());
    message.extend_from_slice(&timestamp.to_le_bytes());
    message.extend_from_slice(&format_no.to_le_bytes());
    message.push(block_no);
    message.extend_from_slice(&[0, 0, 0]);
    message.extend_from_slice(&0u32.to_le_bytes());
    message.extend_from_slice(data);
    message
}

#[test]
fn rdpsnd_advertise_pcm_only() {
    let mut state = RdpsndState::default();
    let pcm = rdpsnd_format_bytes(0x0001, 2, 44100, 16);
    let ima = rdpsnd_format_bytes(0x0011, 2, 44100, 4);
    let outcome = rdpsnd::process(&mut state, &rdpsnd_formats_pdu(&[pcm, ima])).expect("formats");
    assert_eq!(outcome.responses[0][0], 7);
    let flags = u32::from_le_bytes(outcome.responses[0][4..8].try_into().unwrap());
    assert_eq!(flags, 3);
    let volume = u32::from_le_bytes(outcome.responses[0][8..12].try_into().unwrap());
    assert_eq!(volume, 0xFFFF_FFFF);
    let count = u16::from_le_bytes([outcome.responses[0][18], outcome.responses[0][19]]);
    assert_eq!(count, 1);
    let version = u16::from_le_bytes([outcome.responses[0][21], outcome.responses[0][22]]);
    assert_eq!(version, 8);
    let tag = u16::from_le_bytes([outcome.responses[0][24], outcome.responses[0][25]]);
    assert_eq!(tag, 0x0001);
}

#[test]
fn rdpsnd_wave2_sends_two_confirms() {
    let mut state = RdpsndState::default();
    let pcm = rdpsnd_format_bytes(0x0001, 1, 8000, 16);
    rdpsnd::process(&mut state, &rdpsnd_formats_pdu(&[pcm])).expect("formats");
    let data = vec![0u8; 160];
    let outcome = rdpsnd::process(&mut state, &rdpsnd_wave2_pdu(1000, 0, 3, &data)).expect("wave2");
    assert_eq!(outcome.responses.len(), 2);
    assert_eq!(outcome.responses[0][0], 5);
    assert_eq!(outcome.responses[1][0], 5);
    assert_eq!(outcome.responses[0][6], 3);
    assert_eq!(outcome.responses[1][6], 3);
}

fn rdpsnd_wave_info_pdu(timestamp: u16, format_no: u16, block_no: u8, data: &[u8]) -> Vec<u8> {
    let body = 8 + data.len();
    let mut message = Vec::new();
    message.push(2);
    message.push(0);
    message.extend_from_slice(&(body as u16).to_le_bytes());
    message.extend_from_slice(&timestamp.to_le_bytes());
    message.extend_from_slice(&format_no.to_le_bytes());
    message.push(block_no);
    message.extend_from_slice(&[0, 0, 0]);
    message.extend_from_slice(&data[..4]);
    message
}

#[test]
fn rdpsnd_wave_info_then_body_sends_two_confirms() {
    let mut state = RdpsndState::default();
    let pcm = rdpsnd_format_bytes(0x0001, 1, 8000, 16);
    rdpsnd::process(&mut state, &rdpsnd_formats_pdu(&[pcm])).expect("formats");
    let data = vec![0u8; 160];
    let info = rdpsnd::process(&mut state, &rdpsnd_wave_info_pdu(1000, 0, 3, &data)).expect("wave info");
    assert!(info.responses.is_empty());
    let mut body = data.clone();
    body[..4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    let outcome = rdpsnd::process(&mut state, &body).expect("wave body");
    assert_eq!(outcome.responses.len(), 2);
    assert_eq!(outcome.responses[0][0], 5);
    assert_eq!(outcome.responses[1][0], 5);
    assert_eq!(outcome.responses[0][6], 3);
    assert_eq!(outcome.responses[1][6], 3);
}

#[test]
fn ncrush_decompresses_freerdp_bells_vector() {
    let compressed: &[u8] =
        b"\xfb\x1d\x7e\xe4\xda\xc7\x1d\x70\xf8\xa1\x6b\x1f\x7d\xc0\xbe\x6b\xef\xb5\xef\x21\x87\xd0\xc5\xe1\x85\x71\xd4\x10\x16\xe7\xda\xfb\x1d\x7e\xe4\xda\x47\x1f\xb0\xef\xbe\xbd\xff\x2f";
    let expected = b"for.whom.the.bell.tolls,.the.bell.tolls.for.thee!";
    let mut context = ncrush::NcrushContext::default();
    let flags = bulk::PACKET_COMPRESSED | bulk::PACKET_COMPR_TYPE_RDP6;
    let plain = ncrush::decompress(&mut context, compressed, flags).expect("ncrush decompress");
    assert_eq!(plain, expected);
}

#[test]
fn persistent_key_list_pdu_is_empty_first_connect() {
    let pdu = activation::build_persistent_key_list(1, 1002, &[Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()]).expect("persist pdu");
    let mut cursor = ReadCursor::new(&pdu);
    let header = pdu::read_share_control_header(&mut cursor).expect("control");
    assert_eq!(header.pdu_type, pdu::PDU_TYPE_DATA);
    let data = pdu::read_share_data_header(&mut cursor).expect("data");
    assert_eq!(data.pdu_type2, pdu::PDU_TYPE2_BITMAP_CACHE_PERSISTENT_LIST);
}

#[test]
fn pointer_8bpp_uses_session_palette() {
    let mut palette = Palette::default();
    palette.load(&[[0x11, 0x22, 0x33], [0x44, 0x55, 0x66]]).expect("palette");
    let mut payload = Vec::new();
    payload.extend_from_slice(&8u16.to_le_bytes());
    payload.extend_from_slice(&0u16.to_le_bytes());
    payload.extend_from_slice(&0u16.to_le_bytes());
    payload.extend_from_slice(&0u16.to_le_bytes());
    payload.extend_from_slice(&2u16.to_le_bytes());
    payload.extend_from_slice(&2u16.to_le_bytes());
    payload.extend_from_slice(&4u16.to_le_bytes());
    payload.extend_from_slice(&4u16.to_le_bytes());
    payload.extend_from_slice(&[1, 0, 0, 0]);
    payload.extend_from_slice(&[0, 0, 0, 0]);
    let mut cursor = ReadCursor::new(&payload);
    let (_, image) = pointer::parse_new_pointer(&mut cursor, &palette).expect("8bpp pointer");
    assert_eq!(image.width, 2);
    assert_eq!(image.height, 2);
    assert_eq!(&image.rgba[0..4], &[0x11, 0x22, 0x33, 0xFF]);
    assert_eq!(&image.rgba[8..12], &[0x44, 0x55, 0x66, 0xFF]);
}

#[test]
fn glyph_index_paints_cached_1bpp() {
    let mut glyphs = GlyphCache::default();
    glyphs.store(0, CachedGlyph { index: 1, x: 0, y: 0, cx: 1, cy: 1, aj: vec![0x80, 0, 0, 0] }).expect("store glyph");
    let mut framebuffer = Framebuffer::new(8, 8);
    let order = DrawOrder {
        clip: None,
        kind: DrawKind::Glyph(GlyphOrder {
            cache_id: 0,
            fl_accel: 0,
            ul_char_inc: 0,
            f_op_redundant: true,
            back_color: 0x00FF00,
            fore_color: 0x0000FF,
            bk_left: 0,
            bk_top: 0,
            bk_right: 7,
            bk_bottom: 7,
            op_left: 0,
            op_top: 0,
            op_right: 7,
            op_bottom: 7,
            x: 2,
            y: 3,
            data: vec![1, 0],
            embedded: None,
        }),
    };
    painter::apply(&mut framebuffer, &order, 24, &Palette::default(), &BitmapCache::default(), &BrushCache::default(), &mut glyphs).expect("draw glyph");
    let frame = framebuffer.take_frame(90).expect("encode").expect("dirty");
    assert!(frame.len() > 9);
}

#[test]
fn jpeg_surface_decode_rejects_truncated() {
    let error = jpeg::decode(&[0xFF, 0xD8], 8, 8).expect_err("truncated jpeg");
    assert!(error.message.contains("JPEG"));
}

#[test]
fn encode_rgba_emits_jpeg() {
    let mut rgba = vec![0u8; 16 * 16 * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[0] = 255;
        pixel[3] = 255;
    }
    let bytes = jpeg::encode(&rgba, 16, 16, 90).expect("jpeg");
    assert!(bytes.len() > 2);
    assert_eq!(&bytes[0..2], &[0xFF, 0xD8]);
}

#[test]
fn encode_rgba_emits_png() {
    let mut rgba = vec![0u8; 16 * 16 * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[2] = 255;
        pixel[3] = 255;
    }
    let bytes = png::encode(&rgba, 16, 16).expect("png");
    assert!(bytes.len() > 8);
    assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
}

#[test]
fn encode_rgba_emits_webp() {
    let mut rgba = vec![0u8; 16 * 16 * 4];
    for (index, pixel) in rgba.chunks_exact_mut(4).enumerate() {
        pixel[0] = index as u8;
        pixel[1] = (index * 3) as u8;
        pixel[2] = (index * 7) as u8;
        pixel[3] = 255;
    }
    let bytes = webp::encode(rgba, 16, 16, 90).expect("webp");
    assert!(bytes.len() > 12);
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WEBP");
}

#[test]
fn suggest_quality_follows_guacamole_clamp() {
    assert_eq!(framebuffer::suggest_quality(0), 90);
    assert_eq!(framebuffer::suggest_quality(20), 90);
    assert_eq!(framebuffer::suggest_quality(50), 60);
    assert_eq!(framebuffer::suggest_quality(80), 30);
    assert_eq!(framebuffer::suggest_quality(1000), 30);
}

#[test]
fn large_dirty_encodes_jpeg() {
    let mut framebuffer = Framebuffer::new(256, 256);
    let mut source = vec![0u8; 256 * 256 * 4];
    for (index, pixel) in source.chunks_exact_mut(4).enumerate() {
        pixel[0] = (index % 251) as u8;
        pixel[1] = ((index * 13) % 251) as u8;
        pixel[2] = ((index * 29) % 251) as u8;
        pixel[3] = 255;
    }
    framebuffer.blit_surface(Rect::from_size(0, 0, 256, 256).expect("rect"), &source, 256 * 4, 0, 0).expect("blit");
    let extents = framebuffer.take_dirty_extents().expect("dirty");
    assert_eq!((extents.left, extents.top, extents.right, extents.bottom), (0, 0, 256, 256));
    let patches = vec![(extents, framebuffer.copy_rgba_rect(&extents))];
    let body = Framebuffer::encode_rgba_patches(&patches, 90).expect("encode");
    assert_eq!(body[0], FRAME_KIND_JPEG);
}

#[test]
fn gfx_caps_advertise_v107_avc444() {
    let advertise = rdpgfx::build_caps_advertise();
    let mut cursor = ReadCursor::new(&advertise);
    cursor.skip(8).expect("header");
    let count = cursor.read_u16_le().expect("count");
    assert_eq!(count, 10);
    let mut versions = Vec::new();
    for index in 0..count {
        let version = cursor.read_u32_le().expect("version");
        let length = cursor.read_u32_le().expect("length");
        let flags = cursor.read_u32_le().expect("flags");
        cursor.skip(length as usize - 4).expect("pad");
        versions.push(version);
        if version >= 0x000A_0002 {
            assert_eq!(flags & 0x0000_0020, 0, "v10 caps 不应禁用 AVC: {:#x}", version);
        }
        if index == 0 {
            assert_eq!(version, 0x000A_0701);
            assert_ne!(flags & 0x0000_0080, 0);
        }
        if version == 0x0008_0105 {
            assert_ne!(flags & 0x0000_0010, 0);
        }
    }
    assert!(versions.contains(&0x000A_0002));
    assert!(versions.contains(&0x0008_0105));
    assert!(versions.contains(&0x0008_0004));
}

fn avc420_wrap_meta(rect: Rect, bitstream: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&rect.left.to_le_bytes());
    data.extend_from_slice(&rect.top.to_le_bytes());
    data.extend_from_slice(&rect.right.to_le_bytes());
    data.extend_from_slice(&rect.bottom.to_le_bytes());
    data.extend_from_slice(&[22, 100]);
    data.extend_from_slice(bitstream);
    data
}

#[test]
fn avc420_surface_command_writes_rgb() {
    let mut encoder = OutputEncoder::new(64, 64).expect("encoder");
    let mut rgba = vec![0u8; 64 * 64 * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[0] = 200;
        pixel[1] = 16;
        pixel[2] = 16;
        pixel[3] = 255;
    }
    encoder.submit(&rgba, 0).expect("submit");
    let bitstream = encoder.take_output(Duration::from_secs(1)).expect("take").expect("bytes");
    let rect = Rect::from_size(0, 0, 64, 64).expect("rect");
    let data = avc420_wrap_meta(rect, &bitstream);
    let mut surfaces = GfxSurfaces::default();
    surfaces.create(1, 64, 64, gfx::PIXEL_FORMAT_XRGB).expect("create");
    surfaces.map_to_output(1, 0, 0).expect("map");
    surfaces.surface_command(&gfx::SurfaceCommand { surface_id: 1, codec_id: gfx::CODEC_AVC420, alpha: false, rect, data: &data }).expect("avc420");
    assert!(surfaces.take_avc_patches().is_empty());
    let mut framebuffer = Framebuffer::new(64, 64);
    surfaces.update(&mut framebuffer).expect("update");
    let pixel = framebuffer.copy_rgba_rect(&rect);
    assert!(pixel[0] > 80, "expected red channel, got {:?}", &pixel[..4]);
    assert!(pixel[1] < 120, "expected low green, got {:?}", &pixel[..4]);
    assert!(pixel[2] < 120, "expected low blue, got {:?}", &pixel[..4]);
}

#[test]
fn solid_rect_encodes_png() {
    let mut framebuffer = Framebuffer::new(32, 32);
    let mut source = vec![0u8; 32 * 32 * 4];
    for pixel in source.chunks_exact_mut(4) {
        pixel[0] = 10;
        pixel[1] = 20;
        pixel[2] = 30;
        pixel[3] = 255;
    }
    framebuffer.blit_surface(Rect::from_size(0, 0, 32, 32).expect("rect"), &source, 32 * 4, 0, 0).expect("blit");
    let frame = framebuffer.take_frame(90).expect("encode").expect("dirty");
    assert_eq!(frame[0], FRAME_KIND_PNG);
}

#[test]
fn noisy_rect_encodes_jpeg() {
    let mut framebuffer = Framebuffer::new(64, 64);
    let mut source = vec![0u8; 64 * 64 * 4];
    for (index, pixel) in source.chunks_exact_mut(4).enumerate() {
        pixel[0] = (index % 251) as u8;
        pixel[1] = ((index * 13) % 251) as u8;
        pixel[2] = ((index * 29) % 251) as u8;
        pixel[3] = 255;
    }
    framebuffer.blit_surface(Rect::from_size(0, 0, 64, 64).expect("rect"), &source, 64 * 4, 0, 0).expect("blit");
    let frame = framebuffer.take_frame(90).expect("encode").expect("dirty");
    assert_eq!(frame[0], FRAME_KIND_JPEG);
}

fn clip_msg(msg_type: u16, flags: u16, body: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(8 + body.len());
    message.extend_from_slice(&msg_type.to_le_bytes());
    message.extend_from_slice(&flags.to_le_bytes());
    message.extend_from_slice(&(body.len() as u32).to_le_bytes());
    message.extend_from_slice(body);
    message
}

fn clip_caps_long_file() -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&12u16.to_le_bytes());
    body.extend_from_slice(&2u32.to_le_bytes());
    body.extend_from_slice(&(0x02 | 0x04 | 0x08 | 0x20u32).to_le_bytes());
    clip_msg(0x0007, 0, &body)
}

fn clip_ready(state: &mut ClipboardState) {
    cliprdr::process(state, &clip_caps_long_file()).expect("caps");
    cliprdr::process(state, &clip_msg(0x0001, 0, &[])).expect("ready");
}

fn utf16_z(name: &str) -> Vec<u8> {
    let mut data = Vec::new();
    for unit in name.encode_utf16() {
        data.extend_from_slice(&unit.to_le_bytes());
    }
    data.extend_from_slice(&0u16.to_le_bytes());
    data
}

fn file_descriptor(name: &str, size: u64, directory: bool) -> Vec<u8> {
    let mut body = vec![0u8; 592];
    body[0..4].copy_from_slice(&0x0064u32.to_le_bytes());
    body[36..40].copy_from_slice(&(if directory { 0x10u32 } else { 0x20 }).to_le_bytes());
    body[64..68].copy_from_slice(&((size >> 32) as u32).to_le_bytes());
    body[68..72].copy_from_slice(&(size as u32).to_le_bytes());
    let mut name_bytes = Vec::new();
    for unit in name.encode_utf16() {
        name_bytes.extend_from_slice(&unit.to_le_bytes());
    }
    body[72..72 + name_bytes.len()].copy_from_slice(&name_bytes);
    body
}

#[test]
fn cliprdr_format_list_requests_file_group_by_name() {
    let mut state = ClipboardState::new(String::new());
    clip_ready(&mut state);
    let mut body = Vec::new();
    body.extend_from_slice(&0x1234u32.to_le_bytes());
    body.extend_from_slice(&utf16_z("FileGroupDescriptorW"));
    let outcome = cliprdr::process(&mut state, &clip_msg(0x0002, 0, &body)).expect("format list");
    assert_eq!(outcome.responses.len(), 2);
    assert_eq!(&outcome.responses[1][0..2], &0x0004u16.to_le_bytes());
    assert_eq!(&outcome.responses[1][8..12], &0x1234u32.to_le_bytes());
}

#[test]
fn cliprdr_html_format_roundtrip() {
    let mut state = ClipboardState::new(String::new());
    clip_ready(&mut state);
    cliprdr::set_local(&mut state, String::new(), Vec::new(), "<b>x</b>".to_string(), None).expect("set html");
    let mut request = Vec::new();
    request.extend_from_slice(&0x0000_C0E0u32.to_le_bytes());
    let outcome = cliprdr::process(&mut state, &clip_msg(0x0004, 0, &request)).expect("html request");
    assert_eq!(&outcome.responses[0][0..2], &0x0005u16.to_le_bytes());
    let data_len = u32::from_le_bytes(outcome.responses[0][4..8].try_into().unwrap()) as usize;
    let html = &outcome.responses[0][8..8 + data_len];
    assert!(html.starts_with(b"Version:0.9"));
    let mut list = Vec::new();
    list.extend_from_slice(&0xABCDu32.to_le_bytes());
    list.extend_from_slice(&utf16_z("HTML Format"));
    cliprdr::process(&mut state, &clip_msg(0x0002, 0, &list)).expect("html list");
    let incoming = cliprdr::process(&mut state, &clip_msg(0x0005, 0x0001, html)).expect("html data");
    assert_eq!(incoming.html.as_deref(), Some("<html>\r\n<body>\r\n<!--StartFragment--><b>x</b><!--EndFragment-->\r\n</body>\r\n</html>"));
}

#[test]
fn cliprdr_local_image_serves_dib() {
    let mut state = ClipboardState::new(String::new());
    clip_ready(&mut state);
    let image = LocalImage { width: 1, height: 1, rgba: vec![0x11, 0x22, 0x33, 0xFF], png: vec![0x89, 0x50] };
    cliprdr::set_local(&mut state, String::new(), Vec::new(), String::new(), Some(image)).expect("set image");
    let mut request = Vec::new();
    request.extend_from_slice(&8u32.to_le_bytes());
    let outcome = cliprdr::process(&mut state, &clip_msg(0x0004, 0, &request)).expect("dib request");
    let data_len = u32::from_le_bytes(outcome.responses[0][4..8].try_into().unwrap()) as usize;
    let dib = &outcome.responses[0][8..8 + data_len];
    assert_eq!(&dib[0..4], &40u32.to_le_bytes());
    assert_eq!(&dib[14..16], &32u16.to_le_bytes());
    assert_eq!(&dib[40..44], &[0x33, 0x22, 0x11, 0xFF]);
}

#[test]
fn cliprdr_empty_remote_file_completes_without_contents() {
    let mut state = ClipboardState::new(String::new());
    clip_ready(&mut state);
    let mut list = Vec::new();
    list.extend_from_slice(&0x1234u32.to_le_bytes());
    list.extend_from_slice(&utf16_z("FileGroupDescriptorW"));
    cliprdr::process(&mut state, &clip_msg(0x0002, 0, &list)).expect("file list");
    let mut payload = Vec::new();
    payload.extend_from_slice(&1u32.to_le_bytes());
    payload.extend_from_slice(&file_descriptor("a.txt", 0, false));
    let outcome = cliprdr::process(&mut state, &clip_msg(0x0005, 0x0001, &payload)).expect("file data");
    let files = outcome.files.expect("files");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "a.txt");
    assert!(files[0].data.is_empty());
}

#[test]
fn cliprdr_unexpected_file_contents_errors() {
    let mut state = ClipboardState::new(String::new());
    clip_ready(&mut state);
    let error = match cliprdr::process(&mut state, &clip_msg(0x0009, 0x0001, &[1, 0, 0, 0])) {
        Err(error) => error,
        Ok(_) => panic!("unexpected file contents succeeded"),
    };
    assert!(error.message.contains("未请求的文件内容"));
}

fn gcc_block(data: &[u8], kind: u16) -> &[u8] {
    let mut offset = 0;
    while offset + 4 <= data.len() {
        let block_kind = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
        let length = u16::from_le_bytes(data[offset + 2..offset + 4].try_into().unwrap()) as usize;
        if length < 4 || offset + length > data.len() {
            panic!("gcc 数据块长度异常: kind={:#06x} length={} offset={}", block_kind, length, offset);
        }
        if block_kind == kind {
            return &data[offset + 4..offset + length];
        }
        offset += length;
    }
    panic!("gcc 缺少数据块 {:#06x}", kind);
}

fn gcc_settings(monitors: Vec<gcc::Monitor>) -> gcc::ClientCoreSettings {
    gcc::ClientCoreSettings {
        desktop_width: 1280,
        desktop_height: 750,
        color_depth: 32,
        keyboard_layout: 0x0409,
        client_name: "termini".to_string(),
        selected_protocol: 1,
        console_session: false,
        extended_client_data: true,
        encryption_methods: 0,
        monitors,
    }
}

fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn i32_at(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

#[test]
fn monitor_rejects_zero_dpi() {
    let error = match gcc::Monitor::from_size(1280, 750, 0, true) {
        Err(error) => error,
        Ok(_) => panic!("dpi=0 succeeded"),
    };
    assert!(error.message.contains("DPI 为 0"));
}

#[test]
fn gcc_client_data_writes_monitor_and_scale() {
    let monitor = gcc::Monitor::from_size(1280, 750, 96, true).expect("monitor");
    assert_eq!(monitor.physical_width, 339);
    assert_eq!(monitor.physical_height, 198);
    assert_eq!(monitor.desktop_scale_factor, 100);
    assert_eq!(monitor.device_scale_factor, 100);
    let data = gcc::build_client_data(&gcc_settings(vec![monitor]), &[]).expect("gcc");
    let core = gcc_block(&data, 0xC001);
    let flags = u16::from_le_bytes(core[140..142].try_into().unwrap());
    assert_ne!(flags & 0x0040, 0);
    let tail = &core[core.len() - 18..];
    assert_eq!(u32_at(tail, 0), 339);
    assert_eq!(u32_at(tail, 4), 198);
    assert_eq!(u16::from_le_bytes(tail[8..10].try_into().unwrap()), 0);
    assert_eq!(u32_at(tail, 10), 100);
    assert_eq!(u32_at(tail, 14), 100);
    let cs_monitor = gcc_block(&data, 0xC005);
    assert_eq!(u32_at(cs_monitor, 0), 0);
    assert_eq!(u32_at(cs_monitor, 4), 1);
    assert_eq!(i32_at(cs_monitor, 8), 0);
    assert_eq!(i32_at(cs_monitor, 12), 0);
    assert_eq!(i32_at(cs_monitor, 16), 1279);
    assert_eq!(i32_at(cs_monitor, 20), 749);
    assert_eq!(u32_at(cs_monitor, 24), 1);
    let cs_monitor_ex = gcc_block(&data, 0xC008);
    assert_eq!(u32_at(cs_monitor_ex, 0), 0);
    assert_eq!(u32_at(cs_monitor_ex, 4), 20);
    assert_eq!(u32_at(cs_monitor_ex, 8), 1);
    assert_eq!(u32_at(cs_monitor_ex, 12), 339);
    assert_eq!(u32_at(cs_monitor_ex, 16), 198);
    assert_eq!(u32_at(cs_monitor_ex, 20), 0);
    assert_eq!(u32_at(cs_monitor_ex, 24), 100);
    assert_eq!(u32_at(cs_monitor_ex, 28), 100);
}

#[test]
fn gcc_writes_two_monitors_rebased_to_primary() {
    let primary = gcc::Monitor::from_size(1280, 750, 144, true).expect("primary");
    assert_eq!(primary.desktop_scale_factor, 150);
    assert_eq!(primary.device_scale_factor, 100);
    let mut secondary = gcc::Monitor::from_size(800, 600, 192, false).expect("secondary");
    assert_eq!(secondary.desktop_scale_factor, 200);
    assert_eq!(secondary.device_scale_factor, 100);
    secondary.left = 1280;
    let data = gcc::build_client_data(&gcc_settings(vec![primary, secondary]), &[]).expect("gcc");
    let cs_monitor = gcc_block(&data, 0xC005);
    assert_eq!(u32_at(cs_monitor, 4), 2);
    assert_eq!(i32_at(cs_monitor, 8), 0);
    assert_eq!(i32_at(cs_monitor, 28), 1280);
    assert_eq!(i32_at(cs_monitor, 32), 0);
    assert_eq!(i32_at(cs_monitor, 36), 2079);
    assert_eq!(i32_at(cs_monitor, 40), 599);
    assert_eq!(u32_at(cs_monitor, 44), 0);
    let cs_monitor_ex = gcc_block(&data, 0xC008);
    assert_eq!(u32_at(cs_monitor_ex, 8), 2);
    assert_eq!(u32_at(cs_monitor_ex, 24), 150);
    assert_eq!(u32_at(cs_monitor_ex, 28), 100);
    assert_eq!(u32_at(cs_monitor_ex, 44), 200);
    assert_eq!(u32_at(cs_monitor_ex, 48), 100);
}

#[test]
fn disp_monitor_layout_includes_physical_and_scale() {
    let monitor = gcc::Monitor::from_size(1280, 750, 144, true).expect("monitor");
    let pdu = disp::build_monitor_layout(&[monitor]).expect("disp");
    assert_eq!(u32_at(&pdu, 0), 2);
    assert_eq!(u32_at(&pdu, 4), 56);
    assert_eq!(u32_at(&pdu, 8), 40);
    assert_eq!(u32_at(&pdu, 12), 1);
    assert_eq!(u32_at(&pdu, 16), 1);
    assert_eq!(i32_at(&pdu, 20), 0);
    assert_eq!(i32_at(&pdu, 24), 0);
    assert_eq!(u32_at(&pdu, 28), 1280);
    assert_eq!(u32_at(&pdu, 32), 750);
    assert_eq!(u32_at(&pdu, 36), monitor.physical_width);
    assert_eq!(u32_at(&pdu, 40), monitor.physical_height);
    assert_eq!(u32_at(&pdu, 44), 0);
    assert_eq!(u32_at(&pdu, 48), 150);
    assert_eq!(u32_at(&pdu, 52), 100);
}

fn drive_req(major: u32, file_id: u32, payload: Vec<u8>) -> DeviceIoRequest {
    DeviceIoRequest { file_id, device_id: 1, completion_id: 1, major, minor: 0, payload }
}

fn drive_create_payload(name: &str) -> Vec<u8> {
    let path = utf16_z(name);
    let mut body = Vec::new();
    body.extend_from_slice(&0x0000_0002u32.to_le_bytes());
    body.extend_from_slice(&0u64.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&3u32.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&(path.len() as u32).to_le_bytes());
    body.extend_from_slice(&path);
    body
}

fn drive_open_file(state: &mut DriveState, name: &str) -> u32 {
    let (status, body) = drive::handle(state, &drive_req(0, 0, drive_create_payload(name))).expect("create");
    assert_eq!(status, 0);
    u32::from_le_bytes(body[0..4].try_into().unwrap())
}

#[test]
fn drive_flush_cleanup_shutdown_complete() {
    let root = std::env::temp_dir().join("termini-rdp-drive-irp");
    std::fs::create_dir_all(&root).expect("temp drive");
    let mut state = DriveState::new(root.to_str().expect("utf8 path"));
    let file_id = drive_open_file(&mut state, "a.txt");
    let (status, body) = drive::handle(&mut state, &drive_req(0x0000_0009, file_id, vec![0u8; 32])).expect("flush");
    assert_eq!(status, 0);
    assert!(body.is_empty());
    let (status, body) = drive::handle(&mut state, &drive_req(0x0000_0012, file_id, vec![0u8; 32])).expect("cleanup");
    assert_eq!(status, 0);
    assert_eq!(body.len(), 5);
    let (status, body) = drive::handle(&mut state, &drive_req(0x0000_0002, file_id, vec![0u8; 32])).expect("close after cleanup");
    assert_eq!(status, 0);
    assert_eq!(body.len(), 5);
    let (status, body) = drive::handle(&mut state, &drive_req(0x0000_0010, 0, vec![0u8; 32])).expect("shutdown");
    assert_eq!(status, 0);
    assert_eq!(body.len(), 5);
}

#[test]
fn drive_device_control_succeeds_empty() {
    let mut state = DriveState::new(std::env::temp_dir().to_str().expect("utf8 path"));
    let (status, body) = drive::handle(&mut state, &drive_req(0x0000_000E, 0, Vec::new())).expect("device control");
    assert_eq!(status, 0);
    assert_eq!(body.as_slice(), 0u32.to_le_bytes());
}

#[test]
fn drive_query_and_set_security_complete() {
    let root = std::env::temp_dir().join("termini-rdp-drive-security");
    std::fs::create_dir_all(&root).expect("temp drive");
    let mut state = DriveState::new(root.to_str().expect("utf8 path"));
    let file_id = drive_open_file(&mut state, "s.txt");
    let (status, body) = drive::handle(&mut state, &drive_req(0x0000_0014, file_id, 0x0000_0004u32.to_le_bytes().to_vec())).expect("query security");
    assert_eq!(status, 0xC000_00BB);
    assert_eq!(body.as_slice(), 0u32.to_le_bytes());
    let mut set = Vec::new();
    set.extend_from_slice(&0x0000_0004u32.to_le_bytes());
    set.extend_from_slice(&4u32.to_le_bytes());
    set.extend_from_slice(&[0u8; 24]);
    set.extend_from_slice(&[1, 2, 3, 4]);
    let (status, body) = drive::handle(&mut state, &drive_req(0x0000_0015, file_id, set)).expect("set security");
    assert_eq!(status, 0xC000_00BB);
    assert_eq!(body.as_slice(), 4u32.to_le_bytes());
}

async fn connect_probe(label: &str, target: RdpSettings) -> Result<(u16, u16), String> {
    let data_dir = std::env::temp_dir().join(format!("termini-rdp-opt-{}-{}", label, target.host));
    std::fs::create_dir_all(&data_dir).expect("create temp data dir");
    std::env::set_var("TERMINI_DATA_DIR", &data_dir);
    match RdpSessionFactory.connect(&target, None, &silent_handler(), None).await {
        Ok(connected) => Ok((connected.desktop.width, connected.desktop.height)),
        Err(error) => Err(error.message),
    }
}

async fn nego_then_tls(label: &str, host: &str, user: &str, protocols: u32) {
    eprintln!("{} tcp {}:{}", label, host, PORT);
    let mut transport = match RdpTransport::connect(host, PORT, 15, 0).await {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("{} tcp err={}", label, error.message);
            return;
        }
    };
    let request = match nego::build_connection_request(None, Some(user), protocols) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("{} cr err={}", label, error.message);
            return;
        }
    };
    eprintln!("{} cr hex={}", label, hex_head(&request, request.len()));
    if let Err(error) = transport.send(&request).await {
        eprintln!("{} send err={}", label, error.message);
        return;
    }
    let confirm = match transport.read_x224().await {
        Ok(confirm) => confirm,
        Err(error) => {
            eprintln!("{} cc err={}", label, error.message);
            return;
        }
    };
    eprintln!("{} cc hex={}", label, hex_head(&confirm, confirm.len()));
    match nego::parse_connection_confirm(&confirm) {
        Ok(response) => {
            eprintln!("{} selected=0x{:08x} {}", label, response.selected_protocol, nego::describe_protocol(response.selected_protocol));
            if response.selected_protocol == nego::PROTOCOL_RDP {
                eprintln!("{} skip tls", label);
                return;
            }
        }
        Err(error) => {
            eprintln!("{} nego err={}", label, error.message);
            return;
        }
    }
    match transport.upgrade_tls(host, true, None).await {
        Ok(TlsUpgradeResult::Established { fingerprint, .. }) => eprintln!("{} tls ok fp={}", label, fingerprint),
        Ok(TlsUpgradeResult::Untrusted { fingerprint }) => eprintln!("{} tls untrusted fp={}", label, fingerprint),
        Err(error) => eprintln!("{} tls err={}", label, error.message),
    }
}

#[tokio::test]
async fn connect_windows8() {
    let full = nego::PROTOCOL_HYBRID_EX | nego::PROTOCOL_HYBRID | nego::PROTOCOL_SSL;
    nego_then_tls("win8-full", WIN8_HOST, WIN10_USER, full).await;
    nego_then_tls("win8-ssl", WIN8_HOST, WIN10_USER, nego::PROTOCOL_SSL).await;
    nego_then_tls("win8-rdp", WIN8_HOST, WIN10_USER, nego::PROTOCOL_RDP).await;
    nego_then_tls("win11-full", WIN11_HOST, WIN11_USER, full).await;
    let size = connect_probe("win8", settings(WIN8_HOST, WIN10_USER)).await;
    eprintln!("win8 connect={:?}", size);
}

#[test]
fn rdp_security_protocols_differ() {
    assert_eq!(RdpSettings::resolve_protocols(SECURITY_AUTO, None), nego::PROTOCOL_HYBRID_EX | nego::PROTOCOL_HYBRID | nego::PROTOCOL_SSL);
    assert_eq!(RdpSettings::resolve_protocols(SECURITY_NLA, None), nego::PROTOCOL_HYBRID_EX | nego::PROTOCOL_HYBRID);
    assert_eq!(RdpSettings::resolve_protocols(SECURITY_TLS, None), nego::PROTOCOL_SSL);
    assert_eq!(
        RdpSettings::resolve_protocols(SECURITY_AUTO, Some("11111111-1111-1111-1111-111111111111")),
        nego::PROTOCOL_HYBRID_EX | nego::PROTOCOL_HYBRID | nego::PROTOCOL_SSL | nego::PROTOCOL_RDSAAD
    );
    assert_eq!(RdpSettings::resolve_protocols(SECURITY_NLA, Some("11111111-1111-1111-1111-111111111111")), nego::PROTOCOL_HYBRID_EX | nego::PROTOCOL_HYBRID | nego::PROTOCOL_RDSAAD);
    assert_eq!(RdpSettings::resolve_protocols(SECURITY_TLS, Some("11111111-1111-1111-1111-111111111111")), nego::PROTOCOL_SSL);
}

#[tokio::test]
async fn connect_windows11_option_matrix() {
    let mut quality = settings(WIN11_HOST, WIN11_USER);
    quality.performance_flags = 0;
    connect_probe("quality", quality).await.expect("quality");
    let mut smooth = settings(WIN11_HOST, WIN11_USER);
    smooth.performance_flags = 0x6f;
    connect_probe("smooth", smooth).await.expect("smooth");
    let mut balanced = settings(WIN11_HOST, WIN11_USER);
    balanced.performance_flags = info::PERF_DISABLE_FULLWINDOWDRAG | info::PERF_DISABLE_MENUANIMATIONS;
    connect_probe("balanced", balanced).await.expect("balanced");
    let mut color16 = settings(WIN11_HOST, WIN11_USER);
    color16.color_depth = 16;
    connect_probe("color16", color16).await.expect("color16");
    let mut color24 = settings(WIN11_HOST, WIN11_USER);
    color24.color_depth = 24;
    connect_probe("color24", color24).await.expect("color24");
    let mut kb_cn = settings(WIN11_HOST, WIN11_USER);
    kb_cn.keyboard_layout = 0x0804;
    connect_probe("kb_cn", kb_cn).await.expect("kb_cn");
    let mut desk = settings(WIN11_HOST, WIN11_USER);
    desk.desktop_width = 1024;
    desk.desktop_height = 768;
    desk.dynamic_desktop = false;
    assert_eq!(connect_probe("desk_1024x768", desk).await.expect("desk_1024x768"), (1024, 768));
    let mut nla = settings(WIN11_HOST, WIN11_USER);
    nla.requested_protocols = RdpSettings::resolve_protocols(SECURITY_NLA, None);
    connect_probe("security_nla", nla).await.expect("security_nla");
    let mut console = settings(WIN11_HOST, WIN11_USER);
    console.console_session = true;
    connect_probe("console", console).await.expect("console");
    let mut timeout = settings(WIN11_HOST, WIN11_USER);
    timeout.connect_timeout_seconds = 3;
    connect_probe("timeout_3", timeout).await.expect("timeout_3");
    let mut keepalive = settings(WIN11_HOST, WIN11_USER);
    keepalive.keepalive_interval_seconds = 5;
    connect_probe("keepalive_5", keepalive).await.expect("keepalive_5");
    let mut tls = settings(WIN11_HOST, WIN11_USER);
    tls.requested_protocols = RdpSettings::resolve_protocols(SECURITY_TLS, None);
    let tls_error = connect_probe("security_tls", tls).await.expect_err("security_tls");
    assert!(tls_error.contains("服务端要求 NLA"), "{tls_error}");
    let mut cert = settings(WIN11_HOST, WIN11_USER);
    cert.ignore_cert = false;
    let cert_error = connect_probe("ignore_cert_off", cert).await.expect_err("ignore_cert_off");
    assert!(cert_error.contains("TLS 证书不受信任"), "{cert_error}");
    let mut aad = settings(WIN11_HOST, WIN11_USER);
    aad.aad_client_id = Some("11111111-1111-1111-1111-111111111111".to_string());
    aad.requested_protocols = RdpSettings::resolve_protocols(SECURITY_AUTO, aad.aad_client_id.as_deref());
    connect_probe("aad", aad).await.expect("aad");
}

fn rdp_security_pair() -> (RdpSecurity, RdpSecurity) {
    let client_random = [0x11u8; 32];
    let server_random = [0x22u8; 32];
    let client = RdpSecurity::establish(crypt::ENCRYPTION_METHOD_128BIT, &client_random, &server_random).expect("client keys");
    let server = RdpSecurity::establish_server(crypt::ENCRYPTION_METHOD_128BIT, &client_random, &server_random).expect("server keys");
    (client, server)
}

fn general_and_bitmap_caps(extra_flags: u16) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&24u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());
    data.extend_from_slice(&0x0200u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&extra_flags.to_le_bytes());
    data.extend_from_slice(&[0u8; 6]);
    data.push(1);
    data.push(1);
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&20u16.to_le_bytes());
    data.extend_from_slice(&32u16.to_le_bytes());
    data.extend_from_slice(&[0u8; 6]);
    data.extend_from_slice(&1024u16.to_le_bytes());
    data.extend_from_slice(&768u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data
}

fn autodetect_rtt_request() -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(10);
    security::write_header(&mut cursor, security::SEC_AUTODETECT_REQ);
    cursor.write_u8(0x06);
    cursor.write_u8(0x00);
    cursor.write_u16_le(1);
    cursor.write_u16_le(0x0001);
    cursor.into_vec()
}

#[test]
fn mac_signature_differs_from_salted() {
    let key = [0xABu8; 16];
    let data = b"rdp-mac";
    assert_ne!(security::mac_signature(&key, data), security::salted_mac_signature(&key, data, 0));
}

#[test]
fn standard_security_wraps_and_unwraps_fastpath_and_share() {
    let (mut client, mut server) = rdp_security_pair();
    let events = vec![0x20, 0x1D];
    let (flags, body) = client.wrap_fastpath(&events).expect("wrap fastpath");
    assert_eq!(flags, crypt::FASTPATH_ENCRYPTED | crypt::FASTPATH_SECURE_CHECKSUM);
    assert_eq!(server.unwrap_fastpath(flags, &body).expect("unwrap fastpath"), events);
    let share = vec![0x10, 0x00, 0x01, 0x02];
    let wrapped = client.wrap(0, &share).expect("wrap share");
    let header = u16::from_le_bytes(wrapped[..2].try_into().unwrap());
    assert_ne!(header & security::SEC_ENCRYPT, 0);
    assert_ne!(header & security::SEC_SECURE_CHECKSUM, 0);
    assert_eq!(server.unwrap(&wrapped).expect("unwrap share"), share);
}

#[test]
fn standard_security_uses_unsalted_mac_when_server_omits_enc_salted() {
    let (mut client, mut server) = rdp_security_pair();
    client.set_salted_checksum(false);
    server.set_salted_checksum(false);
    let events = vec![0x01, 0x02, 0x03];
    let (flags, body) = client.wrap_fastpath(&events).expect("wrap unsalted");
    assert_eq!(flags, crypt::FASTPATH_ENCRYPTED);
    assert_eq!(flags & crypt::FASTPATH_SECURE_CHECKSUM, 0);
    assert_eq!(server.unwrap_fastpath(flags, &body).expect("unwrap unsalted"), events);
    let wrapped = client.wrap(0, b"body").expect("wrap share unsalted");
    let header = u16::from_le_bytes(wrapped[..2].try_into().unwrap());
    assert_eq!(header & security::SEC_SECURE_CHECKSUM, 0);
    assert_eq!(server.unwrap(&wrapped).expect("unwrap share unsalted"), b"body");
}

#[test]
fn message_channel_decrypts_autodetect_before_process() {
    let (mut client, mut server) = rdp_security_pair();
    let wrapped = server.wrap(0, &autodetect_rtt_request()).expect("server wrap rtt");
    let plain = client.unwrap(&wrapped).expect("client unwrap rtt");
    assert_eq!(u16::from_le_bytes(plain[..2].try_into().unwrap()) & security::SEC_ENCRYPT, 0);
    let mut state = MessageChannelState::default();
    let response = msgchannel::process(&mut state, &plain).expect("process rtt").expect("rtt response");
    assert_ne!(u16::from_le_bytes(response[..2].try_into().unwrap()) & security::SEC_AUTODETECT_RSP, 0);
    let wrapped_response = client.wrap(0, &response).expect("client wrap rtt");
    let plain_response = server.unwrap(&wrapped_response).expect("server unwrap rtt");
    assert_ne!(u16::from_le_bytes(plain_response[..2].try_into().unwrap()) & security::SEC_AUTODETECT_RSP, 0);
}

#[test]
fn fastpath_encrypted_without_keys_fails() {
    let error = RdpSecurity::None.unwrap_fastpath(crypt::FASTPATH_ENCRYPTED, &[0u8; 16]).expect_err("missing keys");
    assert!(error.message.contains("未建立密钥"), "{}", error.message);
}

#[test]
fn general_capability_extra_flags_set_salted_checksum() {
    let with_salt = capabilities::parse_server_capability_sets(&mut ReadCursor::new(&general_and_bitmap_caps(0x0010)), 2).expect("salted");
    assert!(with_salt.salted_checksum);
    let without_salt = capabilities::parse_server_capability_sets(&mut ReadCursor::new(&general_and_bitmap_caps(0)), 2).expect("unsalted");
    assert!(!without_salt.salted_checksum);
}

async fn connect_and_dump(target: RdpSettings, require_gfx: bool) {
    let data_dir = std::env::temp_dir().join(format!("termini-rdp-test-{}", target.host));
    std::fs::create_dir_all(&data_dir).expect("create temp data dir");
    std::env::set_var("TERMINI_DATA_DIR", &data_dir);
    let handler = silent_handler();
    let connected = RdpSessionFactory.connect(&target, None, &handler, None).await.expect("rdp connect");
    eprintln!("connected desktop={}x{} global={} user={}", connected.desktop.width, connected.desktop.height, connected.channel.global_channel_id, connected.channel.user_id);
    let rdpsnd_id = connected.channels.channel_id(channel::RDPSND_CHANNEL_NAME).expect("rdpsnd channel id");
    eprintln!("rdpsnd channel id={}", rdpsnd_id);
    let input_flags = connected.input_flags;
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
        update_state: UpdateState::default(),
        framebuffer: Framebuffer::new(connected.desktop.width, connected.desktop.height),
        surfaces: GfxSurfaces::default(),
        gfx_zgfx: ZgfxContext::default(),
        gfx_pdus: 0,
        sound: RdpsndState::default(),
        device: RdpdrState::default(),
        color_depth: target.color_depth,
        keyboard_layout: target.keyboard_layout,
        expecting_wave: false,
        sound_types: Vec::new(),
        server_format_tags: Vec::new(),
        server_version: 0,
        client_volume: 0,
        client_version: 0,
        client_formats: 0,
        wave_count: 0,
        wave2_count: 0,
        training_count: 0,
        play_sound_count: 0,
        dyn_created: Vec::new(),
        disp_caps: None,
        demand_desktops: Vec::new(),
    };
    let (mut reader, mut writer) = connected.transport.split();
    let down = input::button_event(1, true, 100, 100, input_flags).expect("right down");
    let flags = u16::from_le_bytes([down[1], down[2]]);
    assert_eq!(flags, 0xA000);
    let up = input::button_event(1, false, 100, 100, input_flags).expect("right up");
    let pdus = fastpath::build_input_pdus(&[down, up], &mut probe.security).expect("input pdus");
    for pdu in pdus {
        writer.write_all(&pdu).await.expect("write right click");
        writer.flush().await.expect("flush right click");
    }
    for index in 0..40 {
        let frame = match tokio::time::timeout(std::time::Duration::from_secs(5), transport::read_frame(&mut reader)).await {
            Ok(Ok(frame)) => frame,
            Ok(Err(error)) => panic!("frame[{}] read failed: {}", index, error.message),
            Err(_) => panic!("frame[{}] timed out", index),
        };
        let result = match &frame {
            RdpFrame::FastPath { flags, payload } => {
                eprintln!("frame[{}] FastPath flags={} len={}", index, flags, payload.len());
                probe.handle_fastpath(*flags, payload).await
            }
            RdpFrame::X224(payload) => {
                eprintln!("frame[{}] X224 len={}", index, payload.len());
                probe.handle_x224(&mut writer, payload).await
            }
        };
        if let Err(error) = result {
            panic!("frame[{}] {}", index, error.message);
        }
    }
    eprintln!("gfx last_codec_id=0x{:04x} gfx_pdus={}", probe.surfaces.last_codec_id, probe.gfx_pdus);
    if require_gfx {
        assert!(probe.gfx_pdus > 0, "未解出任何 RDPGFX PDU");
    }
    let _reader: ReadHalf<RdpStream> = reader;
}

async fn probe_win7_sound_and_desktop() {
    let mut target = settings(WIN7_HOST, WIN10_USER);
    target.desktop_width = 1024;
    target.desktop_height = 768;
    let data_dir = std::env::temp_dir().join(format!("termini-rdp-test-{}", target.host));
    std::fs::create_dir_all(&data_dir).expect("create temp data dir");
    std::env::set_var("TERMINI_DATA_DIR", &data_dir);
    let handler = silent_handler();
    let connected = RdpSessionFactory.connect(&target, None, &handler, None).await.expect("rdp connect");
    eprintln!("requested=1024x768 desktop={}x{}", connected.desktop.width, connected.desktop.height);
    let rdpsnd_id = connected.channels.channel_id(channel::RDPSND_CHANNEL_NAME).expect("rdpsnd channel id");
    let drdynvc_id = connected.channels.channel_id(channel::DRDYNVC_CHANNEL_NAME).expect("drdynvc channel id");
    eprintln!("rdpsnd={} drdynvc={}", rdpsnd_id, drdynvc_id);
    let input_flags = connected.input_flags;
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
        update_state: UpdateState::default(),
        framebuffer: Framebuffer::new(connected.desktop.width, connected.desktop.height),
        surfaces: GfxSurfaces::default(),
        gfx_zgfx: ZgfxContext::default(),
        gfx_pdus: 0,
        sound: RdpsndState::default(),
        device: RdpdrState::default(),
        color_depth: target.color_depth,
        keyboard_layout: target.keyboard_layout,
        expecting_wave: false,
        sound_types: Vec::new(),
        server_format_tags: Vec::new(),
        server_version: 0,
        client_volume: 0,
        client_version: 0,
        client_formats: 0,
        wave_count: 0,
        wave2_count: 0,
        training_count: 0,
        play_sound_count: 0,
        dyn_created: Vec::new(),
        disp_caps: None,
        demand_desktops: Vec::new(),
    };
    let (mut reader, mut writer) = connected.transport.split();
    let down = input::button_event(0, true, 200, 200, input_flags).expect("left down");
    let up = input::button_event(0, false, 200, 200, input_flags).expect("left up");
    let key_down = input::scancode_event(0x1E, true, false).expect("A down");
    let key_up = input::scancode_event(0x1E, false, false).expect("A up");
    let pdus = fastpath::build_input_pdus(&[down, up, key_down, key_up], &mut probe.security).expect("input pdus");
    for pdu in pdus {
        writer.write_all(&pdu).await.expect("write input");
        writer.flush().await.expect("flush input");
    }
    let mut sent_disp = false;
    for index in 0..80 {
        let frame = match tokio::time::timeout(std::time::Duration::from_secs(3), transport::read_frame(&mut reader)).await {
            Ok(Ok(frame)) => frame,
            Ok(Err(error)) => panic!("frame[{}] read failed: {}", index, error.message),
            Err(_) => {
                eprintln!("frame[{}] timed out", index);
                break;
            }
        };
        let result = match &frame {
            RdpFrame::FastPath { flags, payload } => {
                eprintln!("frame[{}] FastPath flags={} len={}", index, flags, payload.len());
                probe.handle_fastpath(*flags, payload).await
            }
            RdpFrame::X224(payload) => {
                eprintln!("frame[{}] X224 len={}", index, payload.len());
                probe.handle_x224(&mut writer, payload).await
            }
        };
        if let Err(error) = result {
            panic!("frame[{}] {}", index, error.message);
        }
        if !sent_disp {
            if let Some(max_num_monitors) = probe.disp_caps {
                if max_num_monitors == 0 {
                    panic!("disp caps max_num_monitors=0");
                }
                let Some(dynamic_id) = probe.dynamic.channel_id(disp::CHANNEL_NAME) else {
                    panic!("disp caps without channel id");
                };
                let monitor = gcc::Monitor::from_size(1600, 900, 96, true).expect("monitor");
                let body = drdynvc::build_data(dynamic_id, &disp::build_monitor_layout(&[monitor]).expect("layout")).expect("disp data");
                probe.write_static(&mut writer, drdynvc_id, &body).await.expect("send disp");
                sent_disp = true;
                eprintln!("sent disp 1600x900");
            }
        }
    }
    eprintln!(
        "sound types={:?} server_ver={} server_tags={:?} client_vol=0x{:08x} client_ver={} client_formats={} wave={} wave2={} training={} play_sound={} dyn={:?} disp_caps={:?} demand={:?} sent_disp={}",
        probe.sound_types,
        probe.server_version,
        probe.server_format_tags,
        probe.client_volume,
        probe.client_version,
        probe.client_formats,
        probe.wave_count,
        probe.wave2_count,
        probe.training_count,
        probe.play_sound_count,
        probe.dyn_created,
        probe.disp_caps,
        probe.demand_desktops,
        sent_disp
    );
    assert_eq!((connected.desktop.width, connected.desktop.height), (1024, 768));
    assert!(probe.disp_caps.is_none(), "win7 出现了 Display Control: {:?}", probe.disp_caps);
    assert!(!probe.dyn_created.contains(&disp::CHANNEL_NAME), "win7 创建了 disp: {:?}", probe.dyn_created);
    if probe.sound_types.contains(&7) {
        assert!(probe.server_format_tags.contains(&0x0001), "win7 服务端格式无 PCM: {:?}", probe.server_format_tags);
        assert_eq!(probe.client_volume, 0xFFFF_FFFF);
        assert_eq!(probe.client_version, 8);
        assert!(probe.client_formats >= 1);
        assert!(probe.training_count > 0, "win7 未收到 rdpsnd TRAINING");
        assert!(probe.wave_count + probe.wave2_count + probe.play_sound_count > 0, "win7 握手后没有 WAVE/WAVE2/PLAY_SOUND");
    }
    let _reader: ReadHalf<RdpStream> = reader;
}

#[tokio::test]
async fn connect_windows7_sound_and_desktop() {
    probe_win7_sound_and_desktop().await;
}

#[tokio::test]
async fn connect_windows10_and_dump_first_failing_pdu() {
    connect_and_dump(settings(WIN10_HOST, WIN10_USER), true).await;
}

#[tokio::test]
async fn connect_windows11_and_dump_first_failing_pdu() {
    connect_and_dump(settings(WIN11_HOST, WIN11_USER), true).await;
}
