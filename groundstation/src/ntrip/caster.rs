use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;

use base64::Engine;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct};

use super::{CONNECT_TIMEOUT, Config, READ_SLICE, SOURCE_TABLE_MAX_BYTES, SOURCE_TABLE_TIMEOUT, Stream};

pub(super) fn download_source_table(config: &Config) -> Result<String, String> {
    if config.use_tls && config.allow_self_signed {
        open(config)?;
    }
    let tls = ureq::tls::TlsConfig::builder().disable_verification(config.allow_self_signed).build();
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(SOURCE_TABLE_TIMEOUT)).tls_config(tls).build().into();
    let url = format!("{}://{}:{}/", if config.use_tls { "https" } else { "http" }, config.host, config.port);
    let request = agent.get(&url).header("Ntrip-Version", "Ntrip/2.0").header("User-Agent", "QGC-NTRIP");
    let request = match config.username.is_empty() && config.password.is_empty() {
        true => request,
        false => request.header("Authorization", format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", config.username, config.password)))),
    };
    let mut response = request.call().map_err(|e| e.to_string())?;
    let body = response.body_mut().with_config().limit(SOURCE_TABLE_MAX_BYTES).read_to_string().map_err(|e| match e {
        ureq::Error::BodyExceedsLimit(_) => format!("Source table too large (exceeds {} MB)", SOURCE_TABLE_MAX_BYTES / (1024 * 1024)),
        other => other.to_string(),
    })?;
    match body.contains("ENDSOURCETABLE") {
        true => Ok(body),
        false => Err("Response does not contain a valid source table".to_string()),
    }
}

const SELF_SIGNED_REJECTED: &str = "Self-signed certificate rejected. Enable 'Accept self-signed certificates' in NTRIP settings to allow.";

type Verified = Result<ServerCertVerified, rustls::Error>;

fn bad_certificate(error: CertificateError) -> rustls::Error {
    rustls::Error::InvalidCertificate(error)
}

fn is_self_signed(der: &[u8]) -> bool {
    x509_parser::parse_x509_certificate(der).is_ok_and(|(_, cert)| cert.subject() == cert.issuer() && cert.verify_signature(None).is_ok())
}

fn self_signed_leaf(der: &CertificateDer<'_>, server_name: &ServerName<'_>, now: UnixTime) -> Verified {
    let (_, cert) = x509_parser::parse_x509_certificate(der).map_err(|_| bad_certificate(CertificateError::BadEncoding))?;
    let at = x509_parser::time::ASN1Time::from_timestamp(i64::try_from(now.as_secs()).unwrap_or(i64::MAX)).map_err(|_| bad_certificate(CertificateError::BadEncoding))?;
    let validity = cert.validity();
    match (at < validity.not_before, at > validity.not_after) {
        (true, _) => Err(bad_certificate(CertificateError::NotValidYet)),
        (_, true) => Err(bad_certificate(CertificateError::Expired)),
        _ => webpki::EndEntityCert::try_from(der)
            .and_then(|leaf| leaf.verify_is_valid_for_subject_name(server_name))
            .map(|()| ServerCertVerified::assertion())
            .map_err(|_| bad_certificate(CertificateError::NotValidForName)),
    }
}

#[derive(Debug)]
struct CasterCertificate {
    trusted: Arc<rustls::client::WebPkiServerVerifier>,
    provider: Arc<rustls::crypto::CryptoProvider>,
    allow_self_signed: bool,
}

impl CasterCertificate {
    fn verifier(provider: &Arc<rustls::crypto::CryptoProvider>, roots: Vec<rustls::pki_types::TrustAnchor<'static>>) -> Result<Arc<rustls::client::WebPkiServerVerifier>, rustls::Error> {
        rustls::client::WebPkiServerVerifier::builder_with_provider(Arc::new(rustls::RootCertStore { roots }), provider.clone()).build().map_err(|e| rustls::Error::General(e.to_string()))
    }

    fn new(provider: Arc<rustls::crypto::CryptoProvider>, allow_self_signed: bool) -> Result<Self, rustls::Error> {
        Ok(Self { trusted: Self::verifier(&provider, webpki_roots::TLS_SERVER_ROOTS.to_vec())?, provider, allow_self_signed })
    }

    fn self_signed_chain(&self, end_entity: &CertificateDer<'_>, intermediates: &[CertificateDer<'_>], server_name: &ServerName<'_>, ocsp: &[u8], now: UnixTime, refused: rustls::Error) -> Verified {
        let chain: Vec<&CertificateDer<'_>> = std::iter::once(end_entity).chain(intermediates).collect();
        let checked = match chain.iter().position(|cert| is_self_signed(cert)) {
            None => return Err(refused),
            Some(0) => self_signed_leaf(end_entity, server_name, now),
            Some(root) => webpki::anchor_from_trusted_cert(chain[root])
                .map_err(|_| bad_certificate(CertificateError::BadEncoding))
                .and_then(|anchor| Self::verifier(&self.provider, vec![anchor.to_owned()]))
                .and_then(|verifier| verifier.verify_server_cert(end_entity, intermediates, server_name, ocsp, now)),
        }?;
        match self.allow_self_signed {
            true => Ok(checked),
            false => Err(rustls::Error::General(SELF_SIGNED_REJECTED.to_string())),
        }
    }
}

impl ServerCertVerifier for CasterCertificate {
    fn verify_server_cert(&self, end_entity: &CertificateDer<'_>, intermediates: &[CertificateDer<'_>], server_name: &ServerName<'_>, ocsp: &[u8], now: UnixTime) -> Verified {
        self.trusted.verify_server_cert(end_entity, intermediates, server_name, ocsp, now).or_else(|refused| self.self_signed_chain(end_entity, intermediates, server_name, ocsp, now, refused))
    }

    fn verify_tls12_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.trusted.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.trusted.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.trusted.supported_verify_schemes()
    }
}

fn tls_failure(error: std::io::Error) -> String {
    match error.get_ref().and_then(|inner| inner.downcast_ref::<rustls::Error>()) {
        Some(rustls::Error::General(reason)) => reason.clone(),
        _ => error.to_string(),
    }
}

fn tls(config: &Config, mut tcp: TcpStream) -> Result<Box<dyn Stream>, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = CasterCertificate::new(provider.clone(), config.allow_self_signed).map_err(|e| e.to_string())?;
    let client = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    let name = ServerName::try_from(config.host.clone()).map_err(|e| e.to_string())?;
    let mut connection = rustls::ClientConnection::new(Arc::new(client), name).map_err(|e| e.to_string())?;
    tcp.set_read_timeout(Some(CONNECT_TIMEOUT)).map_err(|e| e.to_string())?;
    while connection.is_handshaking() {
        connection.complete_io(&mut tcp).map_err(tls_failure)?;
    }
    tcp.set_read_timeout(Some(READ_SLICE)).map_err(|e| e.to_string())?;
    Ok(Box::new(rustls::StreamOwned::new(connection, tcp)))
}

pub(super) fn open(config: &Config) -> Result<Box<dyn Stream>, String> {
    let address = (config.host.as_str(), config.port).to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or_else(|| format!("{} does not resolve", config.host))?;
    let tcp = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).map_err(|e| if e.kind() == std::io::ErrorKind::TimedOut { "Connection timeout".to_string() } else { e.to_string() })?;
    tcp.set_read_timeout(Some(READ_SLICE)).map_err(|e| e.to_string())?;
    match config.use_tls {
        true => tls(config, tcp),
        false => Ok(Box::new(tcp)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const SELF_SIGNED: &[u8] = include_bytes!("../../tests/fixtures/ntrip-tls/self-signed.der");
    const PRIVATE_ROOT: &[u8] = include_bytes!("../../tests/fixtures/ntrip-tls/private-root.der");
    const PRIVATE_LEAF: &[u8] = include_bytes!("../../tests/fixtures/ntrip-tls/private-leaf.der");
    const JANUARY_2027: u64 = 1_800_000_000;
    const YEAR_2200: u64 = 7_258_118_400;

    fn caster_certificate(allow_self_signed: bool, chain: &[&'static [u8]], host: &str, now: u64) -> Result<(), rustls::Error> {
        let verifier = CasterCertificate::new(Arc::new(rustls::crypto::ring::default_provider()), allow_self_signed).unwrap();
        let certs: Vec<CertificateDer<'static>> = chain.iter().map(|der| CertificateDer::from(*der)).collect();
        let name = ServerName::try_from(host.to_string()).unwrap();
        verifier.verify_server_cert(&certs[0], &certs[1..], &name, &[], UnixTime::since_unix_epoch(Duration::from_secs(now))).map(|_| ())
    }

    #[test]
    fn opting_in_forgives_only_the_self_signed_error_like_ntrip_http_transport() {
        assert_eq!(caster_certificate(true, &[SELF_SIGNED], "caster.test", JANUARY_2027), Ok(()), "a self-signed CA:TRUE leaf is accepted once opted in");
        assert_eq!(caster_certificate(true, &[PRIVATE_LEAF, PRIVATE_ROOT], "caster.test", JANUARY_2027), Ok(()), "SelfSignedCertificateInChain is ignorable too");
        assert_eq!(caster_certificate(true, &[SELF_SIGNED], "other.test", JANUARY_2027), Err(bad_certificate(CertificateError::NotValidForName)), "a host name mismatch stays fatal");
        assert_eq!(caster_certificate(true, &[SELF_SIGNED], "caster.test", YEAR_2200), Err(bad_certificate(CertificateError::Expired)), "expiry stays fatal");
        assert!(caster_certificate(true, &[PRIVATE_LEAF, PRIVATE_ROOT], "other.test", JANUARY_2027).is_err());
        assert_eq!(caster_certificate(true, &[PRIVATE_LEAF], "caster.test", JANUARY_2027), Err(bad_certificate(CertificateError::UnknownIssuer)), "an issuer the caster did not send is not self-signed");
    }

    #[test]
    fn without_the_opt_in_a_self_signed_caster_is_refused_with_qgcs_hint() {
        let rejected = Err(rustls::Error::General(SELF_SIGNED_REJECTED.to_string()));
        assert_eq!(caster_certificate(false, &[SELF_SIGNED], "caster.test", JANUARY_2027), rejected);
        assert_eq!(caster_certificate(false, &[PRIVATE_LEAF, PRIVATE_ROOT], "caster.test", JANUARY_2027), rejected);
        assert_eq!(caster_certificate(false, &[SELF_SIGNED], "other.test", JANUARY_2027), Err(bad_certificate(CertificateError::NotValidForName)), "other faults win over the hint, as Qt reports them fatal first");
        assert_eq!(tls_failure(std::io::Error::new(std::io::ErrorKind::InvalidData, rustls::Error::General(SELF_SIGNED_REJECTED.to_string()))), SELF_SIGNED_REJECTED);
    }
}
