//! IPv4 UDP NAT behavior discovery. Results describe this socket/path/time only.
use super::{connectivity, stun};
use crate::{models::*, validation, Error, Result};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket as RouteSocket},
    time::{Duration, Instant},
};
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

const LIMITATIONS: &[&str] = &[
    "仅代表本次 IPv4 UDP 端口、所选服务器和当前路径；不等于游戏主机 NAT 1/2/3 或 Open/Moderate/Strict / Applies only to this IPv4 UDP socket, selected server and current path; not a console NAT 1/2/3 or Open/Moderate/Strict rating",
    "STUN 未认证；VPN、TUN、UDP 代理、多层 NAT 与防火墙可能共同影响结果，不能据此确认 CGNAT、双重 NAT 或 P2P 一定可用 / STUN is unauthenticated; VPNs, TUNs, UDP proxies, multiple NATs and firewalls can affect results; CGNAT, double NAT and P2P success are not established",
    "超时可能来自服务器、丢包或过滤；未响应不代表整体断网 / A timeout may reflect the server, packet loss or filtering; no reply does not mean an Internet outage",
];

fn empty(server: &str) -> NatResult {
    NatResult {
        server: server.into(),
        resolved_server: None,
        local_endpoint: None,
        mapped_endpoint: None,
        alternate_endpoint: None,
        mapping: NatBehavior::Unknown,
        filtering: NatBehavior::Unknown,
        nat_type: NatType::Unknown,
        status: Status::Unknown,
        confidence: "low".into(),
        summary: "证据不足，无法判断 NAT 类型 / Insufficient evidence to classify NAT".into(),
        evidence: vec![],
        limitations: LIMITATIONS.iter().map(|s| (*s).into()).collect(),
        probes: vec![],
        duration_ms: 0,
    }
}

pub fn classify(no_translation: bool, mapping: NatBehavior, filtering: NatBehavior) -> NatType {
    use NatBehavior::*;
    if no_translation {
        return NatType::NoTranslation;
    }
    match (mapping, filtering) {
        (AddressDependent | AddressAndPortDependent, _) => NatType::Symmetric,
        (EndpointIndependent, EndpointIndependent) => NatType::FullCone,
        (EndpointIndependent, AddressDependent) => NatType::RestrictedCone,
        (EndpointIndependent, AddressAndPortDependent) => NatType::PortRestrictedCone,
        _ => NatType::Unknown,
    }
}
pub fn type_label(kind: NatType) -> &'static str {
    match kind {
        NatType::Unknown => "无法判断 / Unknown",
        NatType::NoTranslation => "未观察到地址转换 / No translation observed",
        NatType::FullCone => "全锥型（本次观测） / Full cone (observed)",
        NatType::RestrictedCone => "地址受限锥型（推测） / Address restricted cone (inferred)",
        NatType::PortRestrictedCone => "端口受限锥型（推测） / Port restricted cone (inferred)",
        NatType::Symmetric => "对称型映射（本次观测） / Symmetric mapping (observed)",
    }
}

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || a == 0
        || a >= 240
        || (a == 100 && (64..=127).contains(&b))
        || (a == 198 && matches!(b, 18 | 19))
        || (a == 192 && b == 0)
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113))
}
fn allowed_alternate(primary: SocketAddr, other: SocketAddr) -> bool {
    let (SocketAddr::V4(p), SocketAddr::V4(o)) = (primary, other) else {
        return false;
    };
    if p.ip() == o.ip() || p.port() == o.port() || validation::endpoint(&other.to_string()).is_err()
    {
        return false;
    }
    // A public server cannot use OTHER-ADDRESS to make the client probe a LAN or loopback host.
    if public_v4(*p.ip()) {
        return public_v4(*o.ip());
    }
    if p.ip().is_loopback() {
        return o.ip().is_loopback();
    }
    if p.ip().is_link_local() {
        return o.ip().is_link_local();
    }
    if p.ip().is_private() {
        return o.ip().is_private() || public_v4(*o.ip());
    }
    public_v4(*o.ip())
}
fn same_baseline(
    reply: &stun::Binding,
    baseline: &stun::Binding,
    primary: SocketAddr,
    other: SocketAddr,
) -> bool {
    reply.mapped == baseline.mapped && reply.origin == Some(primary) && reply.other == Some(other)
}

struct Session<'a, F> {
    socket: UdpSocket,
    timeout: Duration,
    token: &'a CancellationToken,
    emit: F,
    result: NatResult,
}
impl<F: Fn(NatProbe)> Session<'_, F> {
    async fn probe(
        &mut self,
        stage: &str,
        target: SocketAddr,
        expected: SocketAddr,
        change: u32,
    ) -> Result<stun::Exchange> {
        // At most ten new transactions/s; no background threads or sustained keepalive traffic.
        crate::bounded(self.token, Duration::from_secs(1), async {
            tokio::time::sleep(Duration::from_millis(110)).await;
            Ok(())
        })
        .await?;
        let reply = match stun::binding(
            &self.socket,
            target,
            expected,
            change,
            self.timeout,
            self.token,
        )
        .await
        {
            Ok(reply) => reply,
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(e) => stun::Exchange {
                binding: None,
                source: None,
                attempts: 0,
                elapsed_ms: 0,
                timed_out: false,
                detail: e.to_string(),
            },
        };
        let probe = NatProbe {
            stage: stage.into(),
            target: target.to_string(),
            expected_source: expected.to_string(),
            response_source: reply.source.map(|s| s.to_string()),
            mapped_endpoint: reply.binding.as_ref().map(|b| b.mapped.to_string()),
            elapsed_ms: reply.elapsed_ms,
            attempts: reply.attempts,
            status: if reply.binding.is_some() {
                Status::Normal
            } else {
                Status::Unknown
            },
            detail: reply.detail.clone(),
        };
        (self.emit)(probe.clone());
        self.result.probes.push(probe);
        Ok(reply)
    }
    async fn discover(&mut self, primary: SocketAddr) -> Result<()> {
        let first = self.probe("binding", primary, primary, 0).await?;
        let Some(baseline) = first.binding else {
            self.result.evidence.push(first.detail);
            return Ok(());
        };
        self.result.mapped_endpoint = Some(baseline.mapped.to_string());
        let local = self.socket.local_addr()?;
        let no_translation = local == baseline.mapped;
        self.result.evidence.push(format!(
            "Local UDP endpoint {local}; server-observed endpoint {}",
            baseline.mapped
        ));
        self.result.nat_type = classify(no_translation, NatBehavior::Unknown, NatBehavior::Unknown);
        if no_translation {
            self.result.limitations.push("未观察到转换不等于不存在入站防火墙 / No translation observed does not establish unrestricted inbound reachability".into());
        }
        let Some(other) = baseline.other.filter(|o| allowed_alternate(primary, *o)) else {
            self.result.limitations.push("服务器未提供可用的 RFC 5780 OTHER-ADDRESS，映射与过滤分类不可完成 / Server did not provide a usable RFC 5780 OTHER-ADDRESS; mapping and filtering cannot be classified".into());
            return Ok(());
        };
        self.result.alternate_endpoint = Some(other.to_string());
        if baseline.origin != Some(primary) || !baseline.xor_mapped {
            self.result.limitations.push("缺少可信的一致 RESPONSE-ORIGIN / XOR-MAPPED-ADDRESS；保留基本映射证据 / Consistent RESPONSE-ORIGIN / XOR-MAPPED-ADDRESS is missing; retaining basic binding evidence only".into());
            return Ok(());
        }
        self.result.evidence.push(format!("RFC 5780 alternate endpoint {other}; filtering tested before contacting alternate endpoints"));

        // Filtering MUST precede mapping: alternate destinations would open NAT bindings.
        let both = self
            .probe("filter_change_ip_port", primary, other, 6)
            .await?;
        let primary_other_port = SocketAddr::new(primary.ip(), other.port());
        let mut filtering = if both
            .binding
            .as_ref()
            .is_some_and(|b| b.mapped == baseline.mapped && b.origin == Some(other))
        {
            NatBehavior::EndpointIndependent
        } else {
            NatBehavior::Unknown
        };
        let mut both_timed_out = false;
        let mut port_timed_out = false;
        if both.timed_out {
            both_timed_out = true;
            let port = self
                .probe("filter_change_port", primary, primary_other_port, 2)
                .await?;
            if port.binding.as_ref().is_some_and(|b| {
                b.mapped == baseline.mapped && b.origin == Some(primary_other_port)
            }) {
                filtering = NatBehavior::AddressDependent;
            }
            port_timed_out = port.timed_out;
        }
        // Ensure the primary server/path was still responding after filtering timeouts.
        let control = self.probe("binding_control", primary, primary, 0).await?;
        if !control
            .binding
            .as_ref()
            .is_some_and(|b| same_baseline(b, &baseline, primary, other))
        {
            self.result.limitations.push("基线复查失败或公网映射变化，无法可靠分类 / Baseline recheck failed or mapped endpoint changed; classification is unreliable".into());
            self.result.nat_type = NatType::Unknown;
            return Ok(());
        }

        let alt_ip_primary_port = SocketAddr::new(other.ip(), primary.port());
        let second = self
            .probe(
                "mapping_other_ip",
                alt_ip_primary_port,
                alt_ip_primary_port,
                0,
            )
            .await?;
        // Test both alternate corners so a dead alternate listener cannot look like restrictive filtering.
        let third = self.probe("mapping_other_ip_port", other, other, 0).await?;
        let mut mapping = NatBehavior::Unknown;
        if let (Some(b), Some(c)) = (&second.binding, &third.binding) {
            if b.origin == Some(alt_ip_primary_port)
                && c.origin == Some(other)
                && b.xor_mapped
                && c.xor_mapped
            {
                mapping = if b.mapped == baseline.mapped && c.mapped == baseline.mapped {
                    NatBehavior::EndpointIndependent
                } else if b.mapped != baseline.mapped && b.mapped == c.mapped {
                    NatBehavior::AddressDependent
                } else if b.mapped != baseline.mapped && b.mapped != c.mapped {
                    NatBehavior::AddressAndPortDependent
                } else {
                    self.result.limitations.push("备用端口结果与 RFC 5780 分类假设不一致 / Alternate-port observations do not fit the RFC 5780 classification assumptions".into());
                    NatBehavior::Unknown
                };
            }
        }
        if both_timed_out
            && port_timed_out
            && third
                .binding
                .as_ref()
                .is_some_and(|b| b.origin == Some(other))
        {
            let reachable = self
                .probe(
                    "alternate_port_control",
                    primary_other_port,
                    primary_other_port,
                    0,
                )
                .await?;
            if reachable
                .binding
                .as_ref()
                .is_some_and(|b| b.origin == Some(primary_other_port))
            {
                filtering = NatBehavior::AddressAndPortDependent;
                if mapping == NatBehavior::EndpointIndependent
                    && reachable
                        .binding
                        .as_ref()
                        .is_some_and(|b| b.mapped != baseline.mapped)
                {
                    mapping = NatBehavior::Unknown;
                    self.result.limitations.push("备用端口控制发现不同映射，撤回与目标无关的映射判断 / Alternate-port control found a different mapping; endpoint-independent mapping withdrawn".into());
                }
            }
        }
        let final_check = self.probe("binding_final", primary, primary, 0).await?;
        if !final_check
            .binding
            .as_ref()
            .is_some_and(|b| same_baseline(b, &baseline, primary, other))
        {
            self.result.limitations.push("最终映射复查失败或映射不稳定，撤回类型判断 / Final mapping recheck failed or changed; NAT classification withdrawn".into());
            self.result.nat_type = NatType::Unknown;
            return Ok(());
        }
        if both_timed_out {
            self.result.limitations.push("受限过滤基于有限重试后的无响应推测，仍可能受丢包或服务器实现影响 / Restrictive filtering is inferred from finite timeouts; packet loss or server implementation may affect it".into());
        }
        self.result.mapping = mapping;
        self.result.filtering = filtering;
        // A local/public endpoint equality must remain true for every mapping observation.
        let unchanged_local = no_translation
            && [second.binding.as_ref(), third.binding.as_ref()]
                .iter()
                .all(|b| b.is_some_and(|b| b.mapped == local));
        self.result.nat_type = classify(unchanged_local, mapping, filtering);
        Ok(())
    }
}

/// Explicit consent is checked before any name lookup, socket creation or network traffic.
pub async fn detect(
    server: &str,
    consent: bool,
    timeout: Duration,
    token: &CancellationToken,
    emit: impl Fn(NatProbe),
) -> Result<NatResult> {
    if !consent {
        return Err(Error::Invalid("Explicit consent is required: STUN contacts the selected server and discovers your public mapped IP/port".into()));
    }
    validation::stun_server(server)?;
    static LIMIT: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
    crate::bounded(token, Duration::from_secs(60), async {
        let _permit = LIMIT.acquire().await.map_err(|_| Error::Cancelled)?;
        detect_inner(
            server,
            timeout.clamp(Duration::from_millis(500), Duration::from_secs(5)),
            token,
            emit,
        )
        .await
    })
    .await
}
async fn detect_inner(
    server: &str,
    timeout: Duration,
    token: &CancellationToken,
    emit: impl Fn(NatProbe),
) -> Result<NatResult> {
    let start = Instant::now();
    let (host, port) = validation::stun_server(server)?;
    let mut result = empty(server);
    let addresses = match connectivity::resolve(host, port, timeout, token).await {
        Ok(a) => a,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(e) => {
            result.evidence.push(e.to_string());
            result.duration_ms = start.elapsed().as_millis() as u64;
            return Ok(result);
        }
    };
    let Some(primary) = addresses.into_iter().find(SocketAddr::is_ipv4) else {
        result
            .evidence
            .push("Selected STUN hostname has no IPv4 address".into());
        result.duration_ms = start.elapsed().as_millis() as u64;
        return Ok(result);
    };
    validation::endpoint(&primary.to_string())?;
    result.resolved_server = Some(primary.to_string());
    // UDP connect on this temporary socket only asks the OS for a source route; it sends no datagram.
    let route = RouteSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    route.connect(primary)?;
    let source_ip = route.local_addr()?.ip();
    drop(route);
    let socket = UdpSocket::bind(SocketAddr::new(source_ip, 0)).await?;
    result.local_endpoint = Some(socket.local_addr()?.to_string());
    if let SocketAddr::V4(v4) = primary {
        if !public_v4(*v4.ip()) {
            result.limitations.push("所选服务器不是可确认的公网 IPv4 地址，此结果不代表互联网出口 NAT / Selected server is not a confirmed public IPv4 address; this is not an Internet-edge NAT verdict".into());
        }
    }
    let mut session = Session {
        socket,
        timeout,
        token,
        emit,
        result,
    };
    session.discover(primary).await?;
    let mut result = session.result;
    result.duration_ms = start.elapsed().as_millis() as u64;
    result.summary = type_label(result.nat_type).into();
    result.confidence = if matches!(
        result.nat_type,
        NatType::FullCone | NatType::Symmetric | NatType::NoTranslation
    ) {
        "medium"
    } else {
        "low"
    }
    .into();
    result.status = match result.nat_type {
        NatType::Unknown => Status::Unknown,
        NatType::Symmetric | NatType::PortRestrictedCone | NatType::RestrictedCone => {
            Status::Warning
        }
        _ => Status::Normal,
    };
    result.evidence.push(format!(
        "Mapping: {:?}; filtering: {:?}; {} bounded probes",
        result.mapping,
        result.filtering,
        result.probes.len()
    ));
    Ok(result)
}

pub fn check(result: &NatResult) -> Check {
    let mut evidence = result.evidence.clone();
    evidence.extend(result.limitations.clone());
    evidence.extend(result.probes.iter().map(|p| {
        format!(
            "{} -> {}: {}; mapped={:?}; {} ms / {} attempts",
            p.stage, p.target, p.detail, p.mapped_endpoint, p.elapsed_ms, p.attempts
        )
    }));
    Check {
        id: "nat".into(),
        status: result.status,
        summary: result.summary.clone(),
        evidence,
        duration_ms: result.duration_ms,
    }
}
pub fn findings(result: &NatResult) -> Vec<Finding> {
    if !matches!(
        result.nat_type,
        NatType::Symmetric | NatType::PortRestrictedCone | NatType::RestrictedCone
    ) {
        return vec![];
    }
    vec![Finding {
        code: "nat_restrictive".into(), severity: Status::Warning,
        phenomenon: "当前 UDP NAT 行为可能限制部分 P2P 连接 / Observed UDP NAT behavior may constrain some P2P connections".into(),
        evidence: vec![result.summary.clone(), format!("Mapping: {:?}; filtering: {:?}", result.mapping, result.filtering)],
        possible_causes: vec!["NAT 或防火墙对目标地址/端口存在依赖；也可能受 VPN、TUN、UDP 代理影响 / Destination-dependent NAT or firewall behavior; VPN, TUN or UDP proxy paths may also contribute".into()],
        confidence: result.confidence.clone(),
        steps: vec!["在实际通信应用中验证连接结果，并使用其 ICE、TURN 或中继回退 / Verify the real application's connectivity and use its ICE/TURN or relay fallback".into(),
            "手动切换获授权的网络或 VPN 后重新测试，对比映射和过滤证据 / Retest after manually switching an authorized network or VPN and compare the mapping/filtering evidence".into(),
            "不要仅凭此结果关闭防火墙、启用 DMZ 或自动修改路由器 / Do not disable firewalls, enable DMZ or automatically reconfigure routers based only on this result".into()],
    }]
}
pub fn report(result: NatResult) -> Report {
    Report {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        started_at: (chrono::Utc::now()
            - chrono::Duration::milliseconds(result.duration_ms as i64))
        .to_rfc3339(),
        platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        duration_ms: result.duration_ms,
        cancelled: false,
        snapshot: Snapshot::default(),
        checks: vec![check(&result)],
        findings: findings(&result),
        score: None,
        score_basis:
            "NAT 行为不计入网络健康评分 / NAT behavior is excluded from the network health score"
                .into(),
        nat: Some(result),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classic_types_require_mapping_and_filtering_evidence() {
        use NatBehavior::*;
        assert_eq!(
            classify(false, EndpointIndependent, Unknown),
            NatType::Unknown
        );
        assert_eq!(
            classify(false, Unknown, AddressAndPortDependent),
            NatType::Unknown
        );
        assert_eq!(
            classify(false, EndpointIndependent, EndpointIndependent),
            NatType::FullCone
        );
        assert_eq!(
            classify(false, EndpointIndependent, AddressDependent),
            NatType::RestrictedCone
        );
        assert_eq!(
            classify(false, EndpointIndependent, AddressAndPortDependent),
            NatType::PortRestrictedCone
        );
        assert_eq!(
            classify(false, AddressDependent, Unknown),
            NatType::Symmetric
        );
        assert_eq!(
            classify(false, AddressAndPortDependent, EndpointIndependent),
            NatType::Symmetric
        );
        assert_eq!(classify(true, Unknown, Unknown), NatType::NoTranslation);
    }
    #[test]
    fn alternate_address_cannot_redirect_a_public_probe_into_a_lan() {
        let primary = "1.1.1.1:3478".parse().unwrap();
        for target in [
            "127.0.0.1:3479",
            "10.0.0.1:3479",
            "192.168.1.1:3479",
            "100.64.0.1:3479",
            "169.254.1.1:3479",
            "224.0.0.1:3479",
            "1.1.1.1:3479",
            "8.8.8.8:3478",
        ] {
            assert!(!allowed_alternate(primary, target.parse().unwrap()));
        }
        assert!(allowed_alternate(primary, "8.8.8.8:3479".parse().unwrap()));
    }
}
