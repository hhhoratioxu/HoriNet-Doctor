use horinet_core::{
    bounded, diagnostics,
    models::*,
    network::{connectivity, dns, icmp, proxy},
    reports, rules,
    storage::Store,
    validation, Error,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

fn checks(states: &[(&str, Status)]) -> Vec<Check> {
    states
        .iter()
        .map(|(id, s)| {
            Check::new(
                id,
                *s,
                "test fixture, never application data",
                vec!["fixture evidence".into()],
            )
        })
        .collect()
}
fn has(states: &[(&str, Status)], code: &str) -> bool {
    rules::evaluate(&Snapshot::default(), &checks(states))
        .iter()
        .any(|f| f.code == code)
}
fn report() -> Report {
    Report {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        started_at: "2026-10-14T12:34:56+00:00".into(),
        platform: "windows x86_64".into(),
        duration_ms: 20,
        cancelled: false,
        snapshot: Snapshot {
            adapters: vec![Adapter {
                name: "Fixture adapter".into(),
                description: "Fixture driver".into(),
                mac: "AA:BB:CC:DD:EE:FF".into(),
                ssid: Some("Secret SSID".into()),
                addresses: vec!["192.168.9.2".into(), "2001:db8::123".into()],
                ..Default::default()
            }],
            proxy: ProxyConfig {
                system_server: Some("proxy.internal:7890".into()),
                pac_url: Some("https://proxy.internal/config.pac".into()),
                ..Default::default()
            },
            ..Default::default()
        },
        checks: checks(&[("ipv4", Status::Normal)]),
        findings: vec![],
        score: Some(100),
        score_basis: "fixture".into(),
    }
}
#[test]
fn dhcp_failure() {
    assert!(has(&[("dhcp", Status::Error)], "dhcp_failed"));
}
#[test]
fn ip_works_dns_fails() {
    assert!(has(
        &[("ipv4", Status::Normal), ("system_dns", Status::Error)],
        "dns_failed"
    ));
}
#[test]
fn icmp_filtered_tcp_works_is_not_offline() {
    let f = rules::evaluate(
        &Snapshot::default(),
        &checks(&[
            ("gateway", Status::Unknown),
            ("quality", Status::Unknown),
            ("ipv4", Status::Normal),
        ]),
    );
    assert!(f
        .iter()
        .all(|f| !matches!(f.code.as_str(), "offline_evidence" | "adapter_disconnected")));
}
#[test]
fn ipv4_only() {
    assert!(has(
        &[("ipv4", Status::Normal), ("ipv6", Status::Error)],
        "ipv6_unavailable"
    ));
}
#[test]
fn ipv6_only() {
    assert!(has(
        &[("ipv6", Status::Normal), ("ipv4", Status::Error)],
        "ipv4_unavailable"
    ));
}
#[test]
fn proxy_port_unavailable() {
    assert!(has(&[("proxy", Status::Error)], "proxy_unreachable"));
}
#[test]
fn disconnected_network() {
    assert!(has(
        &[
            ("adapters", Status::Error),
            ("ipv4", Status::Error),
            ("ipv6", Status::Error)
        ],
        "offline_evidence"
    ));
}
#[test]
fn target_outage_not_general_outage() {
    let c = checks(&[
        ("ipv4", Status::Normal),
        ("system_dns", Status::Normal),
        ("https", Status::Error),
    ]);
    let f = rules::evaluate(&Snapshot::default(), &c);
    assert!(f.iter().any(|f| f.code == "target_https_failed"));
    assert!(!f.iter().any(|f| f.code == "offline_evidence"));
}
#[test]
fn route_missing() {
    assert!(has(&[("routes", Status::Error)], "route_missing"));
}
#[test]
fn portal_is_low_confidence() {
    let f = rules::evaluate(&Snapshot::default(), &checks(&[("http", Status::Warning)]));
    assert_eq!(f[0].code, "portal_possible");
    assert_eq!(f[0].confidence, "low");
}
#[test]
fn wifi_and_variation_rules() {
    assert!(has(&[("wifi", Status::Warning)], "wifi_weak"));
    assert!(has(&[("quality", Status::Warning)], "quality_unstable"));
}
#[test]
fn conflict_is_only_possible() {
    let mut s = Snapshot::default();
    s.proxy.system_enabled = true;
    let f = rules::evaluate(
        &s,
        &checks(&[("proxy", Status::Normal), ("https", Status::Error)]),
    );
    assert_eq!(f[0].code, "proxy_conflict_possible");
    assert_eq!(f[0].confidence, "low");
}
#[test]
fn empty_results_do_not_produce_full_score() {
    assert_eq!(rules::score(&[]).0, None);
    assert_eq!(rules::score(&checks(&[("a", Status::Unknown)])).0, None);
    assert_eq!(
        rules::score(&checks(&[("a", Status::Normal), ("b", Status::Unknown)])).0,
        Some(50)
    );
}
#[tokio::test]
async fn cancellation() {
    let token = CancellationToken::new();
    let t = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        t.cancel();
    });
    let result = bounded(&token, Duration::from_secs(2), async {
        std::future::pending::<horinet_core::Result<()>>().await
    })
    .await;
    assert!(matches!(result, Err(Error::Cancelled)));
}
#[tokio::test]
async fn cancelled_diagnosis_returns_partial_record() {
    let t = CancellationToken::new();
    t.cancel();
    let r = diagnostics::run(Settings::default(), t, |_, _| {})
        .await
        .unwrap();
    assert!(r.cancelled);
    assert!(r.checks.is_empty());
    assert_eq!(r.score, None);
}
#[tokio::test]
async fn timeout() {
    let result = bounded(
        &CancellationToken::new(),
        Duration::from_millis(10),
        async { std::future::pending::<horinet_core::Result<()>>().await },
    )
    .await;
    assert!(matches!(result, Err(Error::Timeout)));
}
#[test]
fn privacy_redacts_nested_evidence_and_preserves_time() {
    let mut r = report();
    r.checks[0].evidence=vec!["192.168.9.2 8.8.8.8 2001:db8::123 fe80::42 AA:BB:CC:DD:EE:FF Secret SSID proxy.internal:7890 C:\\Users\\FixtureUser\\Desktop".into()];
    let text = reports::json(&r, false).unwrap();
    for secret in [
        "192.168.9.2",
        "8.8.8.8",
        "2001:db8::123",
        "fe80::42",
        "AA:BB:CC:DD:EE:FF",
        "Secret SSID",
        "proxy.internal",
        "FixtureUser",
    ] {
        assert!(!text.contains(secret), "leaked {secret}");
    }
    assert!(text.contains(&r.started_at));
    assert!(reports::json(&r, true).unwrap().contains("Secret SSID"));
    assert!(!reports::markdown(&r, false).contains("Secret SSID"));
}
#[test]
fn history_retention_and_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(dir.path()).unwrap();
    let a = report();
    s.save(&a, 1).unwrap();
    let mut b = report();
    b.started_at = "2026-10-15T00:00:00Z".into();
    s.save(&b, 1).unwrap();
    assert_eq!(s.list().unwrap().len(), 1);
    assert!(s.get(&a.id).is_err());
    assert!(s.get("../../secrets").is_err());
    s.clear().unwrap();
    assert!(s.list().unwrap().is_empty());
}
#[test]
fn settings_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(dir.path()).unwrap();
    let mut cfg = Settings {
        theme: "light".into(),
        ..Settings::default()
    };
    s.save_settings(&cfg).unwrap();
    assert_eq!(s.settings().unwrap().theme, "light");
    cfg.theme = "dark".into();
    s.save_settings(&cfg).unwrap();
    assert_eq!(s.settings().unwrap().theme, "dark");
}
#[test]
fn input_rejects_shell_urls_credentials_and_multicast() {
    for h in [
        "x;calc",
        "a b",
        "$(test)",
        "https://example.com",
        "-oops",
        "a..b",
    ] {
        assert!(validation::host(h).is_err(), "{h}");
    }
    assert!(validation::endpoint("224.0.0.1:53").is_err());
    assert!(validation::endpoint("255.255.255.255:53").is_err());
    assert!(validation::web_url("https://user:password@example.com", true).is_err());
    assert!(validation::web_url("file:///etc/passwd", false).is_err());
}
#[test]
fn dns_parser_rejects_malformed_and_mismatched_packets() {
    let q = dns::query_packet("example.com", false, 42).unwrap();
    for len in 0..q.len() {
        assert!(dns::parse_response(&q[..len], &q).is_err());
    }
    let mut r = q.clone();
    r[2] = 0x81;
    let (a, t) = dns::parse_response(&r, &q).unwrap();
    assert!(a.is_empty());
    assert!(!t);
    r[0] = 9;
    assert!(dns::parse_response(&r, &q).is_err());
    r[0] = q[0];
    r[12] = 0xc0;
    r[13] = 12;
    assert!(dns::parse_response(&r, &q).is_err());
}
#[test]
fn dns_parser_decodes_a_and_aaaa_and_nxdomain() {
    for v6 in [false, true] {
        let q = dns::query_packet("example.com", v6, 42).unwrap();
        let mut r = q.clone();
        r[2] = 0x81;
        r[3] = 0x80;
        r[7] = 1;
        r.extend([
            0xc0,
            0x0c,
            0,
            if v6 { 28 } else { 1 },
            0,
            1,
            0,
            0,
            0,
            60,
            0,
            if v6 { 16 } else { 4 },
        ]);
        if v6 {
            r.extend(std::net::Ipv6Addr::LOCALHOST.octets());
        } else {
            r.extend([127, 0, 0, 1]);
        }
        assert_eq!(dns::parse_response(&r, &q).unwrap().0.len(), 1);
        r[3] = 0x83;
        assert!(dns::parse_response(&r, &q)
            .unwrap_err()
            .to_string()
            .contains("NXDOMAIN"));
    }
}
#[tokio::test]
async fn loopback_tcp_and_icmp() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let token = CancellationToken::new();
    assert!(connectivity::tcp(
        listener.local_addr().unwrap(),
        Duration::from_secs(1),
        &token
    )
    .await
    .is_ok());
    #[cfg(windows)]
    {
        let r = icmp::ping(
            "127.0.0.1".parse().unwrap(),
            64,
            Duration::from_secs(1),
            &token,
        )
        .await
        .unwrap();
        assert!(r.reached);
        assert_eq!(r.address.as_deref(), Some("127.0.0.1"));
        let v6 = icmp::ping("::1".parse().unwrap(), 64, Duration::from_secs(1), &token)
            .await
            .unwrap();
        assert!(v6.reached);
        assert_eq!(v6.address.as_deref(), Some("::1"));
    }
}
#[tokio::test]
async fn udp_dns_uses_real_socket() {
    let s = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = s.local_addr().unwrap();
    let responder = tokio::spawn(async move {
        let mut buf = [0; 512];
        let (n, peer) = s.recv_from(&mut buf).await.unwrap();
        buf[2] = 0x81;
        buf[3] = 0x83;
        s.send_to(&buf[..n], peer).await.unwrap();
    });
    let r = dns::query(
        "example.com",
        &addr.to_string(),
        "udp",
        false,
        Duration::from_secs(1),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(r.error.unwrap().contains("NXDOMAIN"));
    responder.await.unwrap();
}
#[tokio::test]
async fn proxy_connect_handshake() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut s, _) = l.accept().await.unwrap();
        let mut buf = [0; 1024];
        let n = s.read(&mut buf).await.unwrap();
        assert!(String::from_utf8_lossy(&buf[..n]).starts_with("CONNECT example.com:443 HTTP/1.1"));
        s.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .await
            .unwrap();
    });
    let r = proxy::test(
        &addr.to_string(),
        "http",
        "https://example.com",
        Duration::from_secs(1),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(r.status, Status::Normal);
    server.await.unwrap();
}
#[tokio::test]
async fn dns_timeout_distinguished_from_records() {
    let s = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let r = dns::query(
        "example.com",
        &s.local_addr().unwrap().to_string(),
        "udp",
        false,
        Duration::from_millis(15),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(r.records.is_empty());
    assert!(r.error.unwrap().contains("timed out"));
}
