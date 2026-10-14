//! Small bounded DNS wire codec for A/AAAA tests. No third-party query API.
use crate::{bounded, models::DnsResult, network::connectivity, validation, Error, Result};
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
};
use tokio_util::sync::CancellationToken;

pub fn query_packet(domain: &str, ipv6: bool, id: u16) -> Result<Vec<u8>> {
    let domain = validation::host(domain)?;
    if domain.parse::<std::net::IpAddr>().is_ok() {
        return Err(Error::Invalid("DNS query requires a domain".into()));
    }
    let mut out = Vec::from(id.to_be_bytes());
    out.extend_from_slice(&[1, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
    for label in domain.trim_end_matches('.').split('.') {
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.extend_from_slice(&[0, 0, if ipv6 { 28 } else { 1 }, 0, 1]);
    Ok(out)
}
fn u16_at(data: &[u8], pos: usize) -> Result<u16> {
    let b = data
        .get(pos..pos + 2)
        .ok_or_else(|| Error::Network("truncated DNS packet".into()))?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}
fn name(data: &[u8], cursor: &mut usize) -> Result<String> {
    let mut pos = *cursor;
    let mut end = None;
    let mut labels = Vec::new();
    for _ in 0..128 {
        let len = *data
            .get(pos)
            .ok_or_else(|| Error::Network("invalid DNS name".into()))?;
        if len & 0xc0 == 0xc0 {
            let offset = (u16_at(data, pos)? & 0x3fff) as usize;
            end.get_or_insert(pos + 2);
            if offset >= pos {
                return Err(Error::Network("invalid DNS compression pointer".into()));
            }
            pos = offset;
            continue;
        }
        if len & 0xc0 != 0 {
            return Err(Error::Network("invalid DNS label".into()));
        }
        pos += 1;
        if len == 0 {
            *cursor = end.unwrap_or(pos);
            return Ok(labels.join("."));
        }
        let part = data
            .get(pos..pos + len as usize)
            .ok_or_else(|| Error::Network("DNS label exceeds packet".into()))?;
        labels.push(String::from_utf8_lossy(part).to_ascii_lowercase());
        pos += len as usize;
        if labels.iter().map(String::len).sum::<usize>() > 253 {
            break;
        }
    }
    Err(Error::Network("DNS name exceeds bounds".into()))
}
pub fn parse_response(data: &[u8], request: &[u8]) -> Result<(Vec<String>, bool)> {
    if data.len() < 12
        || request.len() < 12
        || data[0..2] != request[0..2]
        || data[2] & 0x80 == 0
        || data[2] & 0x78 != 0
    {
        return Err(Error::Network("DNS response ID / header mismatch".into()));
    }
    if u16_at(data, 4)? != 1 {
        return Err(Error::Network("DNS question count mismatch".into()));
    }
    let mut cursor = 12;
    let qname = name(data, &mut cursor)?;
    let mut expected = 12;
    let expected_name = name(request, &mut expected)?;
    let qtype = u16_at(data, cursor)?;
    if qname != expected_name
        || qtype != u16_at(request, expected)?
        || u16_at(data, cursor + 2)? != 1
    {
        return Err(Error::Network("DNS question mismatch".into()));
    }
    cursor += 4;
    let rcode = data[3] & 15;
    if rcode != 0 {
        return Err(Error::Network(
            match rcode {
                1 => "FORMERR",
                2 => "SERVFAIL",
                3 => "NXDOMAIN",
                5 => "REFUSED",
                _ => "DNS error",
            }
            .into(),
        ));
    }
    let truncated = data[2] & 2 != 0;
    if truncated {
        return Ok((vec![], true));
    }
    let mut records = Vec::new();
    for _ in 0..u16_at(data, 6)?.min(256) {
        let _owner = name(data, &mut cursor)?;
        let typ = u16_at(data, cursor)?;
        let class = u16_at(data, cursor + 2)?;
        let len = u16_at(data, cursor + 8)? as usize;
        cursor += 10;
        let bytes = data
            .get(cursor..cursor + len)
            .ok_or_else(|| Error::Network("DNS RDATA exceeds packet".into()))?;
        if class == 1 && typ == qtype {
            if typ == 1 && len == 4 {
                records.push(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]).to_string());
            }
            if typ == 28 && len == 16 {
                let mut ip = [0; 16];
                ip.copy_from_slice(bytes);
                records.push(Ipv6Addr::from(ip).to_string());
            }
        }
        cursor += len;
    }
    records.sort();
    records.dedup();
    Ok((records, false))
}
pub async fn query(
    domain: &str,
    server: &str,
    transport: &str,
    ipv6: bool,
    timeout: Duration,
    token: &CancellationToken,
) -> Result<DnsResult> {
    let request = query_packet(
        domain,
        ipv6,
        u16::from_be_bytes([
            uuid::Uuid::new_v4().as_bytes()[0],
            uuid::Uuid::new_v4().as_bytes()[1],
        ]),
    )?;
    if !matches!(transport, "udp" | "tcp" | "doh") {
        return Err(Error::Invalid("transport must be udp, tcp or doh".into()));
    }
    let endpoint = if transport != "doh" {
        Some(validation::endpoint(server)?)
    } else {
        validation::web_url(server, true)?;
        None
    };
    let start = Instant::now();
    let result = bounded(token, timeout, async {
        let response = match transport {
            "udp" => {
                let addr = endpoint.ok_or_else(|| Error::Invalid("missing DNS endpoint".into()))?;
                let local: SocketAddr = if addr.is_ipv6() {
                    "[::]:0"
                } else {
                    "0.0.0.0:0"
                }
                .parse()
                .map_err(|_| Error::Invalid("internal bind".into()))?;
                let socket = UdpSocket::bind(local).await?;
                socket.connect(addr).await?;
                socket.send(&request).await?;
                let mut data = vec![0u8; 65535];
                let count = socket.recv(&mut data).await?;
                data.truncate(count);
                data
            }
            "tcp" => {
                let mut stream = TcpStream::connect(
                    endpoint.ok_or_else(|| Error::Invalid("missing DNS endpoint".into()))?,
                )
                .await?;
                stream.write_u16(request.len() as u16).await?;
                stream.write_all(&request).await?;
                let len = stream.read_u16().await? as usize;
                let mut data = vec![0; len];
                stream.read_exact(&mut data).await?;
                data
            }
            _ => {
                let mut response = connectivity::client(timeout)?
                    .post(server)
                    .header("content-type", "application/dns-message")
                    .header("accept", "application/dns-message")
                    .body(request.clone())
                    .send()
                    .await
                    .map_err(connectivity::request_error)?;
                if !response.status().is_success() {
                    return Err(Error::Network(format!("DoH HTTP {}", response.status())));
                }
                if !response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.starts_with("application/dns-message"))
                {
                    return Err(Error::Network(
                        "DoH content type is not DNS wire format".into(),
                    ));
                }
                let mut data = Vec::new();
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(connectivity::request_error)?
                {
                    if data.len() + chunk.len() > 65535 {
                        return Err(Error::Network("DoH response exceeds 65535 bytes".into()));
                    }
                    data.extend_from_slice(&chunk);
                }
                data
            }
        };
        parse_response(&response, &request)
    })
    .await;
    if matches!(result, Err(Error::Cancelled)) {
        return Err(Error::Cancelled);
    }
    let (records, truncated, error) = match result {
        Ok((r, t)) => (r, t, None),
        Err(e) => (vec![], false, Some(e.to_string())),
    };
    Ok(DnsResult {
        server: server.into(),
        transport: transport.into(),
        record_type: if ipv6 { "AAAA" } else { "A" }.into(),
        records,
        elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
        error,
        truncated,
    })
}
