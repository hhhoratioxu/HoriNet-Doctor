//! Small, bounded UDP STUN codec and transaction layer (RFC 8489 / RFC 5780).
//! No authentication, redirects, RESPONSE-PORT, TCP, or legacy RFC 3489 discovery.
use crate::{bounded, Error, Result};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};
use tokio::{net::UdpSocket, time::Instant};
use tokio_util::sync::CancellationToken;

const COOKIE: [u8; 4] = [0x21, 0x12, 0xa4, 0x42];
const MAX_PACKET: usize = 2048;

#[derive(Debug, Clone)]
pub(super) struct Binding {
    pub mapped: SocketAddr,
    pub origin: Option<SocketAddr>,
    pub other: Option<SocketAddr>,
    pub xor_mapped: bool,
}
#[derive(Debug)]
pub(super) struct Exchange {
    pub binding: Option<Binding>,
    pub source: Option<SocketAddr>,
    pub attempts: u8,
    pub elapsed_ms: u64,
    pub timed_out: bool,
    pub detail: String,
}
enum Message {
    Success(Binding),
    Error(u16),
}

fn invalid() -> Error {
    Error::Network("Malformed or unsupported STUN response".into())
}
fn address(value: &[u8], xor: bool, tx: &[u8; 12]) -> Result<SocketAddr> {
    if value.len() < 4 || value[0] != 0 {
        return Err(invalid());
    }
    let mut port = u16::from_be_bytes([value[2], value[3]]);
    if xor {
        port ^= 0x2112;
    }
    let ip = match (value[1], value.len()) {
        (1, 8) => {
            let mut octets: [u8; 4] = value[4..8].try_into().map_err(|_| invalid())?;
            if xor {
                for (b, mask) in octets.iter_mut().zip(COOKIE) {
                    *b ^= mask;
                }
            }
            IpAddr::V4(Ipv4Addr::from(octets))
        }
        (2, 20) => {
            let mut octets: [u8; 16] = value[4..20].try_into().map_err(|_| invalid())?;
            if xor {
                for (b, mask) in octets.iter_mut().zip(COOKIE.iter().chain(tx)) {
                    *b ^= mask;
                }
            }
            IpAddr::V6(Ipv6Addr::from(octets))
        }
        _ => return Err(invalid()),
    };
    let endpoint = SocketAddr::new(ip, port);
    crate::validation::endpoint(&endpoint.to_string()).map_err(|_| invalid())?;
    Ok(endpoint)
}
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}
fn decode(data: &[u8], tx: &[u8; 12]) -> Result<Message> {
    if data.len() < 20 || data.len() > MAX_PACKET || data[4..8] != COOKIE || data[8..20] != *tx {
        return Err(invalid());
    }
    let length = usize::from(u16::from_be_bytes([data[2], data[3]]));
    let kind = u16::from_be_bytes([data[0], data[1]]);
    if length % 4 != 0 || data.len() != length + 20 || !matches!(kind, 0x0101 | 0x0111) {
        return Err(invalid());
    }
    let (mut mapped, mut xor_mapped, mut origin, mut other, mut error) =
        (None, None, None, None, None);
    let mut pos = 20;
    let mut count = 0;
    while pos < data.len() {
        count += 1;
        if count > 64 || pos + 4 > data.len() {
            return Err(invalid());
        }
        let attr = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let len = usize::from(u16::from_be_bytes([data[pos + 2], data[pos + 3]]));
        let end = pos + 4 + len;
        let padded = pos + 4 + ((len + 3) & !3);
        if end > data.len() || padded > data.len() {
            return Err(invalid());
        }
        let value = &data[pos + 4..end];
        let slot = match attr {
            0x0001 => Some((&mut mapped, false)),
            0x0020 => Some((&mut xor_mapped, true)),
            0x802b => Some((&mut origin, false)),
            0x802c => Some((&mut other, false)),
            _ => None,
        };
        if let Some((slot, xor)) = slot {
            if slot.is_some() {
                return Err(invalid());
            }
            *slot = Some(address(value, xor, tx)?);
        } else {
            match attr {
                0x0009 => {
                    if len < 4
                        || error.is_some()
                        || !(3..=6).contains(&(value[2] & 7))
                        || value[3] > 99
                    {
                        return Err(invalid());
                    }
                    error = Some(u16::from(value[2] & 7) * 100 + u16::from(value[3]));
                }
                0x8028 => {
                    if len != 4 || padded != data.len() {
                        return Err(invalid());
                    }
                    let fp = u32::from_be_bytes(value.try_into().map_err(|_| invalid())?);
                    if fp != (crc32(&data[..pos]) ^ 0x5354_554e) {
                        return Err(invalid());
                    }
                }
                // Authenticated responses cannot be verified by this anonymous client.
                0x0008 | 0x001c => {
                    return Err(Error::Network(
                        "STUN authentication is not supported".into(),
                    ))
                }
                0x000a => {} // UNKNOWN-ATTRIBUTES, useful with a 420 error.
                v if v < 0x8000 => return Err(invalid()),
                _ => {} // Unknown comprehension-optional attributes are safely skipped.
            }
        }
        pos = padded;
    }
    if kind == 0x0111 {
        return error.map(Message::Error).ok_or_else(invalid);
    }
    if error.is_some() {
        return Err(invalid());
    }
    Ok(Message::Success(Binding {
        mapped: xor_mapped.or(mapped).ok_or_else(invalid)?,
        xor_mapped: xor_mapped.is_some(),
        origin,
        other,
    }))
}
fn request(tx: &[u8; 12], change: u32) -> Vec<u8> {
    let mut data = vec![0, 1, 0, if change == 0 { 0 } else { 8 }];
    data.extend(COOKIE);
    data.extend(tx);
    if change != 0 {
        data.extend([0, 3, 0, 4]);
        data.extend(change.to_be_bytes());
    }
    data
}

pub(super) async fn binding(
    socket: &UdpSocket,
    target: SocketAddr,
    expected: SocketAddr,
    change: u32,
    timeout: Duration,
    token: &CancellationToken,
) -> Result<Exchange> {
    let uuid = uuid::Uuid::new_v4();
    // Skip UUID's version/variant bytes: all 96 transaction ID bits are random.
    let mut tx = [0u8; 12];
    tx[..6].copy_from_slice(&uuid.as_bytes()[..6]);
    tx[6..].copy_from_slice(&uuid.as_bytes()[9..15]);
    let data = request(&tx, change);
    let start = Instant::now();
    let deadline = start + timeout;
    let mut attempts = 0;
    let mut packet = [0u8; MAX_PACKET + 1];
    let mut received = 0;
    while attempts < 3 {
        bounded(
            token,
            deadline.saturating_duration_since(Instant::now()),
            async {
                socket.send_to(&data, target).await?;
                Ok(())
            },
        )
        .await?;
        attempts += 1;
        let wait_until = if attempts < 3 {
            (Instant::now() + Duration::from_millis(500 << (attempts - 1))).min(deadline)
        } else {
            deadline
        };
        loop {
            let incoming = bounded(
                token,
                wait_until.saturating_duration_since(Instant::now()),
                async { Ok(socket.recv_from(&mut packet).await?) },
            )
            .await;
            let (len, source) = match incoming {
                Ok(v) => v,
                Err(Error::Timeout) => break,
                Err(e) => return Err(e),
            };
            received += 1;
            if received > 64 {
                return Err(Error::Network(
                    "Too many unrelated UDP packets during STUN probe".into(),
                ));
            }
            if source != expected && source != target {
                continue;
            }
            if len < 20 || packet[8..20] != tx {
                continue;
            }
            let mut exchange = Exchange {
                binding: None,
                source: Some(source),
                attempts,
                elapsed_ms: start.elapsed().as_millis() as u64,
                timed_out: false,
                detail: String::new(),
            };
            match decode(&packet[..len], &tx) {
                Ok(Message::Error(code)) => {
                    exchange.detail = format!("STUN server returned error {code}; no NAT inference")
                }
                Ok(Message::Success(reply)) => {
                    if source != expected {
                        exchange.detail = "Response source did not honor CHANGE-REQUEST; filtering remains unknown".into();
                    } else if reply.origin.is_some_and(|origin| origin != source) {
                        exchange.detail = "RESPONSE-ORIGIN differs from UDP source; path or server may rewrite STUN".into();
                    } else if !reply.mapped.is_ipv4() {
                        exchange.detail =
                            "IPv4 UDP test received a different mapped address family".into();
                    } else {
                        exchange.detail = "Matching STUN Binding response received".into();
                        exchange.binding = Some(reply);
                    }
                }
                Err(e) => exchange.detail = e.to_string(),
            }
            return Ok(exchange);
        }
        if Instant::now() >= deadline {
            break;
        }
    }
    Ok(Exchange { binding: None, source: None, attempts, elapsed_ms: start.elapsed().as_millis() as u64,
        timed_out: true, detail: "No matching STUN response within the probe timeout; server, UDP path or filtering may be responsible".into() })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn response(tx: &[u8; 12], attrs: &[u8]) -> Vec<u8> {
        let mut data = vec![1, 1];
        data.extend((attrs.len() as u16).to_be_bytes());
        data.extend(COOKIE);
        data.extend(tx);
        data.extend(attrs);
        data
    }
    #[test]
    fn xor_ipv4_ipv6_and_transaction_validation() {
        let tx = [7u8; 12];
        let mut value = vec![0, 1];
        value.extend((54321u16 ^ 0x2112).to_be_bytes());
        value.extend([203, 0, 113, 9].into_iter().zip(COOKIE).map(|(a, b)| a ^ b));
        let mut attrs = vec![0, 0x20, 0, 8];
        attrs.extend(value);
        let data = response(&tx, &attrs);
        let Message::Success(b) = decode(&data, &tx).unwrap() else {
            panic!("success expected")
        };
        assert_eq!(b.mapped.to_string(), "203.0.113.9:54321");
        assert!(decode(&data, &[8; 12]).is_err());
        let ip: Ipv6Addr = "2001:db8::1234".parse().unwrap();
        let mut value = vec![0, 2];
        value.extend((1234u16 ^ 0x2112).to_be_bytes());
        value.extend(
            ip.octets()
                .iter()
                .zip(COOKIE.iter().chain(tx.iter()))
                .map(|(a, b)| a ^ b),
        );
        assert_eq!(
            address(&value, true, &tx).unwrap().to_string(),
            "[2001:db8::1234]:1234"
        );
    }
    #[test]
    fn malformed_lengths_cookie_attributes_and_fingerprint_are_rejected() {
        let tx = [3; 12];
        for len in 0..20 {
            assert!(decode(&[0; 20][..len], &tx).is_err());
        }
        assert!(decode(&response(&tx, &[0, 0x20, 0, 255]), &tx).is_err());
        assert!(decode(&response(&tx, &[0, 0x20, 0, 4, 0, 1, 0, 1]), &tx).is_err());
        assert!(decode(&response(&tx, &[0x80, 0x28, 0, 4, 0, 0, 0, 0]), &tx).is_err());
        let mut packet = response(&tx, &[0, 1, 0, 8, 0, 1, 0, 1, 127, 0, 0, 1]);
        let mut duplicate = packet[20..].to_vec();
        duplicate.extend_from_within(..);
        assert!(decode(&response(&tx, &duplicate), &tx).is_err());
        packet[4] ^= 1;
        assert!(decode(&packet, &tx).is_err());
        packet[4] ^= 1;
        packet.push(0);
        assert!(decode(&packet, &tx).is_err());
        // Reject every truncation of a valid packet without panicking.
        for len in 0..packet.len() - 1 {
            assert!(decode(&packet[..len], &tx).is_err());
        }
    }
    #[test]
    fn error_420_and_padded_optional_attributes() {
        let tx = [4; 12];
        let mut data = response(&tx, &[0, 9, 0, 4, 0, 0, 4, 20]);
        data[1] = 0x11;
        assert!(matches!(decode(&data, &tx).unwrap(), Message::Error(420)));
        let attrs = [
            0x80, 0x22, 0, 1, b'x', 0, 0, 0, 0, 1, 0, 8, 0, 1, 0, 1, 127, 0, 0, 1,
        ];
        assert!(matches!(
            decode(&response(&tx, &attrs), &tx).unwrap(),
            Message::Success(_)
        ));
    }
}
