use serde_json::Value;
use crate::util::id_factory::uuid;
use crate::module::rdp::protocol::core::nego;
use crate::module::rdp::auth::auth_gate::AuthGate;
use crate::module::rdp::auth::auth_gate::CERT_ALGORITHM;
use crate::module::common::client_terminal::EventHandler;
use crate::module::rdp::auth::auth_flow::config_password;
use crate::module::rdp::auth::auth_gate::rdp_cert_prompt;
use crate::module::rdp::protocol::codec::bulk::BulkContext;
use crate::module::rdp::protocol::core::crypt::RdpSecurity;
use crate::module::common::context::context_rdp::ContextRdp;
use crate::module::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::module::rdp::protocol::core::channel::ChannelTable;
use crate::module::common::dto::ssh_auth_prompt::FingerprintPrompt;
use crate::module::rdp::client_rdp_session::{RdpChannel, RdpSettings};
use crate::module::rdp::protocol::core::msgchannel::MessageChannelState;
use crate::module::rdp::protocol::core::transport::{RdpTransport, TlsUpgradeResult};
use crate::module::rdp::protocol::core::error::{RdpError, RdpProtocolError, RdpResult};
use crate::model::resp::event::resp_event_rdp::{OpenRdpStep, RespEventRdp, RespRdpDesktopSize, RespRdpOpenStepType as Step};

#[derive(Clone, Copy)]
pub struct RdpSessionFactory;

pub struct ConnectedRdp {
    pub input_flags: u16,
    pub bulk: BulkContext,
    pub refresh_rect: bool,
    pub channel: RdpChannel,
    pub suppress_output: bool,
    pub security: RdpSecurity,
    pub channels: ChannelTable,
    pub transport: RdpTransport,
    pub frame_acknowledge: bool,
    pub desktop: RespRdpDesktopSize,
    pub message: MessageChannelState,
    pub multifragment_update: Option<u32>,
}

impl RdpSessionFactory {
    pub async fn connect(&self, base_settings: &RdpSettings, context: Option<&ContextRdp>, handler: &EventHandler<RespEventRdp>, gate: Option<&AuthGate>) -> RdpResult<ConnectedRdp> {
        let mut settings = base_settings.clone();
        loop {
            match self.handshake(&settings, handler, gate).await {
                Ok(connected) => return Ok(connected),
                Err(error) if error.kind == RdpError::Authentication => match (context, gate) {
                    (Some(context), Some(gate)) => {
                        self.prompt_password_retry(gate, handler, &error.message).await?;
                        settings.password = config_password(context).unwrap_or_default();
                    }
                    _ => return self.fail(handler, Step::ErrorAuthFailed, 38.0, error).await,
                },
                Err(error) => return Err(error),
            }
        }
    }

    async fn handshake(&self, settings: &RdpSettings, handler: &EventHandler<RespEventRdp>, gate: Option<&AuthGate>) -> RdpResult<ConnectedRdp> {
        let mut bulk = BulkContext::default();
        let mut message = MessageChannelState::default();
        let mut ignore_cert = settings.ignore_cert;

        let (mut transport, public_key, selected, negotiation) = loop {
            self.emit_open_step(handler, Step::TcpConnecting, 5.0, None).await;
            let mut transport = match RdpTransport::connect(&settings.host, settings.port, settings.connect_timeout_seconds, settings.keepalive_interval_seconds).await {
                Ok(transport) => transport,
                Err(error) => return self.fail(handler, Step::ErrorTcpFailed, 5.0, error).await,
            };
            self.emit_open_step(handler, Step::TcpConnected, 12.0, None).await;

            self.emit_open_step(handler, Step::Negotiating, 18.0, None).await;
            let negotiation = match self.negotiate(&mut transport, settings).await {
                Ok(negotiation) => negotiation,
                Err(error) => return self.fail(handler, Step::ErrorNegotiateFailed, 18.0, error).await,
            };
            let selected = negotiation.selected_protocol;
            if selected == nego::PROTOCOL_RDP {
                break (transport, Vec::new(), selected, negotiation);
            }

            self.emit_open_step(handler, Step::TlsHandshaking, 24.0, None).await;
            let stored = gate.and_then(|gate| gate.known_host.find_by_host_port(&settings.host, settings.port).map(|ptr| ptr.read().clone()));
            let trusted_sha256 = stored.as_ref().filter(|item| item.algorithm == CERT_ALGORITHM).map(|item| item.sha256.as_str());
            match transport.upgrade_tls(&settings.host, ignore_cert, trusted_sha256).await {
                Ok(TlsUpgradeResult::Established { transport, public_key, fingerprint }) => {
                    self.emit_open_step(handler, Step::TlsEstablished, 32.0, Some(Value::String(fingerprint))).await;
                    break (transport, public_key, selected, negotiation);
                }
                Ok(TlsUpgradeResult::Untrusted { fingerprint }) => {
                    let Some(gate) = gate else {
                        return self.fail(handler, Step::ErrorTlsFailed, 24.0, RdpProtocolError::protocol(format!("TLS certificate is not trusted: {}", fingerprint))).await;
                    };
                    match rdp_cert_prompt(stored.as_ref().map(|item| item.algorithm.as_str()), stored.as_ref().map(|item| item.sha256.as_str()), &fingerprint) {
                        None => {
                            return self.fail(handler, Step::ErrorTlsFailed, 24.0, RdpProtocolError::protocol(format!("TLS did not accept the trusted certificate: {}", fingerprint))).await;
                        }
                        Some((reason, previous_sha256)) => {
                            let request_id = uuid();
                            let payload = FingerprintPrompt {
                                request_id: request_id.clone(),
                                port: settings.port,
                                reason,
                                host: settings.host.clone(),
                                sha256: fingerprint.clone(),
                                algorithm: CERT_ALGORITHM.to_string(),
                                previous_sha256,
                            };
                            let data = serde_json::to_value(payload).map_err(|error| RdpProtocolError::encoding(format!("failed to serialize certificate prompt: {}", error)))?;
                            let rx = gate.arm(&request_id);
                            self.emit_open_step(handler, Step::CertFingerprintConfirm, 28.0, Some(data)).await;
                            let answer = gate.wait(&request_id, rx).await?;
                            match answer {
                                SshAuthAnswer::HostFingerprintTrustOnce => {
                                    ignore_cert = true;
                                    continue;
                                }
                                SshAuthAnswer::HostFingerprintTrustAlways => {
                                    if let Err(error) = gate.known_host.upsert_trusted(settings.host.clone(), settings.port, CERT_ALGORITHM.to_string(), fingerprint).await {
                                        return self.fail(handler, Step::ErrorTlsFailed, 24.0, RdpProtocolError::protocol(format!("failed to save host certificate fingerprint: {}", error))).await;
                                    }
                                    ignore_cert = true;
                                    continue;
                                }
                                SshAuthAnswer::HostFingerprintReject => {
                                    return self.fail(handler, Step::ErrorTlsFailed, 24.0, RdpProtocolError::protocol("TLS certificate rejected".to_string())).await;
                                }
                                _ => return self.fail(handler, Step::ErrorTlsFailed, 24.0, RdpProtocolError::protocol("invalid TLS certificate confirmation answer".to_string())).await,
                            }
                        }
                    }
                }
                Err(error) => return self.fail(handler, Step::ErrorTlsFailed, 24.0, error).await,
            }
        };

        if selected & (nego::PROTOCOL_HYBRID | nego::PROTOCOL_HYBRID_EX) != 0 {
            self.emit_open_step(handler, Step::Authenticating, 38.0, None).await;
            self.authenticate(&mut transport, settings, public_key, selected).await.map_err(|error| RdpProtocolError::authentication(error.message))?;
            self.emit_open_step(handler, Step::Authenticated, 48.0, None).await;
        }

        if selected & nego::PROTOCOL_RDSAAD != 0 {
            self.emit_open_step(handler, Step::Authenticating, 38.0, None).await;
            self.authenticate_aad(&mut transport, settings).await.map_err(|error| RdpProtocolError::authentication(error.message))?;
            self.emit_open_step(handler, Step::Authenticated, 48.0, None).await;
        }

        if selected & nego::PROTOCOL_RDSTLS != 0 {
            self.emit_open_step(handler, Step::Authenticating, 38.0, None).await;
            self.authenticate_rdstls(&mut transport, settings).await.map_err(|error| RdpProtocolError::authentication(error.message))?;
            self.emit_open_step(handler, Step::Authenticated, 48.0, None).await;
        }

        self.emit_open_step(handler, Step::McsConnecting, 54.0, None).await;
        let (channel, mut channels, mut security) = match self.connect_mcs(&mut transport, settings, &negotiation).await {
            Ok(connected) => connected,
            Err(error) => return self.fail(handler, Step::ErrorMcsFailed, 54.0, error).await,
        };
        self.emit_open_step(handler, Step::McsConnected, 64.0, None).await;

        self.emit_open_step(handler, Step::Licensing, 70.0, None).await;
        if let Err(error) = self.exchange_license(&mut transport, settings, &channel, &mut security, &mut message).await {
            return self.fail(handler, Step::ErrorLicenseFailed, 70.0, error).await;
        }

        self.emit_open_step(handler, Step::CapabilitiesExchanging, 80.0, None).await;
        let activated = match self.exchange_capabilities(&mut transport, settings, &channel, &mut channels, &mut bulk, &mut security, &mut message).await {
            Ok(activated) => activated,
            Err(error) => return self.fail(handler, Step::ErrorCapabilitiesFailed, 80.0, error).await,
        };
        let demand = activated.demand;

        self.emit_open_step(handler, Step::Activating, 90.0, None).await;
        let channel = RdpChannel { share_id: demand.share_id, ..channel };
        if let Err(error) = self.finalize(&mut transport, &channel, &mut bulk, &mut security, &mut message).await {
            return self.fail(handler, Step::ErrorActivateFailed, 90.0, error).await;
        }

        let desktop = RespRdpDesktopSize { width: demand.capabilities.desktop_width, height: demand.capabilities.desktop_height };
        Ok(ConnectedRdp {
            transport,
            channel,
            channels,
            bulk,
            security,
            message,
            desktop,
            refresh_rect: demand.capabilities.refresh_rect,
            suppress_output: demand.capabilities.suppress_output,
            frame_acknowledge: demand.capabilities.frame_acknowledge.is_some_and(|value| value > 0),
            input_flags: activated.input_flags,
            multifragment_update: demand.capabilities.multifragment_update,
        })
    }

    async fn negotiate(&self, transport: &mut RdpTransport, settings: &RdpSettings) -> RdpResult<nego::NegotiationResponse> {
        let request = nego::build_connection_request(settings.routing_token.as_deref(), Some(&settings.username), settings.requested_protocols)?;
        transport.send(&request).await?;
        let confirm = transport.read_x224().await?;
        let response = nego::parse_connection_confirm(&confirm)?;
        let implemented = nego::PROTOCOL_RDP | nego::PROTOCOL_SSL | nego::PROTOCOL_HYBRID | nego::PROTOCOL_HYBRID_EX | nego::PROTOCOL_RDSTLS | nego::PROTOCOL_RDSAAD;
        if response.selected_protocol != nego::PROTOCOL_RDP && response.selected_protocol & implemented == 0 {
            return Err(RdpProtocolError::unsupported(format!("server selected an unimplemented security layer: {}", nego::describe_protocol(response.selected_protocol))));
        }
        Ok(response)
    }

    pub(crate) async fn emit_open_step(&self, handler: &EventHandler<RespEventRdp>, step_type: Step, progress: f32, data: Option<Value>) {
        let step = OpenRdpStep { progress, step_type, data };
        let _ = handler(RespEventRdp::OnOpenStep(step)).await;
    }

    pub(crate) async fn fail(&self, handler: &EventHandler<RespEventRdp>, step_type: Step, progress: f32, error: RdpProtocolError) -> RdpResult<ConnectedRdp> {
        self.emit_open_step(handler, step_type, progress, Some(Value::String(error.message.clone()))).await;
        Err(error)
    }
}
