use std::sync::Arc;
use std::time::Duration;
use summer::tracing::debug;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use tokio_rustls::TlsConnector;
use crate::dao::dao_base::BaseDao;
use x509_parser::prelude::FromDer;
use crate::dao::dao_device::DeviceDao;
use crate::collect::dto::CollectContext;
use crate::collect::collector::Collector;
use crate::collect::collector::AgentCollector;
use x509_parser::certificate::X509Certificate;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::job_indicator::JobIndicatorProtocol;
use crate::model::job_indicator_value::JobIndicatorValue;
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_rustls::rustls::{self, ClientConfig, DigitallySignedStruct, SignatureScheme};
use crate::model::dto::job::indicator_arg_template::{IndicatorArgTemplate, IndicatorArgType};
use tokio_rustls::rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};

pub const TLS_CERT_ARG_ID: &str = "tls-port";
pub const TLS_CERT_PORT_DEFAULT: u16 = 443;
pub const TLS_CERT_TIMEOUT: Duration = Duration::from_secs(10);
pub const TLS_CERT_ARG_LABEL_I18N: &str = "job.indicator.arg.tlsCert.port";

#[base_app(sort = 256)]
#[derive(Clone, Service)]
pub struct CollectorTlsCert {
    #[inject(component)]
    device_dao: DeviceDao,

    #[inject(component)]
    agent_collector: AgentCollector,
}

#[async_trait]
impl BaseApp for CollectorTlsCert {
    fn name(&self) -> &'static str {
        "CollectorTlsCert"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.agent_collector.register(Arc::new(self.clone()));
        Ok(())
    }
}

#[async_trait]
impl Collector for CollectorTlsCert {
    fn get_name(&self) -> &'static str {
        "CollectorTlsCert"
    }

    fn get_protocol(&self) -> i16 {
        JobIndicatorProtocol::TlsCert as i16
    }

    fn get_arg_template(&self) -> IndicatorArgTemplate {
        IndicatorArgTemplate { id: TLS_CERT_ARG_ID.to_string(), arg_type: IndicatorArgType::Number as i16, label_i18n: TLS_CERT_ARG_LABEL_I18N.to_string(), protocol: self.get_protocol() }
    }

    async fn collection(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let host = self.resolve_host(ctx).await?;
        let port = Self::resolve_port(ctx);
        let value = match Self::cert_days_left(&host, port).await {
            Ok(days) => ctx.new_value(Some(format!("{:.1}", days)), true, None),
            Err(error) => ctx.new_value(None, false, Some(format!("TLS certificate probe failed: {}", error))),
        };
        debug!("[collect] job_id={} host={} port={} success={} value={:?}", ctx.job_id(), host, port, value.success, value.value);
        Ok(vec![value])
    }
}

impl CollectorTlsCert {
    async fn resolve_host(&self, ctx: &CollectContext) -> anyhow::Result<String> {
        let device_id = ctx.device_id();
        let Some(device) = self.device_dao.find_by_id(device_id.clone()).await? else {
            anyhow::bail!("device not found, it may have been deleted");
        };
        let host = device.read().host.clone();
        if host.trim().is_empty() {
            anyhow::bail!("device has no IP or hostname configured");
        }
        Ok(host)
    }

    fn resolve_port(ctx: &CollectContext) -> u16 {
        let port = ctx.arg().and_then(|arg| arg.value.trim().parse::<u16>().ok()).unwrap_or(TLS_CERT_PORT_DEFAULT);
        if port == 0 {
            return TLS_CERT_PORT_DEFAULT;
        }
        port
    }

    async fn cert_days_left(host: &str, port: u16) -> anyhow::Result<f64> {
        let server_name = ServerName::try_from(host.to_string())?;
        let config = ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyCert))
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));

        let connect = tokio::net::TcpStream::connect((host, port));
        let stream = tokio::time::timeout(TLS_CERT_TIMEOUT, connect).await.map_err(|_| anyhow::anyhow!("connection timed out ({}s)", TLS_CERT_TIMEOUT.as_secs()))??;
        let handshake = connector.connect(server_name, stream);
        let tls_stream = tokio::time::timeout(TLS_CERT_TIMEOUT, handshake).await.map_err(|_| anyhow::anyhow!("TLS handshake timed out ({}s)", TLS_CERT_TIMEOUT.as_secs()))??;

        let (_, session) = tls_stream.get_ref();
        let Some(certs) = session.peer_certificates() else {
            anyhow::bail!("server returned no certificate");
        };
        let Some(leaf) = certs.first() else {
            anyhow::bail!("server certificate chain is empty");
        };
        let (_, cert) = X509Certificate::from_der(leaf.as_ref()).map_err(|error| anyhow::anyhow!("failed to parse certificate: {}", error))?;
        let not_after = cert.validity().not_after.timestamp();
        let now = chrono::Utc::now().timestamp();
        Ok((not_after - now) as f64 / 86400.0)
    }
}

#[derive(Debug)]
struct AcceptAnyCert;

impl ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(&self, _end_entity: &CertificateDer<'_>, _intermediates: &[CertificateDer<'_>], _server_name: &ServerName<'_>, _ocsp_response: &[u8], _now: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(&self, _message: &[u8], _cert: &CertificateDer<'_>, _dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(&self, _message: &[u8], _cert: &CertificateDer<'_>, _dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}
