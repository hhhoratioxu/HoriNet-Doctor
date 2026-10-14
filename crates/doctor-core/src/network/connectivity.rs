use crate::{bounded, validation, Error, Result};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{io::AsyncReadExt, net::TcpStream};
use tokio_util::sync::CancellationToken;

pub async fn resolve(
    host: &str,
    port: u16,
    timeout: Duration,
    token: &CancellationToken,
) -> Result<Vec<SocketAddr>> {
    validation::host(host)?;
    bounded(token, timeout, async {
        let mut addresses: Vec<_> = tokio::net::lookup_host((host, port))
            .await?
            .take(32)
            .collect();
        addresses.sort();
        addresses.dedup();
        if addresses.is_empty() {
            return Err(Error::Network(
                "system resolver returned no addresses".into(),
            ));
        }
        Ok(addresses)
    })
    .await
}
pub async fn tcp(addr: SocketAddr, timeout: Duration, token: &CancellationToken) -> Result<f64> {
    validation::endpoint(&addr.to_string())?;
    bounded(token, timeout, async {
        let start = Instant::now();
        let _stream = TcpStream::connect(addr).await?;
        Ok(start.elapsed().as_secs_f64() * 1000.0)
    })
    .await
}
pub async fn tls(url: &str, timeout: Duration, token: &CancellationToken) -> Result<String> {
    let url = validation::web_url(url, true)?;
    let host = url
        .host_str()
        .ok_or_else(|| Error::Invalid("missing host".into()))?
        .trim_matches(['[', ']'])
        .to_owned();
    bounded(token, timeout, async {
        let roots = tokio_rustls::rustls::RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        );
        let config = tokio_rustls::rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let stream =
            TcpStream::connect((host.as_str(), url.port_or_known_default().unwrap_or(443))).await?;
        let name = tokio_rustls::rustls::pki_types::ServerName::try_from(host.clone())
            .map_err(|_| Error::Invalid("invalid TLS server name".into()))?;
        let stream = tokio_rustls::TlsConnector::from(Arc::new(config))
            .connect(name, stream)
            .await
            .map_err(|e| Error::Network(format!("TLS handshake / certificate: {e}")))?;
        Ok(format!(
            "Certificate verified; {:?}; {:?}",
            stream.get_ref().1.protocol_version(),
            stream.get_ref().1.negotiated_cipher_suite()
        ))
    })
    .await
}
#[derive(Debug)]
pub struct HttpResult {
    pub status: u16,
    pub redirect: bool,
    pub expected_body: Option<bool>,
    pub detail: String,
}
pub fn client(timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .user_agent("HoriNet-Doctor/0.1.0-beta.1")
        .build()
        .map_err(|e| Error::Network(e.to_string()))
}
pub fn request_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        return Error::Timeout;
    }
    use std::error::Error as _;
    let error = error.without_url();
    let kind = if error.is_connect() {
        "Connection / TLS / bootstrap DNS failure"
    } else if error.is_body() {
        "Response body failure"
    } else {
        "HTTP transport failure"
    };
    let mut messages = vec![error.to_string()];
    let mut source = error.source();
    for _ in 0..4 {
        let Some(e) = source else {
            break;
        };
        messages.push(e.to_string());
        source = e.source();
    }
    Error::Network(format!("{kind}: {}", messages.join("; ")))
}
pub async fn http(url: &str, timeout: Duration, token: &CancellationToken) -> Result<HttpResult> {
    let parsed = validation::web_url(url, false)?;
    bounded(token, timeout, async {
        let mut response = client(timeout)?
            .get(parsed.clone())
            .send()
            .await
            .map_err(request_error)?;
        let status = response.status().as_u16();
        let redirect = response.status().is_redirection();
        let expected_body = if parsed.as_str() == "http://www.msftconnecttest.com/connecttest.txt"
            && status == 200
        {
            // Bound downloaded data, including chunked bodies; never follow a redirect.
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(request_error)? {
                if body.len() + chunk.len() > 4096 {
                    return Ok(HttpResult {
                        status,
                        redirect,
                        expected_body: Some(false),
                        detail: "Connectivity response exceeds 4096 bytes".into(),
                    });
                }
                body.extend_from_slice(&chunk);
            }
            Some(body == b"Microsoft Connect Test")
        } else {
            None
        };
        Ok(HttpResult {
            status,
            redirect,
            expected_body,
            detail: format!("HTTP {status}; direct connection; redirects disabled"),
        })
    })
    .await
}
pub async fn read_headers(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while bytes.len() < 8192 {
        let b = stream.read_u8().await?;
        bytes.push(b);
        if bytes.ends_with(b"\r\n\r\n") {
            return Ok(bytes);
        }
    }
    Err(Error::Network("proxy headers exceed 8192 bytes".into()))
}
pub fn usable_ip(ip: IpAddr) -> bool {
    !ip.is_unspecified() && !ip.is_multicast() && !ip.is_loopback()
}
