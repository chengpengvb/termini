use crate::module::rdp::protocol::core::mcs;
use crate::module::rdp::protocol::core::info;
use crate::module::rdp::protocol::core::license;
use crate::module::rdp::protocol::core::security;
use crate::module::rdp::protocol::core::msgchannel;
use crate::module::rdp::protocol::core::crypt::RdpSecurity;
use crate::module::rdp::client_rdp_license as license_store;
use crate::module::rdp::protocol::core::transport::RdpTransport;
use crate::module::rdp::client_session_factory::RdpSessionFactory;
use crate::module::rdp::protocol::core::msgchannel::MessageChannelState;
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};
use crate::module::rdp::client_rdp_session::{RdpChannel, RdpSettings, CLIENT_NAME};

impl RdpSessionFactory {
    pub(crate) async fn exchange_license(
        &self,
        transport: &mut RdpTransport,
        settings: &RdpSettings,
        channel: &RdpChannel,
        security: &mut RdpSecurity,
        message: &mut MessageChannelState,
    ) -> RdpResult<()> {
        let client_info = info::ClientInfoSettings {
            domain: settings.domain.clone(),
            username: settings.username.clone(),
            password: settings.password.clone(),
            performance_flags: settings.performance_flags,
            redirection_password: settings.redirection_password.clone(),
            auto_reconnect: settings.auto_reconnect,
        };
        self.send_share(transport, channel, security, security::SEC_INFO_PKT, &info::build_client_info(&client_info)?).await?;

        let stored = license_store::load(CLIENT_NAME)?;
        let mut license = license::LicenseState::new(settings.username.clone(), CLIENT_NAME.to_string(), stored);
        loop {
            let payload = self.read_share_payload(transport, channel, security, message).await?;
            match license::process(&mut license, &payload)? {
                license::LicenseOutcome::Complete(issued) => {
                    if let Some(issued) = issued {
                        license_store::store(CLIENT_NAME, &issued)?;
                    }
                    return Ok(());
                }
                license::LicenseOutcome::Respond(body) => self.send_share(transport, channel, security, security::SEC_LICENSE_PKT, &body).await?,
            }
        }
    }

    pub(crate) async fn send_share(&self, transport: &mut RdpTransport, channel: &RdpChannel, security: &mut RdpSecurity, flags: u16, body: &[u8]) -> RdpResult<()> {
        let wrapped = security.wrap(flags, body)?;
        self.send_channel(transport, channel, &wrapped).await
    }

    pub(crate) async fn read_share_payload(&self, transport: &mut RdpTransport, channel: &RdpChannel, security: &mut RdpSecurity, message: &mut MessageChannelState) -> RdpResult<Vec<u8>> {
        loop {
            let indication = mcs::parse_send_data_indication(&transport.read_x224().await?)?;
            let payload = security.unwrap(&indication.payload)?;
            if indication.channel_id == channel.global_channel_id {
                return Ok(payload);
            }
            if channel.message_channel_id == 0 || indication.channel_id != channel.message_channel_id {
                return Err(RdpProtocolError::protocol(format!("received virtual channel data during connection phase: id={}", indication.channel_id)));
            }
            if let Some(response) = msgchannel::process(message, &payload)? {
                let wrapped = security.wrap(0, &response)?;
                let bytes = mcs::build_send_data_request(channel.user_id, channel.message_channel_id, &wrapped)?;
                transport.send(&bytes).await?;
            }
        }
    }
}
