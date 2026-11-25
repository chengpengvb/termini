use sspi::credssp::ClientState;
use crate::module::rdp::protocol::core::nego;
use crate::module::rdp::protocol::sspi::credssp;
use crate::module::rdp::client_rdp_session::CLIENT_NAME;
use crate::module::rdp::client_rdp_session::RdpSettings;
use crate::module::rdp::protocol::core::transport::RdpTransport;
use crate::module::rdp::client_session_factory::RdpSessionFactory;
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};

impl RdpSessionFactory {
    pub(crate) async fn authenticate(&self, transport: &mut RdpTransport, settings: &RdpSettings, public_key: Vec<u8>, selected_protocol: u32) -> RdpResult<()> {
        let mut client = credssp::new_client(&settings.host, &settings.domain, &settings.username, &settings.password, CLIENT_NAME, public_key)?;
        let mut incoming = sspi::credssp::TsRequest::default();
        loop {
            match credssp::process(&mut client, incoming)? {
                ClientState::ReplyNeeded(request) => {
                    transport.send(&credssp::encode_sspi_ts_request(&request)?).await?;
                    incoming = credssp::decode_sspi_ts_request(&transport.read_der().await?)?;
                }
                ClientState::FinalMessage(request) => {
                    transport.send(&credssp::encode_sspi_ts_request(&request)?).await?;
                    break;
                }
            }
        }
        if selected_protocol & nego::PROTOCOL_HYBRID_EX == 0 {
            return Ok(());
        }
        let result = transport.read_u32_le().await?;
        if result != credssp::AUTHZ_SUCCESS {
            return Err(RdpProtocolError::rejected(credssp::describe_early_user_auth(result)));
        }
        Ok(())
    }
}
