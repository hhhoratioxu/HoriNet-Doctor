use crate::{
    models::*,
    network::{connectivity, dns, icmp, proxy},
    rules, validation, Error, Result,
};
use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

pub const TOTAL: usize = 16;
pub async fn inspect(token: &CancellationToken) -> Result<Snapshot> {
    use std::sync::{Arc, OnceLock};
    static LIMIT: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    let sem = LIMIT
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(1)))
        .clone();
    let permit = crate::bounded(token, Duration::from_secs(10), async {
        sem.acquire_owned().await.map_err(|_| Error::Cancelled)
    })
    .await?;
    crate::bounded(token, Duration::from_secs(10), async {
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            crate::platform::snapshot()
        })
        .await
        .map_err(|e| Error::Platform(e.to_string()))?
    })
    .await
}
pub async fn run(
    settings: Settings,
    token: CancellationToken,
    emit: impl Fn(usize, Check),
) -> Result<Report> {
    validation::settings(&settings)?;
    let start = Instant::now();
    let mut report = Report {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        duration_ms: 0,
        cancelled: false,
        snapshot: Snapshot::default(),
        checks: vec![],
        findings: vec![],
        score: None,
        score_basis: String::new(),
    };
    let timeout = Duration::from_millis(settings.timeout_ms);
    match inspect(&token).await {
        Ok(s) => report.snapshot = s,
        Err(Error::Cancelled) => {
            report.cancelled = true;
            finish(&mut report, start);
            return Ok(report);
        }
        Err(e) => report.snapshot.warnings.push(e.to_string()),
    }
    macro_rules! save {
        ($check:expr) => {{
            if token.is_cancelled() {
                report.cancelled = true;
                finish(&mut report, start);
                return Ok(report);
            }
            let check = $check;
            emit(report.checks.len() + 1, check.clone());
            report.checks.push(check);
        }};
    }
    let s = &report.snapshot;
    let active: Vec<_> = s
        .adapters
        .iter()
        .filter(|a| a.up && a.kind != "loopback")
        .collect();
    let unknown = !s.warnings.is_empty() && s.adapters.is_empty();
    save!(Check::new(
        "adapters",
        if unknown {
            Status::Unknown
        } else if active.is_empty() {
            Status::Error
        } else {
            Status::Normal
        },
        format!("{} connected non-loopback adapters", active.len()),
        s.adapters
            .iter()
            .map(|a| format!(
                "{} [{}] up={} virtual={} {} Mbps",
                a.name, a.kind, a.up, a.virtual_adapter, a.link_mbps
            ))
            .chain(s.warnings.clone())
            .collect()
    ));
    let addresses: Vec<_> = active.iter().flat_map(|a| a.addresses.clone()).collect();
    save!(Check::new(
        "addresses",
        if unknown {
            Status::Unknown
        } else if addresses
            .iter()
            .any(|a| a.parse::<IpAddr>().is_ok_and(connectivity::usable_ip))
        {
            Status::Normal
        } else {
            Status::Error
        },
        "Local IPv4 / IPv6 addresses",
        addresses.clone()
    ));
    let dhcp: Vec<_> = active.iter().filter(|a| a.dhcp).collect();
    let bad_dhcp = dhcp.iter().any(|a| {
        !a.addresses.iter().any(|s| {
            s.parse::<std::net::Ipv4Addr>()
                .is_ok_and(|ip| !ip.is_link_local() && !ip.is_unspecified())
        })
    });
    save!(Check::new(
        "dhcp",
        if unknown || dhcp.is_empty() {
            Status::Unknown
        } else if bad_dhcp {
            Status::Error
        } else {
            Status::Normal
        },
        "DHCP flag and usable IPv4 lease evidence (not a lease renewal)",
        dhcp.iter()
            .map(|a| format!("{}: {:?}", a.name, a.addresses))
            .collect()
    ));
    save!(Check::new(
        "routes",
        if unknown {
            Status::Unknown
        } else if s.routes.is_empty() {
            Status::Error
        } else {
            Status::Normal
        },
        format!("{} default routes", s.routes.len()),
        s.routes
            .iter()
            .map(|r| format!(
                "{} if={} via {} metric={}",
                r.family, r.interface_index, r.next_hop, r.metric
            ))
            .collect()
    ));
    let mut servers: Vec<_> = active.iter().flat_map(|a| a.dns_servers.clone()).collect();
    servers.sort();
    servers.dedup();
    save!(Check::new(
        "dns_config",
        if unknown {
            Status::Unknown
        } else if servers.is_empty() {
            Status::Warning
        } else {
            Status::Normal
        },
        "DNS servers on active adapters; split DNS policy may differ",
        servers.clone()
    ));
    let gateway = s
        .routes
        .iter()
        .filter_map(|r| r.next_hop.parse::<IpAddr>().ok())
        .find(|ip| ip.is_ipv4() && !ip.is_unspecified());
    let tick = Instant::now();
    let mut gateway_check = Check::new(
        "gateway",
        Status::Unknown,
        "No IPv4 gateway to probe (on-link/VPN/IPv6 routes may still work)",
        vec![],
    );
    if let Some(ip) = gateway {
        match icmp::ping(ip, 64, timeout, &token).await {
            Ok(r) => {
                gateway_check.status = if r.reached {
                    Status::Normal
                } else {
                    Status::Unknown
                };
                gateway_check.summary = r.message;
                gateway_check.evidence =
                    vec![format!("{ip}: {:?} ms, code {}", r.rtt_ms, r.status_code)];
            }
            Err(e) => gateway_check.evidence.push(e.to_string()),
        }
    }
    gateway_check.duration_ms = tick.elapsed().as_millis() as u64;
    save!(gateway_check);
    for (id, target) in [("ipv4", &settings.tcp_v4), ("ipv6", &settings.tcp_v6)] {
        let tick = Instant::now();
        let addr = validation::endpoint(target)?;
        let result = connectivity::tcp(addr, timeout, &token).await;
        let mut c = Check::new(
            id,
            if result.is_ok() {
                Status::Normal
            } else {
                Status::Error
            },
            "Direct TCP connection to configured IP/port; one target only",
            vec![format!(
                "{target}: {}",
                result
                    .map(|t| format!("{t:.2} ms"))
                    .unwrap_or_else(|e| e.to_string())
            )],
        );
        c.duration_ms = tick.elapsed().as_millis() as u64;
        save!(c);
    }
    let tick = Instant::now();
    let resolution = connectivity::resolve(&settings.domain, 443, timeout, &token).await;
    let mut c = Check::new(
        "system_dns",
        if resolution.is_ok() {
            Status::Normal
        } else {
            Status::Error
        },
        format!("System resolver: {}", settings.domain),
        resolution
            .map(|a| a.iter().map(|v| v.ip().to_string()).collect())
            .unwrap_or_else(|e| vec![e.to_string()]),
    );
    c.duration_ms = tick.elapsed().as_millis() as u64;
    save!(c);
    let tick = Instant::now();
    let c = if let Some(ip) = servers.first().and_then(|s| s.parse::<IpAddr>().ok()) {
        match dns::query(
            &settings.domain,
            &SocketAddr::new(ip, 53).to_string(),
            "udp",
            false,
            timeout,
            &token,
        )
        .await
        {
            Ok(r) => Check::new(
                "dns_udp",
                if r.error.is_some() {
                    Status::Error
                } else if r.truncated || r.records.is_empty() {
                    Status::Warning
                } else {
                    Status::Normal
                },
                "Direct UDP A query to one configured resolver",
                vec![format!(
                    "{}: records {:?}; error {:?}; {:.2} ms; truncated={}",
                    r.server, r.records, r.error, r.elapsed_ms, r.truncated
                )],
            ),
            Err(e) => Check::new("dns_udp", Status::Unknown, e.to_string(), vec![]),
        }
    } else {
        Check::new(
            "dns_udp",
            Status::Unknown,
            "No configured DNS endpoint available",
            vec![],
        )
    };
    let mut c = c;
    c.duration_ms = tick.elapsed().as_millis() as u64;
    save!(c);
    for (id, url) in [("http", &settings.http_url), ("https", &settings.https_url)] {
        let tick = Instant::now();
        let mut c = match connectivity::http(url, timeout, &token).await {
            Ok(r) => Check::new(
                id,
                if r.redirect || r.expected_body == Some(false) {
                    Status::Warning
                } else if (200..300).contains(&r.status) {
                    Status::Normal
                } else {
                    Status::Error
                },
                r.detail,
                vec![
                    url.clone(),
                    format!(
                        "known_body_matches={:?}; redirect={}",
                        r.expected_body, r.redirect
                    ),
                ],
            ),
            Err(e) => Check::new(id, Status::Error, e.to_string(), vec![url.clone()]),
        };
        c.duration_ms = tick.elapsed().as_millis() as u64;
        save!(c);
    }
    let tick = Instant::now();
    let tls = connectivity::tls(&settings.https_url, timeout, &token).await;
    let mut c = Check::new(
        "tls",
        if tls.is_ok() {
            Status::Normal
        } else {
            Status::Error
        },
        tls.unwrap_or_else(|e| e.to_string()),
        vec![
            settings.https_url.clone(),
            "Direct TLS; public WebPKI roots; system enterprise roots are not imported".into(),
        ],
    );
    c.duration_ms = tick.elapsed().as_millis() as u64;
    save!(c);
    let tick = Instant::now();
    let mut c = Check::new(
        "proxy",
        Status::Unknown,
        "No static system proxy; PAC is not executed",
        report.snapshot.proxy.notes.clone(),
    );
    if let Some(config) = report.snapshot.proxy.system_server.as_ref() {
        if let Some((host, port)) = proxy::configured_endpoint(config) {
            let r = async {
                let addr = connectivity::resolve(&host, port, timeout, &token).await?[0];
                connectivity::tcp(addr, timeout, &token).await
            }
            .await;
            c.status = if r.is_ok() {
                Status::Normal
            } else {
                Status::Error
            };
            c.summary = "Static proxy TCP port only; protocol/auth not inferred".into();
            c.evidence.push(format!(
                "{host}:{port}: {}",
                r.map(|t| format!("{t:.2} ms"))
                    .unwrap_or_else(|e| e.to_string())
            ));
        } else {
            c.evidence
                .push("Proxy format cannot be safely parsed".into());
        }
    }
    c.duration_ms = tick.elapsed().as_millis() as u64;
    save!(c);
    let wifi: Vec<_> = report
        .snapshot
        .adapters
        .iter()
        .filter(|a| a.up && a.kind == "wifi")
        .filter_map(|a| a.signal_percent.map(|q| format!("{}: {q}%", a.name)))
        .collect();
    let weak = report
        .snapshot
        .adapters
        .iter()
        .any(|a| a.up && a.kind == "wifi" && a.signal_percent.is_some_and(|v| v < 40));
    save!(Check::new(
        "wifi",
        if wifi.is_empty() {
            Status::Unknown
        } else if weak {
            Status::Warning
        } else {
            Status::Normal
        },
        "WLAN signal quality; unknown if unavailable / location permission denied",
        wifi
    ));
    let tick = Instant::now();
    let mut rtts = Vec::new();
    let mut evidence = Vec::new();
    let target = validation::endpoint(&settings.tcp_v4)?.ip();
    for i in 0..4 {
        if token.is_cancelled() {
            break;
        }
        match icmp::ping(target, 64, timeout, &token).await {
            Ok(r) => {
                if r.reached {
                    if let Some(t) = r.rtt_ms {
                        rtts.push(t);
                    }
                }
                evidence.push(format!("sample {}: {} {:?} ms", i + 1, r.message, r.rtt_ms));
            }
            Err(e) => evidence.push(e.to_string()),
        }
        if i < 3 {
            tokio::select! { _ = token.cancelled() => {}, _ = tokio::time::sleep(Duration::from_secs(1)) => {} }
        }
    }
    let min = rtts.iter().copied().fold(f64::INFINITY, f64::min);
    let max = rtts.iter().copied().fold(0.0, f64::max);
    let avg = rtts.iter().sum::<f64>() / rtts.len().max(1) as f64;
    let mut c = Check::new(
        "quality",
        if rtts.len() < 3 {
            Status::Unknown
        } else if avg > 150.0 || max - min > 50.0 {
            Status::Warning
        } else {
            Status::Normal
        },
        format!(
            "{} / 4 ICMP replies; mean {avg:.1} ms; range {:.1} ms; short observation only",
            rtts.len(),
            if rtts.is_empty() { 0.0 } else { max - min }
        ),
        evidence,
    );
    c.duration_ms = tick.elapsed().as_millis() as u64;
    save!(c);
    finish(&mut report, start);
    Ok(report)
}
fn finish(report: &mut Report, start: Instant) {
    report.duration_ms = start.elapsed().as_millis() as u64;
    report.findings = rules::evaluate(&report.snapshot, &report.checks);
    (report.score, report.score_basis) = rules::score(&report.checks);
    if report.cancelled {
        report.score = None;
        report.score_basis =
            "诊断已取消，仅保留部分证据，不计算评分 / Cancelled; partial evidence only, no score"
                .into();
    }
}
