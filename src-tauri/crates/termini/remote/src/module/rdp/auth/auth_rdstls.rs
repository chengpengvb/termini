use crate::module::rdp::protocol::core::rdstls;
use crate::module::rdp::client_rdp_session::RdpSettings;
use crate::module::rdp::protocol::core::error::RdpResult;
use crate::module::rdp::protocol::core::transport::RdpTransport;
use crate::module::rdp::client_session_factory::RdpSessionFactory;

impl RdpSessionFactory {
    pub(crate) async fn authenticate_rdstls(&self, transport: &mut RdpTransport, settings: &RdpSettings) -> RdpResult<()> {
        let capabilities = transport.read_bytes(rdstls::CAPABILITIES_LENGTH).await?;
        let version = rdstls::parse_capabilities(&capabilities)?;
        let rdstls_settings = rdstls::RdstlsSettings {
            username: settings.username.clone(),
            domain: settings.domain.clone(),
            session_id: settings.redirection_session_id,
            password: settings.redirection_password.clone(),
            redirection_guid: settings.redirection_guid.clone(),
            auto_reconnect: settings.auto_reconnect,
        };
        transport.send(&rdstls::build_authentication_request(version, &rdstls_settings)?).await?;
        let response = transport.read_bytes(rdstls::AUTHENTICATION_RESPONSE_LENGTH).await?;
        rdstls::parse_authentication_response(&response)
    }
}
