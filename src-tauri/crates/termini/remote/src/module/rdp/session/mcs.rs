use crate::module::rdp::protocol::core::gcc;
use crate::module::rdp::protocol::core::mcs;
use crate::module::rdp::protocol::core::nego;
use crate::module::rdp::protocol::core::crypt;
use crate::module::rdp::protocol::crypto::rsa;
use crate::module::rdp::protocol::core::channel;
use crate::module::rdp::protocol::core::security;
use crate::module::rdp::protocol::crypto::certificate;
use crate::module::rdp::protocol::core::crypt::RdpSecurity;
use crate::module::rdp::protocol::core::stream::WriteCursor;
use crate::module::rdp::protocol::core::channel::ChannelTable;
use crate::module::rdp::protocol::core::security::random_bytes;
use crate::module::rdp::protocol::core::transport::RdpTransport;
use crate::module::rdp::client_session_factory::RdpSessionFactory;
use crate::module::rdp::client_rdp_session::{RdpChannel, RdpSettings};
use crate::module::rdp::client_rdp_session::{desktop_monitors, CLIENT_NAME};
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};

const CLIENT_RANDOM_LENGTH: usize = 32;
const CLIENT_RANDOM_PADDING: usize = 8;

impl RdpSessionFactory {
    pub(crate) async fn connect_mcs(&self, transport: &mut RdpTransport, settings: &RdpSettings, negotiation: &nego::NegotiationResponse) -> RdpResult<(RdpChannel, ChannelTable, RdpSecurity)> {
        let core = gcc::ClientCoreSettings {
            selected_protocol: negotiation.selected_protocol,
            extended_client_data: negotiation.extended_client_data,
            desktop_width: settings.desktop_width,
            desktop_height: settings.desktop_height,
            color_depth: settings.color_depth,
            keyboard_layout: settings.keyboard_layout,
            client_name: CLIENT_NAME.to_string(),
            console_session: settings.console_session,
            encryption_methods: crypt::ENCRYPTION_METHOD_40BIT | crypt::ENCRYPTION_METHOD_56BIT | crypt::ENCRYPTION_METHOD_128BIT,
            monitors: desktop_monitors(settings.desktop_width, settings.desktop_height, settings.desktop_dpi)?,
        };
        let client_data = gcc::build_client_data(&core, &channel::CLIENT_CHANNELS)?;
        let conference = gcc::build_conference_create_request(&client_data)?;
        transport.send(&mcs::build_connect_initial(&conference)?).await?;

        let response = transport.read_x224().await?;
        let server_data = gcc::parse_conference_create_response(&mcs::parse_connect_response(&response)?)?;

        transport.send(&mcs::build_erect_domain_request()?).await?;
        transport.send(&mcs::build_attach_user_request()?).await?;
        let user_id = mcs::parse_attach_user_confirm(&transport.read_x224().await?)?;

        let mut channels = vec![user_id, server_data.mcs_channel_id];
        if server_data.message_channel_id != 0 {
            channels.push(server_data.message_channel_id);
        }
        channels.extend_from_slice(&server_data.channel_ids);
        for channel_id in channels {
            transport.send(&mcs::build_channel_join_request(user_id, channel_id)?).await?;
            let joined = mcs::parse_channel_join_confirm(&transport.read_x224().await?)?;
            if joined != channel_id {
                return Err(RdpProtocolError::protocol(format!("MCS channel join mismatch: requested {} got {}", channel_id, joined)));
            }
        }
        let channel = RdpChannel { user_id, share_id: 0, global_channel_id: server_data.mcs_channel_id, message_channel_id: server_data.message_channel_id };
        let security = self.exchange_keys(transport, &channel, &server_data).await?;
        Ok((channel, ChannelTable::new(&server_data.channel_ids)?, security))
    }

    async fn exchange_keys(&self, transport: &mut RdpTransport, channel: &RdpChannel, server_data: &gcc::ServerData) -> RdpResult<RdpSecurity> {
        if server_data.encryption_level == security::ENCRYPTION_LEVEL_NONE {
            return Ok(RdpSecurity::None);
        }
        let public_key = certificate::parse_server_certificate(&server_data.server_certificate)?;
        let client_random = random_bytes(CLIENT_RANDOM_LENGTH);
        let encrypted = rsa::public_encrypt(&client_random, &public_key.modulus, &public_key.exponent)?;

        let mut body = WriteCursor::with_capacity(encrypted.len() + 16);
        security::write_header(&mut body, security::SEC_EXCHANGE_PKT | security::SEC_LICENSE_ENCRYPT_SC);
        body.write_u32_le(u32::try_from(encrypted.len() + CLIENT_RANDOM_PADDING).map_err(|_| RdpProtocolError::encoding("client random is too long".to_string()))?);
        body.write_slice(&encrypted);
        body.write_zeros(CLIENT_RANDOM_PADDING);
        self.send_channel(transport, channel, body.as_slice()).await?;
        RdpSecurity::establish(server_data.encryption_method, &client_random, &server_data.server_random)
    }

    pub(crate) async fn send_channel(&self, transport: &mut RdpTransport, channel: &RdpChannel, body: &[u8]) -> RdpResult<()> {
        let bytes = mcs::build_send_data_request(channel.user_id, channel.global_channel_id, body)?;
        transport.send(&bytes).await
    }
}
