import { useState } from "react";
import { Eye, EyeOff, Network, ShieldCheck } from "lucide-react";
import type { NatBehavior, NatProbe, NatResult } from "./models";

const stages: Record<string, [string, string]> = {
  binding: ["建立基线", "Baseline binding"],
  filter_change_ip_port: ["更换响应 IP 与端口", "Change response IP & port"],
  filter_change_port: ["更换响应端口", "Change response port"],
  binding_control: ["复查基线", "Baseline control"],
  mapping_other_ip: ["比较备用 IP 映射", "Compare alternate-IP mapping"],
  mapping_other_ip_port: [
    "比较备用 IP / 端口映射",
    "Compare alternate IP / port",
  ],
  alternate_port_control: ["验证备用端口可用", "Alternate-port control"],
  binding_final: ["验证映射稳定性", "Final stability check"],
};
const behaviors: Record<NatBehavior, [string, string]> = {
  unknown: ["无法判断", "Unknown"],
  endpoint_independent: ["与目标无关", "Endpoint independent"],
  address_dependent: ["依赖目标 IP", "Address dependent"],
  address_and_port_dependent: [
    "依赖目标 IP 和端口",
    "Address and port dependent",
  ],
};
const local = (value: string, en: boolean) => {
  const split = value.indexOf(" / ");
  return split < 0
    ? value
    : en
      ? value.slice(split + 3)
      : value.slice(0, split);
};

export function NatProbeTable({
  probes,
  en,
  reveal = false,
}: {
  probes: NatProbe[];
  en: boolean;
  reveal?: boolean;
}) {
  const t = (zh: string, english: string) => (en ? english : zh);
  const endpoint = (value: string | null) =>
    value ? (reveal ? value : "••••••") : "—";
  return (
    <div className="table-wrap">
      <table className="nat-probes">
        <thead>
          <tr>
            <th>{t("探测步骤", "Probe")}</th>
            <th>{t("响应", "Response")}</th>
            <th>{t("映射端点", "Mapped endpoint")}</th>
            <th>{t("时间 / 发送次数", "Time / attempts")}</th>
          </tr>
        </thead>
        <tbody>
          {probes.map((probe, i) => (
            <tr key={`${probe.stage}-${i}`}>
              <td>
                <strong>
                  {stages[probe.stage]?.[en ? 1 : 0] ?? probe.stage}
                </strong>
                <details>
                  <summary>{t("查看证据", "View evidence")}</summary>
                  <p>{probe.detail}</p>
                  <p className="mono">
                    {t("目标", "Target")}: {endpoint(probe.target)}
                    <br />
                    {t("预期响应源", "Expected source")}:{" "}
                    {endpoint(probe.expected_source)}
                    <br />
                    {t("实际响应源", "Actual source")}:{" "}
                    {endpoint(probe.response_source)}
                  </p>
                </details>
              </td>
              <td>
                <span className={`badge ${probe.status}`}>
                  {probe.status === "normal"
                    ? t("已响应", "Replied")
                    : t("无有效证据", "Inconclusive")}
                </span>
              </td>
              <td className="mono">{endpoint(probe.mapped_endpoint)}</td>
              <td>
                {probe.elapsed_ms} ms · {probe.attempts}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export default function NatResultView({
  result,
  en,
}: {
  result: NatResult;
  en: boolean;
}) {
  const [reveal, setReveal] = useState(false);
  const t = (zh: string, english: string) => (en ? english : zh);
  const endpoint = (value: string | null) =>
    value ? (reveal ? value : "••••••") : t("无法取得", "Unavailable");
  return (
    <>
      <section className="glass section nat-result" aria-live="polite">
        <div className="section-heading">
          <div>
            <div className="eyebrow">NAT · IPv4 UDP</div>
            <h2>{local(result.summary, en)}</h2>
            <p>
              {t("置信度", "Confidence")}:{" "}
              {result.confidence === "medium"
                ? t("中", "Medium")
                : t("低", "Low")}{" "}
              · {(result.duration_ms / 1000).toFixed(1)} s ·{" "}
              {t("不计入网络健康评分", "Excluded from the health score")}
            </p>
          </div>
          <Network size={23} />
        </div>
        <div className="split-grid nat-behaviors">
          <div>
            <small>{t("映射行为", "Mapping behavior")}</small>
            <strong>{behaviors[result.mapping][en ? 1 : 0]}</strong>
            <p>
              {t(
                "访问不同服务器时，外部 IP 与端口是否保持一致。",
                "Whether the external endpoint stays the same across destinations.",
              )}
            </p>
          </div>
          <div>
            <small>{t("过滤行为", "Filtering behavior")}</small>
            <strong>{behaviors[result.filtering][en ? 1 : 0]}</strong>
            <p>
              {t(
                "未主动访问的外部 IP / 端口能否向本端口发回响应。",
                "Whether replies from previously uncontacted addresses or ports arrive.",
              )}
            </p>
          </div>
        </div>
        <div className="section-heading">
          <h3>{t("地址与证据", "Endpoints & evidence")}</h3>
          <button
            className="button secondary"
            onClick={() => setReveal(!reveal)}
          >
            {reveal ? <EyeOff size={15} /> : <Eye size={15} />}
            {reveal
              ? t("隐藏地址", "Hide addresses")
              : t("显示地址", "Reveal addresses")}
          </button>
        </div>
        <div className="nat-endpoints">
          <p>
            {t("所选服务器", "Selected server")}:{" "}
            <code>{endpoint(result.server)}</code>
          </p>
          <p>
            {t("本地 UDP 端点", "Local UDP endpoint")}:{" "}
            <code>{endpoint(result.local_endpoint)}</code>
          </p>
          <p>
            {t("服务器看到的映射", "Server-observed endpoint")}:{" "}
            <code>{endpoint(result.mapped_endpoint)}</code>
          </p>
          <p>
            {t("备用服务器端点", "Alternate endpoint")}:{" "}
            <code>{endpoint(result.alternate_endpoint)}</code>
          </p>
        </div>
        <NatProbeTable probes={result.probes} en={en} reveal={reveal} />
        <details className="nat-evidence">
          <summary>{t("额外判断依据", "Additional evidence")}</summary>
          {result.evidence.map((e, i) => (
            <p key={i} className="mono">
              {reveal
                ? local(e, en)
                : t(
                    "显示地址后可查看详细证据",
                    "Reveal addresses to view detailed evidence",
                  )}
            </p>
          ))}
        </details>
      </section>
      <section className="glass section">
        <div className="section-heading">
          <h2>{t("如何理解结果", "How to interpret this result")}</h2>
          <ShieldCheck size={19} />
        </div>
        {result.limitations.map((limitation, i) => (
          <p key={i} className="muted">
            {local(limitation, en)}
          </p>
        ))}
        <p>
          {t(
            "可在切换网络或手动切换 VPN 后重新测试。对称型或受限过滤可能影响部分 P2P；实际应用应通过 ICE / TURN 或中继验证和回退。",
            "Retest after switching networks or manually changing VPN state. Symmetric mapping or restrictive filtering can affect some P2P paths; real applications should verify connectivity with ICE / TURN and support a relay fallback.",
          )}
        </p>
      </section>
    </>
  );
}
