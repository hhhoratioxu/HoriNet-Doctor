export type Status = "normal" | "error" | "warning" | "unknown";
export interface Adapter {
  name: string;
  description: string;
  index: number;
  kind: string;
  up: boolean;
  virtual_adapter: boolean;
  dhcp: boolean;
  addresses: string[];
  gateways: string[];
  dns_servers: string[];
  mac: string;
  signal_percent: number | null;
  ssid: string | null;
  link_mbps: number;
}
export interface Route {
  family: string;
  interface_index: number;
  next_hop: string;
  metric: number;
}
export interface ProxyConfig {
  system_enabled: boolean;
  system_server: string | null;
  pac_url: string | null;
  auto_detect: boolean;
  winhttp_server: string | null;
  notes: string[];
}
export interface Snapshot {
  adapters: Adapter[];
  routes: Route[];
  proxy: ProxyConfig;
  warnings: string[];
}
export interface Check {
  id: string;
  status: Status;
  summary: string;
  evidence: string[];
  duration_ms: number;
}
export interface Finding {
  code: string;
  severity: Status;
  phenomenon: string;
  evidence: string[];
  possible_causes: string[];
  confidence: string;
  steps: string[];
}
export interface Report {
  schema_version: number;
  id: string;
  started_at: string;
  platform: string;
  duration_ms: number;
  cancelled: boolean;
  snapshot: Snapshot;
  checks: Check[];
  findings: Finding[];
  score: number | null;
  score_basis: string;
}
export interface Settings {
  language: string;
  theme: string;
  tcp_v4: string;
  tcp_v6: string;
  domain: string;
  http_url: string;
  https_url: string;
  doh_url: string;
  timeout_ms: number;
  interval_ms: number;
  retention: number;
}
export interface PingReply {
  address: string | null;
  rtt_ms: number | null;
  status_code: number;
  reached: boolean;
  message: string;
}
export interface Sample {
  sequence: number;
  elapsed_ms: number;
  icmp: PingReply;
  tcp_ms: number | null;
  tcp_error: string | null;
}
export interface Hop {
  ttl: number;
  reply: PingReply;
}
export interface DnsResult {
  server: string;
  transport: string;
  record_type: string;
  records: string[];
  elapsed_ms: number;
  error: string | null;
  truncated: boolean;
}
export interface ProxyTest {
  protocol: string;
  endpoint: string;
  tcp_ms: number | null;
  status: Status;
  detail: string;
}
export interface DoctorEvent {
  job_id: string;
  kind: string;
  done: boolean;
  completed: number | null;
  total: number | null;
  check: Check | null;
  report: Report | null;
  sample: Sample | null;
  hop: Hop | null;
  dns: DnsResult | null;
  proxy: ProxyTest | null;
  tcp_ms: number | null;
  error: string | null;
}
export const defaults: Settings = {
  language: "zh-CN",
  theme: "dark",
  tcp_v4: "1.1.1.1:443",
  tcp_v6: "[2606:4700:4700::1111]:443",
  domain: "example.com",
  http_url: "http://www.msftconnecttest.com/connecttest.txt",
  https_url: "https://example.com/",
  doh_url: "https://cloudflare-dns.com/dns-query",
  timeout_ms: 3000,
  interval_ms: 1500,
  retention: 100,
};
export const checkNames: Record<string, [string, string]> = {
  adapters: ["网络适配器", "Network adapters"],
  addresses: ["IPv4 / IPv6 地址", "Local addresses"],
  dhcp: ["DHCP 地址获取", "DHCP addressing"],
  routes: ["默认网关与路由", "Gateway & routes"],
  dns_config: ["系统 DNS 配置", "System DNS settings"],
  gateway: ["网关连通性", "Gateway reachability"],
  ipv4: ["外部 IPv4 TCP", "External IPv4 TCP"],
  ipv6: ["外部 IPv6 TCP", "External IPv6 TCP"],
  system_dns: ["系统域名解析", "System resolution"],
  dns_udp: ["UDP DNS 查询", "UDP DNS query"],
  http: ["HTTP / 门户检测", "HTTP / portal check"],
  https: ["HTTPS 访问", "HTTPS request"],
  tls: ["TLS 握手与证书", "TLS & certificate"],
  proxy: ["系统代理端口", "System proxy port"],
  wifi: ["Wi-Fi 信号", "Wi-Fi signal"],
  quality: ["延迟与波动", "Latency & variation"],
};
