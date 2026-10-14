use crate::{models::PingReply, Error, Result};
use std::{
    net::IpAddr,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub async fn ping(
    ip: IpAddr,
    ttl: u8,
    timeout: Duration,
    token: &CancellationToken,
) -> Result<PingReply> {
    if ip.is_multicast() || ip.is_unspecified() || ip == IpAddr::V4(std::net::Ipv4Addr::BROADCAST) {
        return Err(Error::Invalid("ICMP target must be unicast".into()));
    }
    static LIMIT: OnceLock<Arc<Semaphore>> = OnceLock::new();
    let sem = LIMIT.get_or_init(|| Arc::new(Semaphore::new(2))).clone();
    let permit = tokio::select! { biased; _ = token.cancelled() => return Err(Error::Cancelled), p = sem.acquire_owned() => p.map_err(|_| Error::Cancelled)? };
    let timeout_ms = timeout.as_millis().clamp(100, 10000) as u32;
    let handle = tokio::task::spawn_blocking(move || {
        let _permit = permit; // Held until native call ends, even after cancellation.
        crate::platform::ping(ip, ttl, timeout_ms)
    });
    tokio::select! { biased; _ = token.cancelled() => Err(Error::Cancelled), result = handle => result.map_err(|e| Error::Platform(e.to_string()))? }
}
