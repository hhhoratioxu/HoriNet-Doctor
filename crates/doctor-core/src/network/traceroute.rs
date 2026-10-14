use crate::{
    models::Hop,
    network::{connectivity, icmp},
    Error, Result,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub async fn trace(
    target: &str,
    ipv6: bool,
    max_hops: u8,
    timeout: Duration,
    token: &CancellationToken,
    emit: impl Fn(Hop),
) -> Result<Vec<Hop>> {
    if !(1..=30).contains(&max_hops) {
        return Err(Error::Invalid("max hops must be 1..30".into()));
    }
    let ip = connectivity::resolve(target, 443, timeout, token)
        .await?
        .into_iter()
        .find(|a| a.is_ipv6() == ipv6)
        .ok_or_else(|| Error::Network("target has no address in selected family".into()))?
        .ip();
    let mut hops = Vec::new();
    for ttl in 1..=max_hops {
        if token.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let reply = icmp::ping(ip, ttl, timeout, token).await?;
        let reached = reply.reached;
        let hop = Hop { ttl, reply };
        emit(hop.clone());
        hops.push(hop);
        if reached {
            break;
        }
        tokio::select! { _ = token.cancelled() => return Err(Error::Cancelled), _ = tokio::time::sleep(Duration::from_millis(100)) => {} }
    }
    Ok(hops)
}
