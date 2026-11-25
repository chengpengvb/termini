use std::pin::Pin;
use std::sync::Arc;
use std::net::IpAddr;
use std::str::FromStr;
use parking_lot::Mutex;
use tokio::net::TcpStream;
use tokio_openssl::SslStream;
use crate::crypto::certificate;
use openssl::x509::X509VerifyResult;
use crate::core::error::{RdpProtocolError, RdpResult};
use openssl::ssl::{Ssl, SslContext, SslMethod, SslMode, SslOptions, SslVerifyMode, SslVersion};

pub struct TlsUpgrade {
    pub public_key: Vec<u8>,
    pub fingerprint: String,
    pub stream: SslStream<TcpStream>,
}

pub enum TlsOutcome {
    Established(TlsUpgrade),
    Untrusted { fingerprint: String },
}

pub async fn upgrade(stream: TcpStream, host: &str, ignore_cert: bool, trusted_sha256: Option<&str>) -> RdpResult<TlsOutcome> {
    let fingerprint = Arc::new(Mutex::new(None));
    let mut builder = SslContext::builder(SslMethod::tls_client()).map_err(|error| RdpProtocolError::protocol(format!("failed to build TLS config: {}", error)))?;
    builder.set_min_proto_version(Some(SslVersion::TLS1)).map_err(|error| RdpProtocolError::protocol(format!("failed to set TLS min version: {}", error)))?;
    builder.set_security_level(0);
    builder.set_mode(SslMode::ACCEPT_MOVING_WRITE_BUFFER | SslMode::ENABLE_PARTIAL_WRITE);
    builder.set_options(SslOptions::ALL | SslOptions::NO_COMPRESSION | SslOptions::DONT_INSERT_EMPTY_FRAGMENTS);
    if ignore_cert {
        builder.set_verify(SslVerifyMode::NONE);
    } else {
        let captured = Arc::clone(&fingerprint);
        let trusted = trusted_sha256.map(|value| value.to_string());
        builder.set_verify_callback(SslVerifyMode::PEER, move |preverify_ok, ctx| {
            let Some(cert) = ctx.current_cert() else {
                return preverify_ok;
            };
            if ctx.error_depth() != 0 {
                return preverify_ok;
            }
            let Ok(der) = cert.to_der() else {
                return preverify_ok;
            };
            let value = certificate::fingerprint_sha256(&der);
            let trusted_match = trusted.as_deref() == Some(value.as_str());
            *captured.lock() = Some(value);
            preverify_ok || trusted_match
        });
    }
    let mut ssl = Ssl::new(&builder.build()).map_err(|error| RdpProtocolError::protocol(format!("failed to build TLS session: {}", error)))?;
    if IpAddr::from_str(host).is_err() {
        ssl.set_hostname(host).map_err(|error| RdpProtocolError::protocol(format!("invalid server name [{}]: {}", host, error)))?;
    }
    let mut stream = SslStream::new(ssl, stream).map_err(|error| RdpProtocolError::protocol(format!("failed to build TLS stream: {}", error)))?;
    match Pin::new(&mut stream).connect().await {
        Ok(()) => {
            let cert = stream.ssl().peer_certificate().ok_or_else(|| RdpProtocolError::protocol("server did not present a TLS certificate".to_string()))?;
            let leaf = cert.to_der().map_err(|error| RdpProtocolError::protocol(format!("failed to encode TLS certificate: {}", error)))?;
            let public_key = certificate::extract_public_key_pkcs1(&leaf)?;
            let fingerprint = certificate::fingerprint_sha256(&leaf);
            Ok(TlsOutcome::Established(TlsUpgrade { stream, public_key, fingerprint }))
        }
        Err(error) => {
            if !ignore_cert && stream.ssl().verify_result() != X509VerifyResult::OK {
                if let Some(captured) = fingerprint.lock().clone() {
                    return Ok(TlsOutcome::Untrusted { fingerprint: captured });
                }
            }
            Err(RdpProtocolError::protocol(format!("TLS handshake failed: {}", error)))
        }
    }
}
