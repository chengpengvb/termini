use crate::module::rdp::protocol::core::pdu;
use crate::module::rdp::protocol::core::errinfo;
use crate::module::rdp::protocol::core::activation;
use crate::module::rdp::protocol::core::capabilities;
use crate::module::rdp::protocol::codec::bulk::BulkContext;
use crate::module::rdp::protocol::core::crypt::RdpSecurity;
use crate::module::rdp::protocol::core::stream::ReadCursor;
use crate::module::rdp::protocol::core::channel::ChannelTable;
use crate::module::rdp::protocol::core::transport::RdpTransport;
use crate::module::rdp::client_session_factory::RdpSessionFactory;
use crate::module::rdp::client_rdp_session::{RdpChannel, RdpSettings};
use crate::module::rdp::protocol::core::msgchannel::MessageChannelState;
use crate::module::rdp::protocol::core::capabilities::ClientCapabilities;
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum FinalizeProgress {
    Pending,
    FontMap,
}

pub(crate) struct ActivatedRdp {
    pub(crate) demand: activation::DemandActive,
    pub(crate) input_flags: u16,
}

impl RdpSessionFactory {
    pub(crate) async fn exchange_capabilities(
        &self,
        transport: &mut RdpTransport,
        settings: &RdpSettings,
        channel: &RdpChannel,
        channels: &mut ChannelTable,
        bulk: &mut BulkContext,
        security: &mut RdpSecurity,
        message: &mut MessageChannelState,
    ) -> RdpResult<ActivatedRdp> {
        let demand = loop {
            let payload = self.read_share_payload(transport, channel, security, message).await?;
            let mut cursor = ReadCursor::new(&payload);
            let header = pdu::read_share_control_header(&mut cursor)?;
            match header.pdu_type {
                pdu::PDU_TYPE_DEMAND_ACTIVE => break activation::parse_demand_active(&mut cursor)?,
                pdu::PDU_TYPE_DATA => {
                    self.inspect_data_pdu(&mut cursor, bulk)?;
                }
                other => return Err(RdpProtocolError::protocol(format!("unexpected PDU during capability negotiation: 0x{:02x}", other))),
            }
        };

        channels.set_chunk_size(demand.capabilities.virtual_channel_chunk_size);
        security.set_salted_checksum(demand.capabilities.salted_checksum);
        let input_flags = capabilities::negotiate_input_flags(demand.capabilities.input_flags);
        let capabilities = ClientCapabilities {
            node_id: channel.user_id,
            color_depth: settings.color_depth,
            keyboard_layout: settings.keyboard_layout,
            desktop_width: demand.capabilities.desktop_width,
            desktop_height: demand.capabilities.desktop_height,
            virtual_channel_chunk_size: channels.chunk_size(),
            input_flags,
            order_support: capabilities::negotiate_order_support(&demand.capabilities.order_support),
            large_pointer: demand.capabilities.large_pointer,
            multifragment_update: demand.capabilities.multifragment_update,
            surface_commands: demand.capabilities.surface_commands,
            bitmap_codecs: demand.capabilities.bitmap_codecs,
            frame_acknowledge: Some(2),
        };
        let confirm = activation::build_confirm_active(demand.share_id, channel.user_id, &capabilities)?;
        self.send_share(transport, channel, security, 0, &confirm).await?;
        Ok(ActivatedRdp { demand, input_flags })
    }

    pub(crate) fn inspect_data_pdu(&self, cursor: &mut ReadCursor<'_>, bulk: &mut BulkContext) -> RdpResult<FinalizeProgress> {
        let header = pdu::read_share_data_header(cursor)?;
        let body = pdu::read_data_pdu_body(cursor, &header, bulk)?;
        let mut body = ReadCursor::new(&body);
        match header.pdu_type2 {
            pdu::PDU_TYPE2_FONT_MAP => Ok(FinalizeProgress::FontMap),
            pdu::PDU_TYPE2_SET_ERROR_INFO => {
                let code = body.read_u32_le()?;
                if code == 0 {
                    Ok(FinalizeProgress::Pending)
                } else {
                    Err(RdpProtocolError::rejected(errinfo::describe(code)))
                }
            }
            _ => Ok(FinalizeProgress::Pending),
        }
    }
}
