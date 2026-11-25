use crate::module::rdp::protocol::core::pdu;
use crate::module::rdp::protocol::core::activation;
use crate::module::rdp::client_rdp_session::RdpChannel;
use crate::module::rdp::protocol::codec::bulk::BulkContext;
use crate::module::rdp::protocol::core::crypt::RdpSecurity;
use crate::module::rdp::protocol::core::stream::ReadCursor;
use crate::module::rdp::protocol::core::transport::RdpTransport;
use crate::module::rdp::session::capabilities::FinalizeProgress;
use crate::module::rdp::client_session_factory::RdpSessionFactory;
use crate::module::rdp::protocol::core::msgchannel::MessageChannelState;
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};

impl RdpSessionFactory {
    pub(crate) async fn finalize(&self, transport: &mut RdpTransport, channel: &RdpChannel, bulk: &mut BulkContext, security: &mut RdpSecurity, message: &mut MessageChannelState) -> RdpResult<()> {
        let synchronize = activation::build_synchronize(channel.share_id, channel.user_id, channel.global_channel_id)?;
        self.send_share(transport, channel, security, 0, &synchronize).await?;
        let cooperate = activation::build_control(channel.share_id, channel.user_id, pdu::CTRL_ACTION_COOPERATE)?;
        self.send_share(transport, channel, security, 0, &cooperate).await?;
        let request_control = activation::build_control(channel.share_id, channel.user_id, pdu::CTRL_ACTION_REQUEST_CONTROL)?;
        self.send_share(transport, channel, security, 0, &request_control).await?;
        let persist = activation::build_persistent_key_list(channel.share_id, channel.user_id, &[Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()])?;
        self.send_share(transport, channel, security, 0, &persist).await?;
        let font_list = activation::build_font_list(channel.share_id, channel.user_id)?;
        self.send_share(transport, channel, security, 0, &font_list).await?;

        loop {
            let payload = self.read_share_payload(transport, channel, security, message).await?;
            let mut cursor = ReadCursor::new(&payload);
            let header = pdu::read_share_control_header(&mut cursor)?;
            if header.pdu_type != pdu::PDU_TYPE_DATA {
                return Err(RdpProtocolError::protocol(format!("unexpected PDU during activation: 0x{:02x}", header.pdu_type)));
            }
            if self.inspect_data_pdu(&mut cursor, bulk)? == FinalizeProgress::FontMap {
                return Ok(());
            }
        }
    }
}
