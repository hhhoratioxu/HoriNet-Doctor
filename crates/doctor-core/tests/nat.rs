//! Synthetic STUN servers on loopback only. These are protocol fixtures, not Internet observations.
use horinet_core::{models::*, network::nat, reports, rules, validation, Error};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy)]
enum Mode {
    FullCone,
    Restricted,
    PortRestricted,
    Symmetric,
    AddressMapped,
    Unsupported,
    IgnoreChange,
    Error420,
    Silent,
    DeadAlternate,
    Unstable,
    Direct,
}
struct Lab {
    address: String,
    log: Arc<Mutex<Vec<(usize, u32, SocketAddr)>>>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}
impl Drop for Lab {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

fn attr(packet: &mut Vec<u8>, kind: u16, value: &[u8]) {
    packet.extend(kind.to_be_bytes());
    packet.extend((value.len() as u16).to_be_bytes());
    packet.extend(value);
    while !packet.len().is_multiple_of(4) {
        packet.push(0);
    }
}
fn address(addr: SocketAddr, xor: bool) -> Vec<u8> {
    let SocketAddr::V4(v4) = addr else {
        panic!("IPv4 fixture")
    };
    let mut data = vec![0, 1];
    data.extend((v4.port() ^ if xor { 0x2112 } else { 0 }).to_be_bytes());
    data.extend(
        v4.ip()
            .octets()
            .iter()
            .zip([0x21, 0x12, 0xa4, 0x42])
            .map(|(b, m)| *b ^ if xor { m } else { 0 }),
    );
    data
}
async fn lab(mode: Mode) -> Lab {
    let primary = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let other_ip = UdpSocket::bind((
        std::net::Ipv4Addr::new(127, 0, 0, 2),
        primary.local_addr().unwrap().port(),
    ))
    .await
    .unwrap();
    let other_port = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let other_both = UdpSocket::bind((
        std::net::Ipv4Addr::new(127, 0, 0, 2),
        other_port.local_addr().unwrap().port(),
    ))
    .await
    .unwrap();
    let sockets: Arc<Vec<_>> = Arc::new(vec![primary, other_ip, other_port, other_both]);
    let alternate = sockets[3].local_addr().unwrap();
    let endpoint = sockets[0].local_addr().unwrap().to_string();
    let log = Arc::new(Mutex::new(vec![]));
    let mut tasks = vec![];
    for index in 0..4 {
        let sockets = sockets.clone();
        let log = log.clone();
        tasks.push(tokio::spawn(async move {
            let mut packet = [0; 2048];
            let mut baseline_count = 0;
            loop {
                let (len, peer) = sockets[index].recv_from(&mut packet).await.unwrap();
                if len < 20 {
                    continue;
                }
                let change = if len >= 28 {
                    u32::from_be_bytes(packet[24..28].try_into().unwrap())
                } else {
                    0
                };
                log.lock().unwrap().push((index, change, peer));
                if matches!(mode, Mode::Silent)
                    || (matches!(mode, Mode::DeadAlternate) && index != 0)
                {
                    continue;
                }
                if index == 0 && change == 0 {
                    baseline_count += 1;
                }
                if (matches!(mode, Mode::Restricted) && change == 6)
                    || (matches!(mode, Mode::PortRestricted | Mode::DeadAlternate) && change != 0)
                {
                    continue;
                }
                let response_index = if matches!(mode, Mode::IgnoreChange) {
                    index
                } else if change == 6 {
                    3
                } else if change == 2 {
                    2
                } else {
                    index
                };
                let error = matches!(mode, Mode::Error420) && change != 0;
                let mut response = vec![1, if error { 0x11 } else { 1 }, 0, 0];
                response.extend([0x21, 0x12, 0xa4, 0x42]);
                response.extend(&packet[8..20]);
                if error {
                    attr(&mut response, 9, &[0, 0, 4, 20]);
                } else {
                    let mapped = if matches!(mode, Mode::Direct) {
                        peer
                    } else {
                        let port = match mode {
                            Mode::Symmetric => [45000, 45001, 45003, 45002][index],
                            Mode::AddressMapped if matches!(index, 1 | 3) => 45001,
                            Mode::Unstable if baseline_count >= 3 => 45010,
                            _ => 45000,
                        };
                        SocketAddr::from(([203, 0, 113, 9], port))
                    };
                    attr(&mut response, 0x20, &address(mapped, true));
                    if !matches!(mode, Mode::Unsupported) {
                        attr(
                            &mut response,
                            0x802b,
                            &address(sockets[response_index].local_addr().unwrap(), false),
                        );
                        attr(&mut response, 0x802c, &address(alternate, false));
                    }
                }
                let size = (response.len() - 20) as u16;
                response[2..4].copy_from_slice(&size.to_be_bytes());
                // An unrelated transaction must not poison the current request.
                let mut stale = response.clone();
                stale[8] ^= 1;
                sockets[response_index].send_to(&stale, peer).await.unwrap();
                sockets[response_index]
                    .send_to(&response, peer)
                    .await
                    .unwrap();
            }
        }));
    }
    Lab {
        address: endpoint,
        log,
        tasks,
    }
}
async fn run(mode: Mode) -> (Lab, NatResult) {
    let lab = lab(mode).await;
    let r = nat::detect(
        &lab.address,
        true,
        Duration::from_millis(500),
        &CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    (lab, r)
}
#[tokio::test]
async fn full_cone_uses_one_socket_and_filters_before_opening_alternate_bindings() {
    let (lab, r) = run(Mode::FullCone).await;
    assert_eq!(r.nat_type, NatType::FullCone);
    assert_eq!(r.mapping, NatBehavior::EndpointIndependent);
    assert_eq!(r.filtering, NatBehavior::EndpointIndependent);
    let log = lab.log.lock().unwrap();
    let alternate = log.iter().position(|(i, _, _)| *i != 0).unwrap();
    assert!(log[..alternate].iter().any(|(_, change, _)| *change == 6));
    assert!(log
        .iter()
        .all(|(_, _, peer)| peer.port() == log[0].2.port()));
    assert!(r.probes.len() <= 8);
}
#[tokio::test]
async fn restricted_cone_is_timeout_based_and_low_confidence() {
    let (_, r) = run(Mode::Restricted).await;
    assert_eq!(r.nat_type, NatType::RestrictedCone);
    assert_eq!(r.filtering, NatBehavior::AddressDependent);
    assert_eq!(r.confidence, "low");
    assert!(r.limitations.iter().any(|s| s.contains("finite timeouts")));
}
#[tokio::test]
async fn port_restricted_requires_working_alternate_listeners() {
    let (_, r) = run(Mode::PortRestricted).await;
    assert_eq!(r.nat_type, NatType::PortRestrictedCone);
    assert_eq!(r.filtering, NatBehavior::AddressAndPortDependent);
    assert!(r
        .probes
        .iter()
        .any(|p| p.stage == "alternate_port_control" && p.status == Status::Normal));
}
#[tokio::test]
async fn symmetric_address_and_port_dependent_mapping() {
    let (_, r) = run(Mode::Symmetric).await;
    assert_eq!(r.nat_type, NatType::Symmetric);
    assert_eq!(r.mapping, NatBehavior::AddressAndPortDependent);
    assert_eq!(nat::findings(&r)[0].code, "nat_restrictive");
}
#[tokio::test]
async fn symmetric_address_dependent_mapping_is_distinguished() {
    let (_, r) = run(Mode::AddressMapped).await;
    assert_eq!(r.nat_type, NatType::Symmetric);
    assert_eq!(r.mapping, NatBehavior::AddressDependent);
}
#[tokio::test]
async fn basic_stun_without_rfc5780_is_not_assumed_full_cone() {
    let (_, r) = run(Mode::Unsupported).await;
    assert_eq!(r.nat_type, NatType::Unknown);
    assert_eq!(r.mapping, NatBehavior::Unknown);
    assert!(r.mapped_endpoint.is_some());
    assert_eq!(r.probes.len(), 1);
}
#[tokio::test]
async fn server_ignoring_change_request_does_not_look_open() {
    let (_, r) = run(Mode::IgnoreChange).await;
    assert_eq!(r.filtering, NatBehavior::Unknown);
    assert_eq!(r.nat_type, NatType::Unknown);
    assert!(r.probes.iter().any(|p| p.detail.contains("did not honor")));
}
#[tokio::test]
async fn error_420_is_not_inferred_as_restrictive_filtering() {
    let (_, r) = run(Mode::Error420).await;
    assert_eq!(r.filtering, NatBehavior::Unknown);
    assert_eq!(r.nat_type, NatType::Unknown);
    assert!(r.probes.iter().any(|p| p.detail.contains("420")));
}
#[tokio::test]
async fn dead_alternates_remain_unknown() {
    let (_, r) = run(Mode::DeadAlternate).await;
    assert_eq!(r.filtering, NatBehavior::Unknown);
    assert_eq!(r.nat_type, NatType::Unknown);
}
#[tokio::test]
async fn unstable_baseline_withdraws_classification() {
    let (_, r) = run(Mode::Unstable).await;
    assert_eq!(r.nat_type, NatType::Unknown);
    assert_eq!(r.mapping, NatBehavior::Unknown);
    assert!(r.limitations.iter().any(|s| s.contains("withdrawn")));
}
#[tokio::test]
async fn no_translation_does_not_claim_unrestricted_inbound_access() {
    let (_, r) = run(Mode::Direct).await;
    assert_eq!(r.nat_type, NatType::NoTranslation);
    assert!(r
        .limitations
        .iter()
        .any(|s| s.contains("inbound reachability")));
}
#[tokio::test]
async fn no_consent_sends_no_packets() {
    let lab = lab(Mode::FullCone).await;
    let r = nat::detect(
        &lab.address,
        false,
        Duration::from_millis(500),
        &CancellationToken::new(),
        |_| {},
    )
    .await;
    assert!(matches!(r, Err(Error::Invalid(_))));
    assert!(lab.log.lock().unwrap().is_empty());
}
#[tokio::test]
async fn user_cancellation_interrupts_stun_wait() {
    let lab = lab(Mode::Silent).await;
    let token = CancellationToken::new();
    let cancel = token.clone();
    let task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        cancel.cancel();
    });
    let start = Instant::now();
    let r = nat::detect(&lab.address, true, Duration::from_secs(5), &token, |_| {}).await;
    assert!(matches!(r, Err(Error::Cancelled)));
    assert!(start.elapsed() < Duration::from_secs(2));
    task.await.unwrap();
}
#[tokio::test]
async fn stun_timeout_is_unknown_not_a_network_outage() {
    let (_, r) = run(Mode::Silent).await;
    assert_eq!(r.status, Status::Unknown);
    assert_eq!(r.nat_type, NatType::Unknown);
    assert!(nat::findings(&r).is_empty());
    assert_eq!(r.probes.len(), 1);
    assert!(r.probes[0].attempts <= 3);
}
#[tokio::test]
async fn nat_history_and_all_export_formats_redact_public_and_local_endpoints() {
    let (_, r) = run(Mode::FullCone).await;
    let mut report = nat::report(r);
    report.snapshot.adapters.push(Adapter {
        name: "normal".into(),
        description: "full_cone".into(),
        ..Default::default()
    });
    let redacted = reports::exported(&report, false);
    assert_eq!(redacted.nat.unwrap().nat_type, NatType::FullCone);
    assert_eq!(redacted.checks[0].status, Status::Normal);
    let temp = tempfile::tempdir().unwrap();
    let store = horinet_core::storage::Store::new(temp.path().to_path_buf()).unwrap();
    store.save(&report, 100).unwrap();
    let loaded = store.get(&report.id).unwrap();
    let mut legacy = serde_json::to_value(&loaded).unwrap();
    legacy.as_object_mut().unwrap().remove("nat");
    assert!(serde_json::from_value::<Report>(legacy)
        .unwrap()
        .nat
        .is_none());
    assert_eq!(loaded.nat.as_ref().unwrap().nat_type, NatType::FullCone);
    for text in [
        reports::json(&loaded, false).unwrap(),
        reports::markdown(&loaded, false),
        reports::summary(&loaded, false),
    ] {
        assert!(!text.contains("203.0.113.9"));
        assert!(!text.contains("127.0.0.1"));
        assert!(!text.contains("127.0.0.2"));
        assert!(text.contains("[redacted]"));
    }
    assert!(reports::json(&loaded, true)
        .unwrap()
        .contains("203.0.113.9"));
}
#[test]
fn nat_does_not_change_health_score_and_old_settings_need_no_new_consent() {
    let mut checks = vec![Check::new("ipv4", Status::Normal, "fixture", vec![])];
    let original = rules::score(&checks);
    checks.push(Check::new("nat", Status::Warning, "fixture", vec![]));
    assert_eq!(rules::score(&checks), original);
    assert_eq!(rules::score(&checks[1..]).0, None);
    let settings: Settings = serde_json::from_str("{}").unwrap();
    assert!(!settings.nat_enabled);
    assert_eq!(horinet_core::diagnostics::total(&settings), 16);
    let enabled = Settings {
        nat_enabled: true,
        ..settings
    };
    assert_eq!(horinet_core::diagnostics::total(&enabled), 17);
    validation::settings(&enabled).unwrap();
}
#[test]
fn stun_input_is_one_valid_ipv4_or_hostname_endpoint() {
    for bad in [
        "",
        "https://a.test:3478",
        "a.test:0",
        "a.test:65536",
        "[::1]:3478",
        "224.0.0.1:3478",
        "0.0.0.0:3478",
        "a.test:3478;whoami",
        "a.test:3478,b.test:3478",
        "a test:3478",
    ] {
        assert!(validation::stun_server(bad).is_err(), "accepted {bad}");
    }
    assert_eq!(
        validation::stun_server("stun.example.com:3478").unwrap(),
        ("stun.example.com", 3478)
    );
}
