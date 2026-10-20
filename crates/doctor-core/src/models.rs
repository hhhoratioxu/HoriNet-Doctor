use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Normal,
    Error,
    Warning,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Adapter {
    pub name: String,
    pub description: String,
    pub index: u32,
    pub kind: String,
    pub up: bool,
    pub virtual_adapter: bool,
    pub dhcp: bool,
    pub addresses: Vec<String>,
    pub gateways: Vec<String>,
    pub dns_servers: Vec<String>,
    pub mac: String,
    pub signal_percent: Option<u32>,
    pub ssid: Option<String>,
    pub link_mbps: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Route {
    pub family: String,
    pub interface_index: u32,
    pub next_hop: String,
    pub metric: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyConfig {
    pub system_enabled: bool,
    pub system_server: Option<String>,
    pub pac_url: Option<String>,
    pub auto_detect: bool,
    pub winhttp_server: Option<String>,
    pub notes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snapshot {
    pub adapters: Vec<Adapter>,
    pub routes: Vec<Route>,
    pub proxy: ProxyConfig,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub id: String,
    pub status: Status,
    pub summary: String,
    pub evidence: Vec<String>,
    pub duration_ms: u64,
}
impl Check {
    pub fn new(
        id: &str,
        status: Status,
        summary: impl Into<String>,
        evidence: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            status,
            summary: summary.into(),
            evidence,
            duration_ms: 0,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub code: String,
    pub severity: Status,
    pub phenomenon: String,
    pub evidence: Vec<String>,
    pub possible_causes: Vec<String>,
    pub confidence: String,
    pub steps: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub id: String,
    pub started_at: String,
    pub platform: String,
    pub duration_ms: u64,
    pub cancelled: bool,
    pub snapshot: Snapshot,
    pub checks: Vec<Check>,
    pub findings: Vec<Finding>,
    pub score: Option<u8>,
    pub score_basis: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nat: Option<NatResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub theme: String,
    pub tcp_v4: String,
    pub tcp_v6: String,
    pub domain: String,
    pub http_url: String,
    pub https_url: String,
    pub doh_url: String,
    pub stun_server: String,
    pub nat_enabled: bool,
    pub timeout_ms: u64,
    pub interval_ms: u64,
    pub retention: usize,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "zh-CN".into(),
            theme: "dark".into(),
            tcp_v4: "1.1.1.1:443".into(),
            tcp_v6: "[2606:4700:4700::1111]:443".into(),
            domain: "example.com".into(),
            http_url: "http://www.msftconnecttest.com/connecttest.txt".into(),
            https_url: "https://example.com/".into(),
            doh_url: "https://cloudflare-dns.com/dns-query".into(),
            stun_server: "stun.miwifi.com:3478".into(),
            nat_enabled: false,
            timeout_ms: 3000,
            interval_ms: 1500,
            retention: 100,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub job_id: String,
    pub completed: usize,
    pub total: usize,
    pub check: Option<Check>,
    pub report: Option<Report>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingReply {
    pub address: Option<String>,
    pub rtt_ms: Option<f64>,
    pub status_code: u32,
    pub reached: bool,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub icmp: PingReply,
    pub tcp_ms: Option<f64>,
    pub tcp_error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hop {
    pub ttl: u8,
    pub reply: PingReply,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsResult {
    pub server: String,
    pub transport: String,
    pub record_type: String,
    pub records: Vec<String>,
    pub elapsed_ms: f64,
    pub error: Option<String>,
    pub truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyTest {
    pub protocol: String,
    pub endpoint: String,
    pub tcp_ms: Option<f64>,
    pub status: Status,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NatBehavior {
    #[default]
    Unknown,
    EndpointIndependent,
    AddressDependent,
    AddressAndPortDependent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NatType {
    #[default]
    Unknown,
    NoTranslation,
    FullCone,
    RestrictedCone,
    PortRestrictedCone,
    Symmetric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatProbe {
    pub stage: String,
    pub target: String,
    pub expected_source: String,
    pub response_source: Option<String>,
    pub mapped_endpoint: Option<String>,
    pub elapsed_ms: u64,
    pub attempts: u8,
    pub status: Status,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatResult {
    pub server: String,
    pub resolved_server: Option<String>,
    pub local_endpoint: Option<String>,
    pub mapped_endpoint: Option<String>,
    pub alternate_endpoint: Option<String>,
    pub mapping: NatBehavior,
    pub filtering: NatBehavior,
    pub nat_type: NatType,
    pub status: Status,
    pub confidence: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub limitations: Vec<String>,
    pub probes: Vec<NatProbe>,
    pub duration_ms: u64,
}
