use crate::{
    bounded,
    models::{ProxyTest, Status},
    network::connectivity,
    validation, Error, Result,
};
use std::time::{Duration, Instant};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::sync::CancellationToken;

pub async fn test(
    endpoint: &str,
    protocol: &str,
    target: &str,
    timeout: Duration,
    token: &CancellationToken,
) -> Result<ProxyTest> {
    let addr = validation::endpoint(endpoint)?;
    if !matches!(protocol, "http" | "socks5") {
        return Err(Error::Invalid(
            "proxy protocol must be http or socks5".into(),
        ));
    }
    let url = validation::web_url(target, true)?;
    let host = url
        .host_str()
        .ok_or_else(|| Error::Invalid("missing proxy destination".into()))?
        .trim_matches(['[', ']']);
    validation::host(host)?;
    let port = url.port_or_known_default().unwrap_or(443);
    let mut tcp_ms = None;
    let result = bounded(token, timeout, async {
        let start = Instant::now(); let mut stream = TcpStream::connect(addr).await?;
        tcp_ms = Some(start.elapsed().as_secs_f64() * 1000.0);
        if protocol == "http" {
            let authority = if host.contains(':') { format!("[{host}]:{port}") } else { format!("{host}:{port}") };
            stream.write_all(format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\nUser-Agent: HoriNet-Doctor\r\n\r\n").as_bytes()).await?;
            let headers = connectivity::read_headers(&mut stream).await?;
            let status = String::from_utf8_lossy(&headers).lines().next().unwrap_or("").to_owned();
            if !(status.starts_with("HTTP/1.0 ") || status.starts_with("HTTP/1.1 ")) || !status.split_whitespace().nth(1).is_some_and(|s| s == "200") { return Err(Error::Network(format!("HTTP CONNECT refused: {status}"))); }
        } else {
            stream.write_all(&[5, 1, 0]).await?;
            let mut response = [0;2]; stream.read_exact(&mut response).await?;
            if response != [5,0] { return Err(Error::Network("SOCKS5 requires authentication or rejected greeting; credentials are never read".into())); }
            let mut req = vec![5,1,0,3,host.len() as u8]; req.extend_from_slice(host.as_bytes()); req.extend_from_slice(&port.to_be_bytes()); stream.write_all(&req).await?;
            let mut head = [0;4]; stream.read_exact(&mut head).await?;
            if head[0] != 5 || head[1] != 0 || head[2] != 0 { return Err(Error::Network(format!("SOCKS5 CONNECT error {}", head[1]))); }
            let len = match head[3] { 1 => 4, 4 => 16, 3 => stream.read_u8().await? as usize, _ => return Err(Error::Network("invalid SOCKS5 reply".into())) };
            let mut rest = vec![0;len + 2]; stream.read_exact(&mut rest).await?;
        }
        Ok("Proxy protocol handshake and outbound CONNECT succeeded; target TLS/content not tested through this tunnel".to_string())
    }).await;
    if matches!(result, Err(Error::Cancelled)) {
        return Err(Error::Cancelled);
    }
    Ok(ProxyTest {
        protocol: protocol.into(),
        endpoint: endpoint.into(),
        tcp_ms,
        status: if result.is_ok() {
            Status::Normal
        } else {
            Status::Error
        },
        detail: result.unwrap_or_else(|e| e.to_string()),
    })
}
pub fn configured_endpoint(config: &str) -> Option<(String, u16)> {
    let first = config
        .split(';')
        .find(|s| s.starts_with("http="))
        .or_else(|| config.split(';').next())?;
    let address = first.split_once('=').map_or(first, |(_, v)| v).trim();
    let url = url::Url::parse(&format!("http://{address}")).ok()?;
    if !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    Some((
        url.host_str()?.trim_matches(['[', ']']).into(),
        url.port_or_known_default()?,
    ))
}
