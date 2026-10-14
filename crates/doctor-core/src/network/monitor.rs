use crate::{
    models::{PingReply, Sample},
    network::{connectivity, icmp},
    Error, Result,
};
use std::time::{Duration, Instant};
use tokio::{sync::watch, time::MissedTickBehavior};
use tokio_util::sync::CancellationToken;

pub async fn monitor(
    target: &str,
    port: u16,
    interval: Duration,
    timeout: Duration,
    mut pause: watch::Receiver<bool>,
    token: &CancellationToken,
    emit: impl Fn(Sample),
) -> Result<()> {
    if port == 0 || !(1000..=60000).contains(&interval.as_millis()) {
        return Err(Error::Invalid(
            "port and sampling interval outside bounds".into(),
        ));
    }
    let addr = connectivity::resolve(target, port, timeout, token)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| Error::Network("no target addresses".into()))?;
    crate::validation::endpoint(&addr.to_string())?;
    let started = Instant::now();
    let mut tick = tokio::time::interval(interval);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    for sequence in 1..=10000 {
        while *pause.borrow_and_update() {
            tokio::select! { _ = token.cancelled() => return Err(Error::Cancelled), r = pause.changed() => if r.is_err() { return Err(Error::Cancelled); } }
        }
        tokio::select! { _ = token.cancelled() => return Err(Error::Cancelled), _ = tick.tick() => {} }
        if *pause.borrow() {
            continue;
        }
        let (icmp, tcp) = tokio::join!(
            icmp::ping(addr.ip(), 64, timeout, token),
            connectivity::tcp(addr, timeout, token)
        );
        if token.is_cancelled() {
            return Err(Error::Cancelled);
        }
        emit(Sample {
            sequence,
            elapsed_ms: started.elapsed().as_millis() as u64,
            icmp: icmp.unwrap_or_else(|e| PingReply {
                address: None,
                rtt_ms: None,
                status_code: u32::MAX,
                reached: false,
                message: e.to_string(),
            }),
            tcp_ms: tcp.as_ref().ok().copied(),
            tcp_error: tcp.err().map(|e| e.to_string()),
        });
        if started.elapsed() > Duration::from_secs(86400) {
            break;
        }
    }
    Ok(())
}
