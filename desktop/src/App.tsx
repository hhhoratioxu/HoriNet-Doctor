import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  ArrowRight,
  ArrowUpRight,
  Check,
  CheckCircle2,
  ChevronRight,
  Clock3,
  Copy,
  Download,
  Globe2,
  HeartPulse,
  History,
  Info,
  LayoutDashboard,
  LoaderCircle,
  LockKeyhole,
  Monitor as MonitorIcon,
  Network,
  Pause,
  Play,
  Radio,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Square,
  Terminal,
  Trash2,
  TriangleAlert,
  Wifi,
  X,
  XCircle,
  Zap,
  type LucideIcon,
} from "lucide-react";
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import {
  checkNames,
  defaults,
  type Check as NetworkCheck,
  type DoctorEvent,
  type DnsResult,
  type Hop,
  type NatProbe,
  type ProxyTest,
  type Report,
  type Sample,
  type Settings,
  type Snapshot,
  type Status,
} from "./models";
import NatResultView, { NatProbeTable } from "./NatResultView";

type Page =
  | "dashboard"
  | "diagnose"
  | "monitor"
  | "dns"
  | "proxy"
  | "nat"
  | "tools"
  | "history"
  | "settings"
  | "about";
const navigation: { id: Page; zh: string; en: string; icon: LucideIcon }[] = [
  { id: "dashboard", zh: "网络概览", en: "Dashboard", icon: LayoutDashboard },
  { id: "diagnose", zh: "智能诊断", en: "Diagnose", icon: HeartPulse },
  { id: "monitor", zh: "质量监测", en: "Monitor", icon: Activity },
  { id: "dns", zh: "DNS Doctor", en: "DNS Doctor", icon: Globe2 },
  { id: "proxy", zh: "Proxy Doctor", en: "Proxy Doctor", icon: ShieldCheck },
  { id: "nat", zh: "NAT Doctor", en: "NAT Doctor", icon: Network },
  { id: "tools", zh: "网络工具", en: "Tools", icon: Terminal },
  { id: "history", zh: "诊断历史", en: "History", icon: History },
  { id: "settings", zh: "设置", en: "Settings", icon: Settings2 },
  { id: "about", zh: "关于", en: "About", icon: Info },
];
const initialNames = Object.keys(checkNames).filter((id) => id !== "nat");
const formatMs = (n: number | null | undefined) =>
  n == null ? "—" : `${n.toFixed(1)} ms`;

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [settings, setSettings] = useState<Settings>(defaults);
  const [draft, setDraft] = useState<Settings>(defaults);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [report, setReport] = useState<Report | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [checks, setChecks] = useState<NetworkCheck[]>([]);
  const [job, setJob] = useState<{ id: string; kind: string } | null>(null);
  const jobRef = useRef<string | null>(null);
  const [completed, setCompleted] = useState(0);
  const [totalChecks, setTotalChecks] = useState(16);
  const [natReport, setNatReport] = useState<Report | null>(null);
  const [natProbes, setNatProbes] = useState<NatProbe[]>([]);
  const [natServer, setNatServer] = useState(defaults.stun_server);
  const [natConsent, setNatConsent] = useState(false);
  const [natCancelled, setNatCancelled] = useState(false);
  const [paused, setPaused] = useState(false);
  const [samples, setSamples] = useState<Sample[]>([]);
  const [hops, setHops] = useState<Hop[]>([]);
  const [dnsResults, setDnsResults] = useState<DnsResult[]>([]);
  const [proxyResult, setProxyResult] = useState<ProxyTest | null>(null);
  const [tcpResult, setTcpResult] = useState<number | null>(null);
  const [history, setHistory] = useState<Report[]>([]);
  const [detailed, setDetailed] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [target, setTarget] = useState("1.1.1.1");
  const [port, setPort] = useState(443);
  const [domain, setDomain] = useState("example.com");
  const [servers, setServers] = useState("1.1.1.1:53, 8.8.8.8:53");
  const [doh, setDoh] = useState(false);
  const [proxyEndpoint, setProxyEndpoint] = useState("127.0.0.1:7890");
  const [proxyProtocol, setProxyProtocol] = useState("http");
  const [traceTarget, setTraceTarget] = useState("example.com");
  const [ipv6, setIpv6] = useState(false);
  const [maxHops, setMaxHops] = useState(20);
  const [tcpEndpoint, setTcpEndpoint] = useState("1.1.1.1:443");
  const [baseline, setBaseline] = useState<Report | null>(null);
  const [inspecting, setInspecting] = useState(false);
  const en = settings.language === "en";
  const t = (zh: string, english: string) => (en ? english : zh);
  const local = (s: string) => {
    const pieces = s.split(" / ");
    return pieces.length > 1
      ? en
        ? pieces.slice(1).join(" / ")
        : pieces[0]
      : s;
  };
  const refreshHistory = useCallback(async () => {
    setHistory(await invoke<Report[]>("list_history"));
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let alive = true;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        let cfg = defaults;
        try {
          cfg = await invoke<Settings>("get_settings");
        } catch (e) {
          if (!alive) return;
          setError(
            `无法读取设置，请在设置页保存有效配置 / Cannot read settings; save valid settings to recover: ${String(e)}`,
          );
          setPage("settings");
        }
        if (!alive) return;
        setSettings(cfg);
        setDraft(cfg);
        setDomain(cfg.domain);
        setNatServer(cfg.stun_server);
        const off = await listen<DoctorEvent>(
          "doctor-event",
          ({ payload: e }) => {
            if (!alive || e.job_id !== jobRef.current) return;
            if (e.check)
              setChecks((c) => [
                ...c.filter((x) => x.id !== e.check!.id),
                e.check!,
              ]);
            if (e.completed != null) setCompleted(e.completed);
            if (e.total != null) setTotalChecks(e.total);
            if (e.report) {
              if (e.kind === "nat") setNatReport(e.report);
              else {
                setReport(e.report);
                setSnapshot(e.report.snapshot);
              }
              void refreshHistory().catch((err) => setError(String(err)));
            }
            if (e.nat_probe)
              setNatProbes((probes) => [...probes, e.nat_probe!]);
            if (e.sample) setSamples((s) => [...s.slice(-9999), e.sample!]);
            if (e.hop) setHops((h) => [...h, e.hop!]);
            if (e.dns) setDnsResults((r) => [...r, e.dns!]);
            if (e.proxy) setProxyResult(e.proxy);
            if (e.tcp_ms != null) setTcpResult(e.tcp_ms);
            if (e.done) {
              jobRef.current = null;
              setJob(null);
              setPaused(false);
              if (e.error && !e.error.includes("cancelled")) setError(e.error);
              if (e.kind === "nat" && e.error?.includes("cancelled"))
                setNatCancelled(true);
            }
          },
        );
        if (!alive) {
          off();
          return;
        }
        unlisten = off;
        setReady(true);
        await refreshHistory();
      } catch (e) {
        if (alive) setError(String(e));
      }
    })();
    return () => {
      alive = false;
      unlisten?.();
    };
  }, [refreshHistory]);
  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme;
    document.documentElement.lang = settings.language;
  }, [settings.theme, settings.language]);
  useEffect(() => {
    window.scrollTo({ top: 0, left: 0, behavior: "instant" });
  }, [page]);
  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(""), 4000);
    return () => window.clearTimeout(timer);
  }, [notice]);
  const act = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      return await fn();
    } catch (e) {
      setError(String(e));
    }
  };
  const start = async (
    kind: string,
    command: string,
    args: Record<string, unknown> = {},
  ) => {
    if (!ready || jobRef.current) return;
    setError("");
    const id = crypto.randomUUID();
    jobRef.current = id;
    setJob({ id, kind });
    try {
      await invoke(command, { jobId: id, ...args });
    } catch (e) {
      jobRef.current = null;
      setJob(null);
      setError(String(e));
    }
  };
  const diagnose = () => {
    if (job || !ready) return;
    setChecks([]);
    setReport(null);
    setCompleted(0);
    setTotalChecks(16 + Number(settings.nat_enabled));
    setDetailed(false);
    setPage("diagnose");
    void start("diagnose", "start_diagnosis");
  };
  const cancel = () => {
    if (job) void act(() => invoke("cancel_job", { jobId: job.id }));
  };
  const inspect = () =>
    void act(async () => {
      setInspecting(true);
      try {
        const s = await invoke<Snapshot>("inspect_network");
        setSnapshot(s);
        return s;
      } finally {
        setInspecting(false);
      }
    });
  const copy = async (value: string) => {
    await navigator.clipboard.writeText(value);
    setNotice(t("已复制", "Copied"));
  };
  const exportReport = (format: string, source: Report | null = report) =>
    source &&
    void act(async () => {
      const ok = await invoke<boolean>("export_report", {
        id: source.id,
        format,
        detailed,
      });
      if (ok) setNotice(t("报告已导出", "Report exported"));
    });
  const badge = (status: Status) => (
    <span className={`badge ${status}`}>
      {status === "normal" ? (
        <CheckCircle2 size={12} />
      ) : status === "error" ? (
        <XCircle size={12} />
      ) : status === "warning" ? (
        <TriangleAlert size={12} />
      ) : (
        <span className="tiny-dot" />
      )}
      {
        {
          normal: t("正常", "Normal"),
          error: t("异常", "Error"),
          warning: t("警告", "Warning"),
          unknown: t("未检测 / 无法判断", "Unknown"),
        }[status]
      }
    </span>
  );
  const empty = (title: string, body: string, icon: LucideIcon = Radio) => {
    const Icon = icon;
    return (
      <div className="empty">
        <div className="empty-icon">
          <Icon size={30} />
        </div>
        <h3>{title}</h3>
        <p>{body}</p>
      </div>
    );
  };
  const field = (label: string, node: ReactNode, help?: string) => (
    <label className="field">
      <span>{label}</span>
      {node}
      {help && <small>{help}</small>}
    </label>
  );
  const statusOf = (id: string): Status =>
    (report?.checks ?? checks).find((c) => c.id === id)?.status ?? "unknown";
  const metric = (label: string, value: ReactNode, sub?: string) => (
    <div className="metric glass">
      <span>{label}</span>
      <strong>{value}</strong>
      {sub && <small>{sub}</small>}
    </div>
  );
  const topology = () => (
    <section className="glass section">
      <div className="section-heading">
        <div>
          <h2>{t("诊断关系", "Diagnostic relationships")}</h2>
          <p>
            {t(
              "依据检测依赖显示，非真实物理网络拓扑",
              "Check dependencies, not a physical network map",
            )}
          </p>
        </div>
        <Network size={20} />
      </div>
      <div className="topology">
        {[
          {
            icon: MonitorIcon,
            label: t("本机设备", "Device"),
            id: "addresses",
          },
          { icon: Wifi, label: t("网络适配器", "Adapter"), id: "adapters" },
          { icon: Network, label: t("默认网关", "Gateway"), id: "routes" },
          { icon: Globe2, label: t("外网连接", "Internet test"), id: "ipv4" },
          { icon: Search, label: t("DNS 服务", "DNS"), id: "system_dns" },
          {
            icon: LockKeyhole,
            label: t("目标网站", "Target website"),
            id: "https",
          },
        ].map((n, i) => (
          <div className="topo-step" key={n.id}>
            <div className={`topo-node ${statusOf(n.id)}`}>
              <n.icon size={21} />
              <span>{n.label}</span>
              <span className="node-dot" />
            </div>
            {i < 5 && <ChevronRight className="topo-arrow" size={16} />}
          </div>
        ))}
      </div>
    </section>
  );
  const reportActions = (source: Report | null = report) =>
    source && (
      <div className="report-actions">
        <button
          className="button secondary"
          onClick={() => exportReport("json", source)}
        >
          <Download size={15} />
          JSON
        </button>
        <button
          className="button secondary"
          onClick={() => exportReport("md", source)}
        >
          <Download size={15} />
          Markdown
        </button>
        <button
          className="button secondary"
          onClick={() =>
            void act(async () =>
              copy(
                await invoke<string>("copy_summary", {
                  id: source.id,
                  detailed,
                }),
              ),
            )
          }
        >
          <Copy size={15} />
          {t("复制摘要", "Copy summary")}
        </button>
        <label className={`checkbox ${detailed ? "privacy-on" : ""}`}>
          <input
            type="checkbox"
            checked={detailed}
            onChange={(e) => setDetailed(e.target.checked)}
          />
          {t(
            "包含详细网络信息（可能泄露隐私）",
            "Include network details (privacy sensitive)",
          )}
        </label>
      </div>
    );
  const findings = () =>
    !report ? null : (
      <section className="glass section">
        <div className="section-heading">
          <div>
            <h2>{t("故障分析与建议", "Findings & next steps")}</h2>
            <p>
              {report.cancelled
                ? t(
                    "本轮已取消，以下仅基于已完成项目",
                    "Cancelled; conclusions use completed checks only",
                  )
                : t(
                    "每条判断都保留证据与置信度",
                    "Evidence and confidence accompany every finding",
                  )}
            </p>
          </div>
          <span className="count">{report.findings.length}</span>
        </div>
        {report.findings.length === 0 ? (
          <p className="muted">
            {t(
              "当前未触发故障规则。这不保证所有网络路径或网站都正常。",
              "No fault rule triggered. This does not guarantee every network path or website works.",
            )}
          </p>
        ) : (
          report.findings.map((f) => (
            <details className={`finding ${f.severity}`} key={f.code} open>
              <summary>
                <TriangleAlert size={17} />
                <strong>{local(f.phenomenon)}</strong>
                <span className="muted">
                  {t("置信度", "Confidence")}:{" "}
                  {local(
                    (
                      {
                        high: "高 / High",
                        medium: "中 / Medium",
                        low: "低 / Low",
                      } as Record<string, string>
                    )[f.confidence] ?? f.confidence,
                  )}
                </span>
              </summary>
              <div className="finding-body">
                <div>
                  <h4>{t("相关证据", "Evidence")}</h4>
                  {f.evidence.map((s, i) => (
                    <p className="mono" key={i}>
                      {s}
                    </p>
                  ))}
                </div>
                <div>
                  <h4>{t("可能原因", "Possible causes")}</h4>
                  {f.possible_causes.map((s, i) => (
                    <p key={i}>{local(s)}</p>
                  ))}
                  <h4>{t("推荐步骤", "Recommended steps")}</h4>
                  <ol>
                    {f.steps.map((s, i) => (
                      <li key={i}>{local(s)}</li>
                    ))}
                  </ol>
                </div>
              </div>
            </details>
          ))
        )}
      </section>
    );
  const checkList = () => (
    <div className="checks">
      {(report?.nat ||
      checks.some((c) => c.id === "nat") ||
      (!report && settings.nat_enabled)
        ? [...initialNames, "nat"]
        : initialNames
      ).map((id) => {
        const c = (report?.checks ?? checks).find((c) => c.id === id);
        return (
          <details className="check-row" key={id}>
            <summary>
              <span className={`check-icon ${c?.status ?? "unknown"}`}>
                {c?.status === "normal" ? (
                  <Check size={17} />
                ) : c?.status === "error" ? (
                  <X size={17} />
                ) : (
                  <Activity size={17} />
                )}
              </span>
              <span className="check-label">
                {checkNames[id][en ? 1 : 0]}
                <small>
                  {c ? `${c.duration_ms} ms` : t("等待检测", "Awaiting check")}
                </small>
              </span>
              {badge(c?.status ?? "unknown")}
              <ChevronRight size={15} />
            </summary>
            <div className="check-detail">
              <p>
                {c?.summary ??
                  t(
                    "开始诊断后显示真实结果",
                    "Run a diagnosis to see real results",
                  )}
              </p>
              {c?.evidence.map((e, i) => (
                <p className="mono" key={i}>
                  {e}
                </p>
              ))}
            </div>
          </details>
        );
      })}
    </div>
  );
  const activeAdapters =
    snapshot?.adapters.filter((a) => a.up && a.kind !== "loopback") ?? [];
  const values = useMemo(() => {
    const icmp = samples.flatMap((s) =>
      s.icmp.reached && s.icmp.rtt_ms != null ? [s.icmp.rtt_ms] : [],
    );
    const tcp = samples.flatMap((s) => (s.tcp_ms != null ? [s.tcp_ms] : []));
    const mean = icmp.length
      ? icmp.reduce((a, b) => a + b, 0) / icmp.length
      : null;
    const jitter =
      icmp.length > 1
        ? icmp.slice(1).reduce((sum, v, i) => sum + Math.abs(v - icmp[i]), 0) /
          (icmp.length - 1)
        : null;
    return {
      min: icmp.length ? Math.min(...icmp) : null,
      max: icmp.length ? Math.max(...icmp) : null,
      mean,
      jitter,
      loss: samples.length
        ? ((samples.length - icmp.length) / samples.length) * 100
        : null,
      tcp: tcp.length ? tcp.reduce((a, b) => a + b, 0) / tcp.length : null,
      received: icmp.length,
      tcpOk: tcp.length,
    };
  }, [samples]);
  const chart = samples.slice(-180).map((s) => ({
    time: (s.elapsed_ms / 1000).toFixed(1),
    icmp: s.icmp.reached ? s.icmp.rtt_ms : null,
    tcp: s.tcp_ms,
  }));

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <HeartPulse size={25} />
          </div>
          <div>
            <strong>
              HoriNet<span>Doctor</span>
            </strong>
            <small>NETWORK INTELLIGENCE</small>
          </div>
        </div>
        <div className="nav-caption">{t("工作空间", "WORKSPACE")}</div>
        <nav aria-label={t("主导航", "Main navigation")}>
          {navigation.slice(0, 8).map((n) => (
            <button
              key={n.id}
              className={`nav-item ${page === n.id ? "active" : ""}`}
              onClick={() => setPage(n.id)}
            >
              <n.icon size={19} />
              <span>{en ? n.en : n.zh}</span>
              {n.id === "diagnose" && job?.kind === "diagnose" && (
                <span className="live-dot" />
              )}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="privacy-card">
            <ShieldCheck size={18} />
            <div>
              <strong>
                {t("你的网络数据，留在本机", "Your data stays here")}
              </strong>
              <p>
                {t(
                  "本地诊断 · 无遥测 · 只读",
                  "Local · No telemetry · Read only",
                )}
              </p>
            </div>
          </div>
          {navigation.slice(8).map((n) => (
            <button
              key={n.id}
              className={`nav-item ${page === n.id ? "active" : ""}`}
              onClick={() => setPage(n.id)}
            >
              <n.icon size={18} />
              <span>{en ? n.en : n.zh}</span>
            </button>
          ))}
          <div className="version">
            <span className="tiny-dot" />
            v0.1.0-beta.2<span>MIT</span>
          </div>
        </div>
      </aside>
      <main className="main">
        <header className="topbar">
          <div className="breadcrumb">
            HoriNet Doctor
            <ChevronRight size={14} />
            <span>
              {en
                ? navigation.find((n) => n.id === page)?.en
                : navigation.find((n) => n.id === page)?.zh}
            </span>
          </div>
          <div className="topbar-right">
            <span className="local-pill">
              <span className="live-dot" />
              {t("本地运行", "Local first")}
            </span>
            <span className="beta-pill">BETA</span>
          </div>
        </header>
        <div className="content">
          {!isTauri() && (
            <div className="banner warning">
              <Info size={18} />
              {t(
                "浏览器预览没有系统检测权限。请运行 Windows 桌面应用获取真实数据。",
                "Browser preview has no native diagnostics. Run the Windows desktop application for real data.",
              )}
            </div>
          )}
          {error && (
            <div className="banner error" role="alert">
              <TriangleAlert size={18} />
              <span>{error}</span>
              <button
                aria-label={t("关闭错误", "Dismiss error")}
                onClick={() => setError("")}
              >
                <X size={16} />
              </button>
            </div>
          )}
          {notice && (
            <div className="toast" role="status">
              <CheckCircle2 size={17} />
              {notice}
            </div>
          )}
          {job &&
            page !== job.kind &&
            !(page === "tools" && ["trace", "tcp"].includes(job.kind)) && (
              <div className="banner">
                <LoaderCircle className="spin" size={16} />
                {t(
                  "检测正在后台运行；同一时间仅运行一个网络任务",
                  "A network job is running; one job at a time",
                )}
                <button onClick={cancel}>{t("停止", "Stop")}</button>
              </div>
            )}

          {page === "dashboard" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">YOUR NETWORK, UNDERSTOOD</div>
                  <h1>
                    {t(
                      "让网络问题，清晰可见。",
                      "Clarity for your connection.",
                    )}
                  </h1>
                  <p>
                    {t(
                      "从连接到网站，循着证据定位问题。",
                      "Follow the evidence, from your device to your destination.",
                    )}
                  </p>
                </div>
                <span className="subtle-chip">
                  <LockKeyhole size={14} />
                  {t("只读诊断", "Read-only diagnostics")}
                </span>
              </div>
              <section className="hero glass">
                <div className="hero-copy">
                  <span className="eyebrow">NETWORK HEALTH</span>
                  <h2>
                    {report
                      ? t("你的最近一次诊断", "Your latest diagnosis")
                      : t(
                          "连接状况，尚待检查",
                          "Your connection awaits a check",
                        )}
                  </h2>
                  <p>
                    {report
                      ? t(
                          `${report.checks.length} 项检查 · ${report.findings.length} 条分析 · ${report.cancelled ? "已取消" : "已完成"}`,
                          `${report.checks.length} checks · ${report.findings.length} findings · ${report.cancelled ? "Cancelled" : "Completed"}`,
                        )
                      : t(
                          "一次诊断，检查网卡、路由、DNS、代理与安全连接。没有检测前，不会给网络下结论。",
                          "Check adapters, routes, DNS, proxies and secure connections. No conclusions before measurements.",
                        )}
                  </p>
                  <button
                    className="button primary large"
                    disabled={!ready || !!job || inspecting}
                    onClick={diagnose}
                  >
                    <Zap size={19} />
                    {t("开始诊断", "Start diagnosis")}
                    <ArrowRight size={18} />
                  </button>
                  <small>
                    {t(
                      "无需管理员权限 · 不修改系统配置",
                      "No administrator privileges · No system changes",
                    )}
                  </small>
                </div>
                <div
                  className={`health-orbit ${report ? "measured" : ""}`}
                  style={
                    {
                      "--score": `${(report?.score ?? 0) * 3.6}deg`,
                    } as React.CSSProperties
                  }
                >
                  <div className="orbit-inner">
                    <HeartPulse size={30} />
                    <strong>
                      {report?.score ?? "—"}
                      {report?.score != null && <small>/100</small>}
                    </strong>
                    <span>
                      {report
                        ? t("本轮检查评分", "Check score")
                        : t("尚未诊断", "Not measured")}
                    </span>
                  </div>
                </div>
              </section>
              <div className="metrics-grid four">
                {metric(
                  t("网络适配器", "Connected adapters"),
                  snapshot ? activeAdapters.length : "—",
                  t("已连接 · 不含回环", "Connected · excludes loopback"),
                )}
                {metric(
                  t("默认路由", "Default routes"),
                  snapshot ? snapshot.routes.length : "—",
                  t("Windows 原生路由表", "Native Windows route table"),
                )}
                {metric(
                  t("域名解析", "System DNS"),
                  report
                    ? statusOf("system_dns") === "normal"
                      ? t("解析成功", "Resolved")
                      : t("查看证据", "See evidence")
                    : "—",
                  settings.domain,
                )}
                {metric(
                  t("诊断耗时", "Diagnosis duration"),
                  report ? `${(report.duration_ms / 1000).toFixed(1)} s` : "—",
                  report
                    ? new Date(report.started_at).toLocaleString()
                    : t("尚无检测数据", "No measurements yet"),
                )}
              </div>
              {report && (
                <p className="score-basis">
                  <Info size={14} />
                  {report.score_basis}
                </p>
              )}
              {topology()}
              <div className="quick-grid">
                {[
                  {
                    id: "monitor" as Page,
                    icon: Activity,
                    zh: "观察连接的每一次波动",
                    en: "See every fluctuation",
                    desc: t(
                      "实时 ICMP 与 TCP 双通道采样",
                      "Live ICMP and TCP measurements",
                    ),
                  },
                  {
                    id: "dns" as Page,
                    icon: Globe2,
                    zh: "给域名解析做个体检",
                    en: "Inspect your DNS",
                    desc: t(
                      "UDP · TCP · DoH · A / AAAA",
                      "UDP · TCP · DoH · A / AAAA",
                    ),
                  },
                  {
                    id: "proxy" as Page,
                    icon: ShieldCheck,
                    zh: "理清代理与连接",
                    en: "Understand your proxy",
                    desc: t(
                      "系统代理 · PAC · TUN / VPN",
                      "System proxy · PAC · TUN / VPN",
                    ),
                  },
                ].map((c) => (
                  <button
                    key={c.id}
                    className="quick-card glass"
                    onClick={() => setPage(c.id)}
                  >
                    <c.icon size={23} />
                    <strong>{en ? c.en : c.zh}</strong>
                    <span>{c.desc}</span>
                    <ArrowUpRight size={18} />
                  </button>
                ))}
              </div>
            </>
          )}

          {page === "diagnose" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">DIAGNOSE</div>
                  <h1>{t("循着证据，找到原因", "Follow the evidence")}</h1>
                  <p>
                    {t(
                      "16 项基础检查，可选 NAT 类型探测，保留判断依据与下一步。",
                      "16 core checks with optional NAT discovery, evidence and next steps.",
                    )}
                  </p>
                </div>
                {job?.kind === "diagnose" ? (
                  <button className="button danger" onClick={cancel}>
                    <Square size={15} />
                    {t("取消诊断", "Cancel")}
                  </button>
                ) : (
                  <button
                    className="button primary"
                    disabled={!ready || !!job || inspecting}
                    onClick={diagnose}
                  >
                    <Zap size={17} />
                    {t("开始诊断", "Start diagnosis")}
                  </button>
                )}
              </div>
              <div className="glass run-summary">
                <div>
                  <span
                    className={
                      job?.kind === "diagnose" ? "live-dot" : "tiny-dot"
                    }
                  />
                  <strong>
                    {job?.kind === "diagnose"
                      ? t("正在检查网络", "Checking your network")
                      : report
                        ? report.cancelled
                          ? t("本轮诊断已取消", "Diagnosis cancelled")
                          : t("诊断已完成", "Diagnosis complete")
                        : t("等待开始", "Ready when you are")}
                  </strong>
                  <span>
                    {completed} / {totalChecks}
                  </span>
                </div>
                <progress
                  max={totalChecks}
                  value={completed}
                  aria-label={t("诊断进度", "Diagnosis progress")}
                />
                <p>
                  {t(
                    "目标可在设置中查看和修改。NAT 探测默认关闭；同意并启用后，会通过所选 STUN 服务器探测公网映射 IP 与端口。",
                    "Review targets in Settings. NAT discovery is off by default; enabling it with consent discovers the public mapped IP and port through the selected STUN server.",
                  )}
                </p>
              </div>
              {topology()}
              <section className="glass section">
                <div className="section-heading">
                  <div>
                    <h2>{t("检查项目", "Checks")}</h2>
                    <p>
                      {t(
                        "展开查看原始测量与错误证据",
                        "Expand a check for measurements and errors",
                      )}
                    </p>
                  </div>
                  <Activity size={20} />
                </div>
                {checkList()}
              </section>
              {findings()}
              {report?.nat && (
                <NatResultView key={report.id} result={report.nat} en={en} />
              )}
              {reportActions()}
            </>
          )}

          {page === "monitor" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">CONNECTION QUALITY</div>
                  <h1>{t("每一刻，都有迹可循", "Every moment, measured")}</h1>
                  <p>
                    {t(
                      "ICMP 与 TCP 独立呈现，不把 Ping 超时当作全网断线。",
                      "ICMP and TCP are independent. Ping timeouts do not establish an Internet outage.",
                    )}
                  </p>
                </div>
                <span className="subtle-chip">
                  {settings.interval_ms} ms / {t("采样", "sample")}
                </span>
              </div>
              <section className="glass section">
                <div className="input-toolbar">
                  {field(
                    t("目标 IP / 域名", "Target IP / hostname"),
                    <input
                      maxLength={253}
                      value={target}
                      onChange={(e) => setTarget(e.target.value)}
                      disabled={!!job}
                    />,
                  )}
                  {field(
                    t("TCP 端口", "TCP port"),
                    <input
                      type="number"
                      min={1}
                      max={65535}
                      value={port}
                      onChange={(e) => setPort(+e.target.value)}
                      disabled={!!job}
                    />,
                  )}
                  <div className="button-group">
                    {job?.kind === "monitor" ? (
                      <>
                        <button
                          className="button secondary"
                          onClick={() =>
                            void act(async () => {
                              await invoke("pause_monitor", {
                                jobId: job.id,
                                paused: !paused,
                              });
                              setPaused(!paused);
                            })
                          }
                        >
                          {paused ? <Play size={16} /> : <Pause size={16} />}{" "}
                          {paused ? t("继续", "Resume") : t("暂停", "Pause")}
                        </button>
                        <button className="button danger" onClick={cancel}>
                          <Square size={15} />
                          {t("停止", "Stop")}
                        </button>
                      </>
                    ) : (
                      <button
                        className="button primary"
                        disabled={!ready || !!job || inspecting}
                        onClick={() => {
                          setSamples([]);
                          setPaused(false);
                          void start("monitor", "start_monitor", {
                            target,
                            port,
                          });
                        }}
                      >
                        <Play size={16} />
                        {t("开始监测", "Start monitor")}
                      </button>
                    )}
                  </div>
                </div>
                <p className="muted small">
                  {t(
                    "单目标、顺序采样；解析一次，最多 10,000 次或 24 小时。图表保留最近 180 次。",
                    "One target, sequential probes; resolves once; capped at 10,000 samples or 24 hours. Chart shows the latest 180.",
                  )}
                </p>
              </section>
              <div className="metrics-grid five">
                {metric(t("最小 RTT", "Minimum RTT"), formatMs(values.min))}
                {metric(t("最大 RTT", "Maximum RTT"), formatMs(values.max))}
                {metric(t("平均 RTT", "Average RTT"), formatMs(values.mean))}
                {metric(
                  t("ICMP 无应答率", "ICMP no-reply rate"),
                  values.loss == null ? "—" : `${values.loss.toFixed(1)}%`,
                  t(
                    "含过滤、超时；不等同线路丢包",
                    "Includes filtering/timeouts, not proven path loss",
                  ),
                )}
                {metric(
                  t("RTT 波动", "RTT variation"),
                  formatMs(values.jitter),
                  t("相邻成功样本差值均值", "Mean delta of adjacent replies"),
                )}
                {metric(
                  t("平均 TCP 连接", "Average TCP connect"),
                  formatMs(values.tcp),
                )}
                {metric(t("检测次数", "Samples"), samples.length || "—")}
                {metric(
                  t("ICMP 回应", "ICMP replies"),
                  samples.length ? `${values.received}/${samples.length}` : "—",
                )}
                {metric(
                  t("TCP 成功", "TCP successes"),
                  samples.length ? `${values.tcpOk}/${samples.length}` : "—",
                )}
                {metric(
                  t("持续时间", "Elapsed"),
                  samples.length
                    ? `${(samples.at(-1)!.elapsed_ms / 1000).toFixed(1)} s`
                    : "—",
                )}
              </div>
              <section className="glass section">
                <div className="section-heading">
                  <div>
                    <h2>{t("连接延迟", "Connection latency")}</h2>
                    <p>
                      {paused
                        ? t("已暂停", "Paused")
                        : job?.kind === "monitor"
                          ? t("实时采样中", "Live measurements")
                          : t(
                              "开始后显示真实测量数据",
                              "Real measurements appear after starting",
                            )}
                    </p>
                  </div>
                  <Activity size={20} />
                </div>
                {samples.length ? (
                  <div className="chart">
                    <ResponsiveContainer width="100%" height={300}>
                      <LineChart
                        data={chart}
                        margin={{ top: 15, right: 15, bottom: 5, left: 0 }}
                      >
                        <CartesianGrid
                          stroke="var(--line)"
                          strokeDasharray="3 6"
                          vertical={false}
                        />
                        <XAxis
                          dataKey="time"
                          tick={{ fill: "var(--muted)", fontSize: 11 }}
                          minTickGap={30}
                          unit="s"
                        />
                        <YAxis
                          tick={{ fill: "var(--muted)", fontSize: 11 }}
                          unit="ms"
                          width={65}
                        />
                        <Tooltip
                          contentStyle={{
                            background: "var(--panel-solid)",
                            border: "1px solid var(--line)",
                            borderRadius: 12,
                            color: "var(--text)",
                          }}
                        />
                        <Legend />
                        <Line
                          dataKey="icmp"
                          name="ICMP RTT"
                          stroke="#5eeacb"
                          strokeWidth={2}
                          dot={false}
                          isAnimationActive={false}
                          connectNulls={false}
                        />
                        <Line
                          dataKey="tcp"
                          name="TCP connect"
                          stroke="#8998ff"
                          strokeWidth={2}
                          dot={false}
                          isAnimationActive={false}
                          connectNulls={false}
                        />
                      </LineChart>
                    </ResponsiveContainer>
                  </div>
                ) : (
                  empty(
                    t("还没有延迟数据", "No latency data yet"),
                    t(
                      "设置一个目标，开始观察真实连接质量。",
                      "Choose a target and start observing connection quality.",
                    ),
                    Activity,
                  )
                )}
                {samples.length > 0 && (
                  <div className="evidence-strip mono">
                    {samples.at(-1)?.icmp.message} · TCP:{" "}
                    {samples.at(-1)?.tcp_error ??
                      formatMs(samples.at(-1)?.tcp_ms)}
                  </div>
                )}
              </section>
            </>
          )}

          {page === "dns" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">DNS DOCTOR</div>
                  <h1>
                    {t("让每一次解析都有答案", "Understand every lookup")}
                  </h1>
                  <p>
                    {t(
                      "比较协议、记录与服务器，区分解析错误和连接错误。",
                      "Compare protocols, records and resolvers; distinguish DNS and connection errors.",
                    )}
                  </p>
                </div>
                <button
                  className="button secondary"
                  disabled={!ready || !!job || inspecting}
                  onClick={inspect}
                >
                  <RefreshCw size={16} />
                  {t("读取系统配置", "Read system settings")}
                </button>
              </div>
              <section className="glass section">
                <h2>{t("当前系统 DNS", "System DNS servers")}</h2>
                <div className="server-chips">
                  {snapshot ? (
                    Array.from(
                      new Set(activeAdapters.flatMap((a) => a.dns_servers)),
                    ).map((s) => (
                      <button
                        key={s}
                        className="mono subtle-chip"
                        onClick={() =>
                          setServers(s.includes(":") ? `[${s}]:53` : `${s}:53`)
                        }
                      >
                        {s}
                        <ArrowRight size={13} />
                      </button>
                    ))
                  ) : (
                    <p className="muted">
                      {t(
                        "点击读取系统配置；也可直接输入自定义服务器。",
                        "Read system settings, or enter custom servers below.",
                      )}
                    </p>
                  )}
                </div>
                <div className="input-toolbar">
                  {field(
                    t("域名", "Domain"),
                    <input
                      value={domain}
                      maxLength={253}
                      onChange={(e) => setDomain(e.target.value)}
                      disabled={!!job}
                    />,
                  )}
                  {field(
                    t("DNS 服务器（最多 3 个）", "DNS servers (up to 3)"),
                    <input
                      value={servers}
                      maxLength={300}
                      onChange={(e) => setServers(e.target.value)}
                      disabled={!!job}
                    />,
                    t(
                      "逗号分隔，格式 IP:53 或 [IPv6]:53",
                      "Comma-separated IP:53 or [IPv6]:53",
                    ),
                  )}
                </div>
                <div className="toolbar-between">
                  <label className="checkbox">
                    <input
                      type="checkbox"
                      checked={doh}
                      onChange={(e) => setDoh(e.target.checked)}
                      disabled={!!job}
                    />
                    {t("同时测试 DoH", "Also test DoH")}{" "}
                    <span className="mono small">{settings.doh_url}</span>
                  </label>
                  {job?.kind === "dns" ? (
                    <button className="button danger" onClick={cancel}>
                      {t("取消", "Cancel")}
                    </button>
                  ) : (
                    <button
                      className="button primary"
                      disabled={!ready || !!job || inspecting}
                      onClick={() => {
                        setDnsResults([]);
                        void start("dns", "start_dns", {
                          input: {
                            domain,
                            servers: servers
                              .split(",")
                              .map((s) => s.trim())
                              .filter(Boolean),
                            doh,
                          },
                        });
                      }}
                    >
                      <Search size={16} />
                      {t("开始 DNS 检测", "Test DNS")}
                    </button>
                  )}
                </div>
                <p className="muted small">
                  {t(
                    "会向所选服务器发送该域名的 A / AAAA 查询；不修改系统 DNS。DoH 失败也可能来自其 HTTPS 连接或引导解析。",
                    "Sends A/AAAA queries to the selected resolvers; system DNS is unchanged. DoH failure may also be its HTTPS connection or bootstrap resolution.",
                  )}
                </p>
              </section>
              <section className="glass section">
                <div className="section-heading">
                  <div>
                    <h2>{t("解析结果对比", "Lookup comparison")}</h2>
                    <p>
                      {t(
                        "CDN、地理位置、缓存与分流都可能造成差异；不能据此断言污染或劫持。",
                        "CDNs, geography, caches and split DNS can cause differences; differences do not prove interception.",
                      )}
                    </p>
                  </div>
                  <Globe2 size={20} />
                </div>
                {dnsResults.length ? (
                  <div className="table-wrap">
                    <table>
                      <thead>
                        <tr>
                          {[
                            t("服务器 / 传输", "Server / transport"),
                            t("类型", "Type"),
                            t("耗时", "Time"),
                            t("记录 / 错误", "Records / error"),
                          ].map((h) => (
                            <th key={h}>{h}</th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {dnsResults.map((r, i) => (
                          <tr key={i}>
                            <td>
                              <span className="mono">{r.server}</span>
                              <small>
                                {r.transport.toUpperCase()} ·{" "}
                                {r.server.startsWith("[")
                                  ? "IPv6"
                                  : r.transport === "doh"
                                    ? "HTTPS"
                                    : "IPv4"}
                              </small>
                            </td>
                            <td>{r.record_type}</td>
                            <td>{formatMs(r.elapsed_ms)}</td>
                            <td className={r.error ? "text-error" : "mono"}>
                              {r.error ??
                                (r.truncated
                                  ? t(
                                      "响应截断，请比较 TCP",
                                      "Truncated response; compare TCP",
                                    )
                                  : r.records.join("\n") ||
                                    t(
                                      "成功响应，无该类型记录（NODATA）",
                                      "Successful response, no records of this type (NODATA)",
                                    ))}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                ) : (
                  empty(
                    t("准备检查解析链路", "Ready to inspect DNS"),
                    t(
                      "选择域名和服务器，测试 UDP、TCP 与可选 DoH。",
                      "Choose a domain and resolvers to test UDP, TCP and optional DoH.",
                    ),
                    Globe2,
                  )
                )}
              </section>
            </>
          )}

          {page === "proxy" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">PROXY DOCTOR</div>
                  <h1>
                    {t("理清连接经过的代理", "Understand the proxy path")}
                  </h1>
                  <p>
                    {t(
                      "只读取 Windows 公开配置；不读取代理应用账号、订阅或令牌。",
                      "Reads Windows public configuration, never proxy accounts, subscriptions or tokens.",
                    )}
                  </p>
                </div>
                <button
                  className="button secondary"
                  disabled={!ready || !!job || inspecting}
                  onClick={inspect}
                >
                  <RefreshCw size={16} />
                  {t("读取配置", "Read settings")}
                </button>
              </div>
              <div className="split-grid">
                <section className="glass section">
                  <h2>
                    {t("Windows 代理配置", "Windows proxy configuration")}
                  </h2>
                  {snapshot ? (
                    <dl className="data-list">
                      {[
                        [
                          t("系统静态代理", "System static proxy"),
                          snapshot.proxy.system_enabled
                            ? snapshot.proxy.system_server
                            : t("未启用", "Disabled"),
                        ],
                        [
                          "WinHTTP",
                          snapshot.proxy.winhttp_server ??
                            t("无静态配置", "No static setting"),
                        ],
                        [
                          "PAC",
                          snapshot.proxy.pac_url ??
                            t("未配置", "Not configured"),
                        ],
                        [
                          "WPAD",
                          snapshot.proxy.auto_detect
                            ? t("自动检测开启", "Autodetect enabled")
                            : t("自动检测关闭", "Autodetect disabled"),
                        ],
                      ].map(([k, v]) => (
                        <div key={k}>
                          <dt>{k}</dt>
                          <dd className="mono">{v}</dd>
                        </div>
                      ))}
                    </dl>
                  ) : (
                    empty(
                      t("配置尚未读取", "Settings not read yet"),
                      t(
                        "点击读取配置获取本机实际状态。",
                        "Read configuration to inspect the actual state.",
                      ),
                      ShieldCheck,
                    )
                  )}
                  <p className="muted small">
                    {t(
                      "PAC/WPAD 脚本只展示，不下载或执行。每应用代理及现代 WinHTTP 自动代理可能不在此快照中。",
                      "PAC/WPAD scripts are reported without downloading or executing them. Per-app proxies and modern WinHTTP automatic settings may differ.",
                    )}
                  </p>
                </section>
                <section className="glass section">
                  <h2>
                    {t("HTTP / SOCKS5 可用性", "HTTP / SOCKS5 availability")}
                  </h2>
                  {field(
                    t("代理监听地址", "Proxy endpoint"),
                    <input
                      value={proxyEndpoint}
                      maxLength={80}
                      onChange={(e) => setProxyEndpoint(e.target.value)}
                      disabled={!!job}
                    />,
                    t(
                      "IP:端口，不读取凭据",
                      "IP:port; no credentials are read",
                    ),
                  )}
                  {field(
                    t("协议", "Protocol"),
                    <select
                      value={proxyProtocol}
                      onChange={(e) => setProxyProtocol(e.target.value)}
                    >
                      <option value="http">HTTP CONNECT</option>
                      <option value="socks5">SOCKS5 (no authentication)</option>
                    </select>,
                  )}
                  <p className="muted small">
                    {t(
                      "主动建立协议握手及 CONNECT 隧道至",
                      "Actively negotiates a proxy CONNECT tunnel to",
                    )}{" "}
                    {settings.https_url}
                  </p>
                  <button
                    className="button primary"
                    disabled={!ready || !!job || inspecting}
                    onClick={() => {
                      setProxyResult(null);
                      void start("proxy", "start_proxy", {
                        endpoint: proxyEndpoint,
                        protocol: proxyProtocol,
                      });
                    }}
                  >
                    <Play size={16} />
                    {t("测试代理", "Test proxy")}
                  </button>
                  {proxyResult && (
                    <div className="result-box">
                      {badge(proxyResult.status)}
                      <p>{proxyResult.detail}</p>
                      <span className="mono">
                        TCP {formatMs(proxyResult.tcp_ms)}
                      </span>
                    </div>
                  )}
                </section>
              </div>
              <section className="glass section">
                <h2>
                  {t("可能的 TUN / VPN 适配器", "Possible TUN / VPN adapters")}
                </h2>
                <p className="muted">
                  {t(
                    "根据类型和描述识别，只作为线索，不推断软件内部状态。",
                    "Type/description heuristics are hints; they do not reveal application internals.",
                  )}
                </p>
                {snapshot ? (
                  snapshot.adapters
                    .filter((a) => a.virtual_adapter)
                    .map((a) => (
                      <div className="adapter-row" key={a.index}>
                        <Network size={18} />
                        <div>
                          <strong>{a.name}</strong>
                          <small>{a.description}</small>
                        </div>
                        {badge(a.up ? "normal" : "unknown")}
                      </div>
                    ))
                ) : (
                  <p className="muted">{t("尚未读取", "Not read yet")}</p>
                )}
                {snapshot &&
                  !snapshot.adapters.some((a) => a.virtual_adapter) && (
                    <p className="muted">
                      {t("当前没有匹配的适配器", "No matching adapters")}
                    </p>
                  )}
              </section>
              <section className="glass section">
                <div className="section-heading">
                  <div>
                    <h2>
                      {t("手动切换前后对比", "Compare a manual proxy change")}
                    </h2>
                    <p>
                      {t(
                        "先运行一次诊断并设为基线；在系统或代理软件中手动切换，再运行诊断。",
                        "Run a diagnosis and save a baseline; change proxy settings manually, then diagnose again.",
                      )}
                    </p>
                  </div>
                  <button
                    className="button secondary"
                    disabled={!report}
                    onClick={() => setBaseline(report)}
                  >
                    {t("设最近诊断为基线", "Use latest diagnosis as baseline")}
                  </button>
                </div>
                {baseline && report ? (
                  <div className="table-wrap">
                    <table>
                      <thead>
                        <tr>
                          <th>{t("检查", "Check")}</th>
                          <th>
                            {new Date(baseline.started_at).toLocaleTimeString()}
                          </th>
                          <th>
                            {new Date(report.started_at).toLocaleTimeString()}
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {["ipv4", "ipv6", "system_dns", "https", "proxy"].map(
                          (id) => (
                            <tr key={id}>
                              <td>{checkNames[id][en ? 1 : 0]}</td>
                              <td>
                                {badge(
                                  baseline.checks.find((c) => c.id === id)
                                    ?.status ?? "unknown",
                                )}
                              </td>
                              <td>
                                {badge(
                                  report.checks.find((c) => c.id === id)
                                    ?.status ?? "unknown",
                                )}
                              </td>
                            </tr>
                          ),
                        )}
                      </tbody>
                    </table>
                  </div>
                ) : (
                  <p className="muted">
                    {t(
                      "尚无对比基线。可以在历史页打开之前的诊断。",
                      "No baseline yet. Open an earlier diagnosis from History.",
                    )}
                  </p>
                )}
              </section>
            </>
          )}

          {page === "nat" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">NAT BEHAVIOR DISCOVERY</div>
                  <h1>
                    {t("看清当前 NAT 行为", "Understand your NAT behavior")}
                  </h1>
                  <p>
                    {t(
                      "真实 STUN 探测，分别判断映射与过滤；证据不足时保留未知。",
                      "Real STUN probes distinguish mapping from filtering and preserve unknown results.",
                    )}
                  </p>
                </div>
                <span className="subtle-chip">IPv4 · UDP · RFC 5780</span>
              </div>
              <section className="glass section">
                <h2>{t("检测目标与知情同意", "Target & consent")}</h2>
                <div className="input-toolbar">
                  {field(
                    t(
                      "STUN 服务器（域名或 IPv4:端口）",
                      "STUN server (hostname or IPv4:port)",
                    ),
                    <input
                      maxLength={259}
                      value={natServer}
                      disabled={!!job}
                      onChange={(e) => {
                        setNatServer(e.target.value);
                        setNatConsent(false);
                      }}
                    />,
                    t(
                      "需支持 RFC 5780 的双 IP / 双端口才能完整分类。可使用自建服务器。",
                      "Full classification requires RFC 5780 with two IPs and two ports. Custom servers are supported.",
                    ),
                  )}
                </div>
                <label className="checkbox nat-consent">
                  <input
                    type="checkbox"
                    checked={natConsent}
                    disabled={!!job}
                    onChange={(e) => setNatConsent(e.target.checked)}
                  />
                  <span>
                    {t(
                      "我同意连接上面的 STUN 服务器及其公布的备用端点，探测公网映射 IP 与端口。接收方会看到来源 IP，结果只保存在本机。",
                      "I consent to contact the STUN server above and its advertised alternate endpoints to discover my public mapped IP and port. Recipients see the source IP; results stay on this device.",
                    )}
                  </span>
                </label>
                <div className="nat-actions">
                  <button
                    className="button primary"
                    disabled={!ready || !!job || inspecting || !natConsent}
                    onClick={() => {
                      setNatReport(null);
                      setNatProbes([]);
                      setNatCancelled(false);
                      setDetailed(false);
                      void start("nat", "start_nat", {
                        server: natServer,
                        consent: natConsent,
                      });
                    }}
                  >
                    <Play size={17} />
                    {t("检测 NAT 类型", "Detect NAT behavior")}
                  </button>
                  {job?.kind === "nat" && (
                    <button className="button danger" onClick={cancel}>
                      <Square size={15} />
                      {t("取消检测", "Cancel")}
                    </button>
                  )}
                  <span className="muted small">
                    {t(
                      "最多 8 个顺序探测，单项最长 5 秒；不修改路由器、防火墙或系统网络配置。",
                      "At most 8 sequential probes, up to 5 seconds each; router, firewall and network settings are unchanged.",
                    )}
                  </span>
                </div>
              </section>
              {natReport?.nat ? (
                <>
                  <NatResultView
                    key={natReport.id}
                    result={natReport.nat}
                    en={en}
                  />
                  {reportActions(natReport)}
                </>
              ) : (
                <section className="glass section">
                  {job?.kind === "nat" ? (
                    <div className="section-heading">
                      <h2>
                        <LoaderCircle className="spin inline" size={17} />{" "}
                        {t("正在探测 NAT", "Probing NAT behavior")}
                      </h2>
                      <span className="count">{natProbes.length} / ≤8</span>
                    </div>
                  ) : natCancelled ? (
                    <p>
                      {t(
                        "检测已取消；以下仅为部分探测，不给出 NAT 类型结论。",
                        "Cancelled; probes below are partial evidence and do not classify NAT.",
                      )}
                    </p>
                  ) : (
                    empty(
                      t("等待一次真实检测", "Ready for a real measurement"),
                      t(
                        "确认目标并勾选同意后开始。NAT 类型不等于网络健康评分，也不保证游戏或 P2P 一定可用。",
                        "Confirm the target and consent to begin. NAT behavior is separate from health scoring and does not guarantee game or P2P connectivity.",
                      ),
                      Network,
                    )
                  )}
                  {natProbes.length > 0 && (
                    <NatProbeTable probes={natProbes} en={en} />
                  )}
                </section>
              )}
            </>
          )}

          {page === "tools" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">NETWORK TOOLKIT</div>
                  <h1>{t("深入一点，看得更清楚", "Look a little closer")}</h1>
                  <p>
                    {t(
                      "有限范围的单目标检测，不执行端口扫描。",
                      "Bounded single-target probes; no port scanning.",
                    )}
                  </p>
                </div>
              </div>
              <div className="split-grid">
                <section className="glass section">
                  <h2>TCP Connect</h2>
                  {field(
                    t("IP 与端口", "IP and port"),
                    <input
                      value={tcpEndpoint}
                      maxLength={80}
                      onChange={(e) => setTcpEndpoint(e.target.value)}
                    />,
                  )}
                  <button
                    className="button primary"
                    disabled={!ready || !!job || inspecting}
                    onClick={() => {
                      setTcpResult(null);
                      void start("tcp", "test_tcp", { endpoint: tcpEndpoint });
                    }}
                  >
                    <Play size={16} />
                    {t("测试连接", "Connect")}
                  </button>
                  {tcpResult != null && (
                    <div className="result-box">
                      <CheckCircle2 size={18} />
                      {formatMs(tcpResult)}
                    </div>
                  )}
                </section>
                <section className="glass section">
                  <h2>Ping / ICMP</h2>
                  <p className="muted">
                    {t(
                      "在质量监测页使用原生 ICMP；同时提供 TCP 对照，避免把 ICMP 过滤误判为断网。",
                      "Use native ICMP in Monitor, with TCP for comparison when ICMP is filtered.",
                    )}
                  </p>
                  <button
                    className="button secondary"
                    onClick={() => setPage("monitor")}
                  >
                    {t("打开实时 Ping", "Open live Ping")}
                    <ArrowUpRight size={15} />
                  </button>
                </section>
              </div>
              <section className="glass section">
                <div className="section-heading">
                  <div>
                    <h2>Traceroute</h2>
                    <p>
                      {t(
                        "Windows 原生 ICMP TTL / Hop Limit 探测；未知跳点不代表线路故障。",
                        "Native Windows ICMP TTL / hop-limit probes. Unknown hops do not establish line failure.",
                      )}
                    </p>
                  </div>
                  <Network size={20} />
                </div>
                <div className="input-toolbar">
                  {field(
                    t("目标 IP / 域名", "Target IP / hostname"),
                    <input
                      value={traceTarget}
                      maxLength={253}
                      disabled={!!job}
                      onChange={(e) => setTraceTarget(e.target.value)}
                    />,
                  )}
                  {field(
                    t("地址族", "IP family"),
                    <select
                      value={ipv6 ? "6" : "4"}
                      disabled={!!job}
                      onChange={(e) => setIpv6(e.target.value === "6")}
                    >
                      <option value="4">IPv4</option>
                      <option value="6">IPv6</option>
                    </select>,
                  )}
                  {field(
                    t("最大跳数", "Max hops"),
                    <input
                      type="number"
                      min={1}
                      max={30}
                      value={maxHops}
                      disabled={!!job}
                      onChange={(e) => setMaxHops(+e.target.value)}
                    />,
                  )}
                  {job?.kind === "trace" ? (
                    <button className="button danger" onClick={cancel}>
                      {t("取消", "Cancel")}
                    </button>
                  ) : (
                    <button
                      className="button primary"
                      disabled={!ready || !!job || inspecting}
                      onClick={() => {
                        setHops([]);
                        void start("trace", "start_trace", {
                          target: traceTarget,
                          ipv6,
                          maxHops,
                        });
                      }}
                    >
                      <Play size={16} />
                      {t("追踪路由", "Trace route")}
                    </button>
                  )}
                </div>
                {hops.length ? (
                  <>
                    <div className="table-wrap">
                      <table>
                        <thead>
                          <tr>
                            <th>Hop</th>
                            <th>{t("响应地址", "Response address")}</th>
                            <th>RTT</th>
                            <th>{t("状态", "Status")}</th>
                          </tr>
                        </thead>
                        <tbody>
                          {hops.map((h) => (
                            <tr key={h.ttl}>
                              <td>{h.ttl}</td>
                              <td className="mono">
                                {h.reply.address ??
                                  t("未知 / 未响应", "Unknown / no response")}
                              </td>
                              <td>{formatMs(h.reply.rtt_ms)}</td>
                              <td>
                                {h.reply.message} ({h.reply.status_code})
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                    <div className="report-actions">
                      <button
                        className="button secondary"
                        onClick={() =>
                          void act(() =>
                            copy(
                              hops
                                .map(
                                  (h) =>
                                    `${h.ttl}\t${detailed ? (h.reply.address ?? "unknown") : "[redacted]"}\t${formatMs(h.reply.rtt_ms)}\t${h.reply.status_code}`,
                                )
                                .join("\n"),
                            ),
                          )
                        }
                      >
                        <Copy size={15} />
                        {t("复制结果", "Copy results")}
                      </button>
                      <button
                        className="button secondary"
                        onClick={() =>
                          void act(() =>
                            invoke("export_trace", { hops, detailed }),
                          )
                        }
                      >
                        <Download size={15} />
                        {t("导出", "Export")}
                      </button>
                      <label className="checkbox">
                        <input
                          type="checkbox"
                          checked={detailed}
                          onChange={(e) => setDetailed(e.target.checked)}
                        />
                        {t(
                          "包含 IP 详情（敏感）",
                          "Include IP details (sensitive)",
                        )}
                      </label>
                    </div>
                  </>
                ) : (
                  empty(
                    t("路径尚未探测", "No path measured yet"),
                    t(
                      "最多 30 跳，每跳一次低频探测，可随时取消。",
                      "Up to 30 hops, one low-rate probe per hop; cancellable.",
                    ),
                    Network,
                  )
                )}
              </section>
            </>
          )}

          {page === "history" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">LOCAL HISTORY</div>
                  <h1>
                    {t("每一次检查，都有据可查", "A record of every diagnosis")}
                  </h1>
                  <p>
                    {t(
                      "历史只保存在本机。导出默认隐藏地址、SSID 与代理等隐私。",
                      "History stays on this device. Exports redact addresses, SSID, proxies and other private details by default.",
                    )}
                  </p>
                </div>
                <div className="button-group">
                  <button
                    className="button secondary"
                    disabled={!ready}
                    onClick={() => void act(refreshHistory)}
                  >
                    <RefreshCw size={15} />
                    {t("刷新", "Refresh")}
                  </button>
                  <button
                    className="button danger"
                    disabled={!history.length}
                    onClick={() => setDeleteId("all")}
                  >
                    <Trash2 size={15} />
                    {t("清空", "Clear")}
                  </button>
                </div>
              </div>
              <section className="glass section">
                {history.length ? (
                  <div className="history-list">
                    {history.map((r) => (
                      <div className="history-row" key={r.id}>
                        <div className="history-icon">
                          <Clock3 size={20} />
                        </div>
                        <button
                          className="history-main"
                          onClick={() => {
                            setDetailed(false);
                            if (r.nat && r.checks.length === 1) {
                              setNatReport(r);
                              setNatProbes(r.nat.probes);
                              setNatCancelled(false);
                              setPage("nat");
                            } else {
                              setReport(r);
                              setSnapshot(r.snapshot);
                              setChecks(r.checks);
                              setCompleted(r.checks.length);
                              setTotalChecks(r.nat ? 17 : 16);
                              setPage("diagnose");
                            }
                          }}
                        >
                          <strong>
                            {r.nat && r.checks.length === 1
                              ? "NAT Doctor · "
                              : ""}
                            {new Date(r.started_at).toLocaleString()}
                          </strong>
                          <small>
                            {r.checks.length} {t("项检查", "checks")} ·{" "}
                            {r.findings.length} {t("条分析", "findings")} ·{" "}
                            {(r.duration_ms / 1000).toFixed(1)} s{" "}
                            {r.cancelled ? `· ${t("已取消", "Cancelled")}` : ""}
                          </small>
                        </button>
                        <span className="history-score">
                          {r.score ?? "—"}
                          <small>/100</small>
                        </span>
                        <button
                          className="icon-button"
                          aria-label={t("删除记录", "Delete record")}
                          onClick={() => setDeleteId(r.id)}
                        >
                          <Trash2 size={16} />
                        </button>
                        <ChevronRight size={16} />
                      </div>
                    ))}
                  </div>
                ) : (
                  empty(
                    t("这里还没有诊断记录", "No diagnoses saved yet"),
                    t(
                      "运行一次诊断后，结果会自动保存在本机。",
                      "Run a diagnosis to save its results locally.",
                    ),
                    History,
                  )
                )}
              </section>
            </>
          )}

          {page === "settings" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">PREFERENCES</div>
                  <h1>{t("按你的方式，了解网络", "Make it your own")}</h1>
                  <p>
                    {t(
                      "目标与频率公开可见。所有设置只影响本程序。",
                      "Targets and intervals are visible. Settings affect this app only.",
                    )}
                  </p>
                </div>
                <button
                  className="button primary"
                  disabled={!ready || !!job}
                  onClick={() =>
                    void act(async () => {
                      await invoke("save_settings", { settings: draft });
                      setSettings(draft);
                      setNatServer(draft.stun_server);
                      setNatConsent(false);
                      setNotice(t("设置已保存", "Settings saved"));
                    })
                  }
                >
                  <Check size={16} />
                  {t("保存设置", "Save settings")}
                </button>
              </div>
              <section className="glass section">
                <h2>{t("外观与语言", "Appearance & language")}</h2>
                <div className="split-grid">
                  {field(
                    t("语言", "Language"),
                    <select
                      value={draft.language}
                      onChange={(e) =>
                        setDraft({ ...draft, language: e.target.value })
                      }
                    >
                      <option value="zh-CN">简体中文</option>
                      <option value="en">English</option>
                    </select>,
                  )}
                  {field(
                    t("主题", "Theme"),
                    <select
                      value={draft.theme}
                      onChange={(e) =>
                        setDraft({ ...draft, theme: e.target.value })
                      }
                    >
                      <option value="dark">
                        {t("深色玻璃", "Dark glass")}
                      </option>
                      <option value="light">
                        {t("浅色玻璃", "Light glass")}
                      </option>
                    </select>,
                  )}
                </div>
              </section>
              <section className="glass section">
                <h2>{t("检测目标", "Probe targets")}</h2>
                <p className="muted small">
                  {t(
                    "点击诊断会连接以下目标，接收方会看到来源 IP。本程序不上传诊断报告。NAT 公网映射探测单独征求同意，默认关闭。HTTP 门户测试为不含凭据的明文请求。",
                    "Starting a diagnosis contacts these targets, which see the source IP. Reports are never uploaded. Public mapped-IP discovery via STUN requires separate consent and is off by default. HTTP portal checks are plaintext and contain no credentials.",
                  )}
                </p>
                <div className="split-grid">
                  {(
                    [
                      "tcp_v4",
                      "tcp_v6",
                      "domain",
                      "http_url",
                      "https_url",
                      "doh_url",
                    ] as const
                  ).map((k) =>
                    field(
                      {
                        tcp_v4: "IPv4 TCP",
                        tcp_v6: "IPv6 TCP",
                        domain: t(
                          "系统 DNS 测试域名",
                          "System DNS test domain",
                        ),
                        http_url: "HTTP / Portal URL",
                        https_url: "HTTPS / TLS URL",
                        doh_url: "DoH URL",
                      }[k],
                      <input
                        value={draft[k]}
                        maxLength={k === "domain" ? 253 : 2048}
                        onChange={(e) =>
                          setDraft({ ...draft, [k]: e.target.value })
                        }
                      />,
                    ),
                  )}
                </div>
              </section>
              <section className="glass section">
                <h2>{t("NAT 类型探测（可选）", "Optional NAT discovery")}</h2>
                {field(
                  t("STUN 服务器", "STUN server"),
                  <input
                    maxLength={259}
                    value={draft.stun_server}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        stun_server: e.target.value,
                        nat_enabled: false,
                      })
                    }
                  />,
                )}
                <label className="checkbox nat-consent">
                  <input
                    type="checkbox"
                    checked={draft.nat_enabled}
                    onChange={(e) =>
                      setDraft({ ...draft, nat_enabled: e.target.checked })
                    }
                  />
                  <span>
                    {t(
                      "我同意在每次一键诊断中连接上面的 STUN 服务器及其备用端点，探测公网映射 IP 与端口，并将结果保存在本机历史。",
                      "I consent to contact the STUN server above and its alternate endpoints during each one-click diagnosis, discover the public mapped IP and port, and save results in local history.",
                    )}
                  </span>
                </label>
                <p className="muted small">
                  {t(
                    "默认关闭。修改 STUN 服务器会重置此同意；保存后生效。也可只在 NAT Doctor 页面主动测试一次。NAT 结果不影响健康评分。",
                    "Off by default. Changing the STUN server resets consent; save to apply. You can also run a single test from NAT Doctor. NAT results do not affect the health score.",
                  )}
                </p>
              </section>
              <section className="glass section">
                <h2>{t("采样与存储", "Sampling & storage")}</h2>
                <div className="three-grid">
                  {field(
                    t("单项超时（ms）", "Probe timeout (ms)"),
                    <input
                      type="number"
                      min={500}
                      max={10000}
                      value={draft.timeout_ms}
                      onChange={(e) =>
                        setDraft({ ...draft, timeout_ms: +e.target.value })
                      }
                    />,
                  )}
                  {field(
                    t("采样间隔（ms）", "Sample interval (ms)"),
                    <input
                      type="number"
                      min={1000}
                      max={60000}
                      value={draft.interval_ms}
                      onChange={(e) =>
                        setDraft({ ...draft, interval_ms: +e.target.value })
                      }
                    />,
                  )}
                  {field(
                    t("历史保留上限", "History retention"),
                    <input
                      type="number"
                      min={1}
                      max={500}
                      value={draft.retention}
                      onChange={(e) =>
                        setDraft({ ...draft, retention: +e.target.value })
                      }
                    />,
                  )}
                </div>
                <p className="muted small">
                  {t(
                    "本机历史包含详细网络信息，依赖 Windows 用户账户权限保护，未额外加密。可在历史页删除。",
                    "Local history contains detailed network information, protected by your Windows user account permissions without additional encryption. Delete it from History.",
                  )}
                </p>
              </section>
            </>
          )}

          {page === "about" && (
            <>
              <section className="about-hero glass">
                <div className="brand-mark large-logo">
                  <HeartPulse size={45} />
                </div>
                <div className="eyebrow">FREE · OPEN SOURCE · LOCAL FIRST</div>
                <h1>HoriNet Doctor</h1>
                <p>Network Diagnostics & Troubleshooting</p>
                <span className="beta-pill">v0.1.0-beta.2 · MIT</span>
                <button
                  className="button primary"
                  disabled={!ready}
                  onClick={() => void act(() => invoke("open_repository"))}
                >
                  GitHub <ArrowUpRight size={17} />
                </button>
              </section>
              <section className="glass section">
                <h2>
                  {t("认真对待每一条结论", "Every conclusion needs evidence")}
                </h2>
                <p>
                  {t(
                    "Windows 10 / 11 x64。独立使用 Rust、Tauri 2、React 与 TypeScript 开发。免费、开源、无广告、无遥测。默认不要求管理员权限，不修改系统网络设置。",
                    "Windows 10 / 11 x64. Independently built with Rust, Tauri 2, React and TypeScript. Free, open source, no ads or telemetry. No administrator privileges or network configuration changes by default.",
                  )}
                </p>
                <p>
                  {t(
                    "Beta 安装包未签名，Windows SmartScreen 可能提示。WebView2 Runtime 为运行依赖。macOS 与 Linux 原生适配尚未实现。",
                    "The beta installer is unsigned and Windows SmartScreen may warn. Requires WebView2 Runtime. Native macOS and Linux adapters are not implemented yet.",
                  )}
                </p>
                <p>
                  {t(
                    "ICMP 超时、路由跳点未知、不同 DNS 返回不同记录，都不能单独证明网络故障或劫持。企业证书与分流策略可能影响测试。",
                    "ICMP timeouts, unknown hops and differing DNS answers do not alone prove outages or interception. Enterprise certificates and split routing policies can affect tests.",
                  )}
                </p>
              </section>
            </>
          )}
          <footer>
            <ShieldCheck size={13} />
            <span>
              {t(
                "本地处理 · 只读诊断 · 无遥测",
                "Local processing · Read only · No telemetry",
              )}
            </span>
            <span>
              HoriNet Doctor <span className="footer-dot">·</span> Network
              Diagnostics & Troubleshooting
            </span>
          </footer>
        </div>
      </main>
      {deleteId && (
        <div className="modal-backdrop">
          <div
            className="modal glass"
            role="dialog"
            aria-modal="true"
            aria-labelledby="delete-title"
          >
            <Trash2 size={28} />
            <h2 id="delete-title">
              {deleteId === "all"
                ? t("清空所有本机诊断记录？", "Delete all local diagnoses?")
                : t("删除这条诊断记录？", "Delete this diagnosis?")}
            </h2>
            <p>
              {t(
                "此操作会永久删除对应的本机记录。已经导出的文件不会受影响。",
                "This permanently deletes the local records. Previously exported files are unaffected.",
              )}
            </p>
            <div className="button-group">
              <button
                autoFocus
                className="button secondary"
                onClick={() => setDeleteId(null)}
              >
                {t("取消", "Cancel")}
              </button>
              <button
                className="button danger"
                onClick={() =>
                  void act(async () => {
                    await invoke(
                      deleteId === "all" ? "clear_history" : "delete_history",
                      deleteId === "all" ? {} : { id: deleteId },
                    );
                    setDeleteId(null);
                    await refreshHistory();
                  })
                }
              >
                {t("确认删除", "Delete")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
