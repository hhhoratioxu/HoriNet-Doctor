use crate::models::Report;
use regex::Regex;
use serde_json::Value;

pub fn exported(report: &Report, detailed: bool) -> Report {
    if detailed {
        return report.clone();
    }
    let mut secrets = Vec::new();
    for a in &report.snapshot.adapters {
        secrets.extend([a.name.clone(), a.description.clone(), a.mac.clone()]);
        secrets.extend(a.ssid.clone());
        secrets.extend(a.addresses.clone());
        secrets.extend(a.gateways.clone());
        secrets.extend(a.dns_servers.clone());
    }
    secrets.extend(report.snapshot.proxy.system_server.clone());
    secrets.extend(report.snapshot.proxy.winhttp_server.clone());
    secrets.extend(report.snapshot.proxy.pac_url.clone());
    if let Ok(user) = std::env::var("USERNAME") {
        if user.len() >= 2 {
            secrets.push(user);
        }
    }
    secrets.retain(|s| !s.is_empty());
    secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
    secrets.dedup();
    let patterns: Vec<_> = secrets
        .iter()
        .filter_map(|s| Regex::new(&format!("(?i){}", regex::escape(s))).ok())
        .collect();
    let ipv4 = Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").expect("static regex");
    let ipv6 = Regex::new(r"(?i)(?:[0-9a-f]{0,4}:){2,}[0-9a-f:.%a-z0-9_-]*").expect("static regex");
    let mac = Regex::new(r"(?i)\b(?:[0-9a-f]{2}[:-]){5}[0-9a-f]{2}\b").expect("static regex");
    let urls = Regex::new(r#"(?i)(?:https?|socks5h?)://[^\s"<>]+"#).expect("static regex");
    let users = Regex::new(r"(?i)[a-z]:[\\/]Users[\\/][^\\/\s]+").expect("static regex");
    fn scrub(v: &mut Value, patterns: &[Regex], generic: &[Regex], ipv6: &Regex) {
        match v {
            Value::String(s) => {
                for regex in patterns.iter().chain(generic) {
                    *s = regex.replace_all(s, "[redacted]").into_owned();
                }
                *s = ipv6
                    .replace_all(s, |caps: &regex::Captures<'_>| {
                        let candidate = caps[0]
                            .split('%')
                            .next()
                            .unwrap_or_default()
                            .trim_end_matches('.');
                        if candidate.parse::<std::net::Ipv6Addr>().is_ok() {
                            "[redacted]".to_string()
                        } else {
                            caps[0].to_string()
                        }
                    })
                    .into_owned();
            }
            Value::Array(a) => {
                for v in a {
                    scrub(v, patterns, generic, ipv6)
                }
            }
            Value::Object(o) => {
                for v in o.values_mut() {
                    scrub(v, patterns, generic, ipv6)
                }
            }
            _ => {}
        }
    }
    // Work on every string, including summaries, nested evidence and error text.
    let mut value = serde_json::to_value(report).expect("Report serialization is infallible");
    scrub(&mut value, &patterns, &[mac, urls, users, ipv4], &ipv6);
    serde_json::from_value(value).expect("redaction preserves Report structure")
}
pub fn json(report: &Report, detailed: bool) -> crate::Result<String> {
    Ok(serde_json::to_string_pretty(&exported(report, detailed))?)
}
fn safe(s: &str) -> String {
    s.replace(['\r', '\n'], " ")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
pub fn markdown(report: &Report, detailed: bool) -> String {
    let r = exported(report, detailed);
    let mut out = format!("# HoriNet Doctor\n\nNetwork Diagnostics & Troubleshooting — v0.1.0-beta.1\n\n- Time (UTC): {}\n- Platform: {}\n- Duration: {} ms\n- Cancelled: {}\n- Privacy: {}\n- Score: {}\n- Basis: {}\n\n",r.started_at,r.platform,r.duration_ms,r.cancelled,if detailed {"Detailed data explicitly included"} else {"Redacted by default"},r.score.map_or("Unknown".into(),|s| s.to_string()),safe(&r.score_basis));
    out.push_str("## Checks\n\n");
    for c in &r.checks {
        out.push_str(&format!(
            "### {} — {:?}\n\n{} ({} ms)\n\n",
            safe(&c.id),
            c.status,
            safe(&c.summary),
            c.duration_ms
        ));
        for e in &c.evidence {
            out.push_str(&format!("- {}\n", safe(e)));
        }
        out.push('\n');
    }
    out.push_str("## Findings\n\n");
    if r.findings.is_empty() {
        out.push_str(
            "No rule triggered. This is not a guarantee that every network path works.\n\n",
        );
    }
    for f in &r.findings {
        out.push_str(&format!(
            "### {}\n\nConfidence: {}\n\n",
            safe(&f.phenomenon),
            safe(&f.confidence)
        ));
        for e in &f.evidence {
            out.push_str(&format!("- Evidence: {}\n", safe(e)));
        }
        for c in &f.possible_causes {
            out.push_str(&format!("- Possible cause: {}\n", safe(c)));
        }
        for s in &f.steps {
            out.push_str(&format!("- Step: {}\n", safe(s)));
        }
        out.push('\n');
    }
    out.push_str("## Native snapshot\n\n```json\n");
    // Escape backticks so user-controlled interface labels cannot end the fence.
    out.push_str(
        &serde_json::to_string_pretty(&r.snapshot)
            .unwrap_or_default()
            .replace('`', "\\u0060"),
    );
    out.push_str("\n```\n\nDiagnostic relationships are not a physical network map. ICMP timeouts and unknown hops are inconclusive. DNS differences do not prove interference.\n");
    out
}
pub fn summary(report: &Report, detailed: bool) -> String {
    let r = exported(report, detailed);
    format!(
        "HoriNet Doctor | {} | {} ms | score {}\n{}\n{}",
        r.started_at,
        r.duration_ms,
        r.score.map_or("unknown".into(), |s| s.to_string()),
        r.score_basis,
        r.findings
            .iter()
            .map(|f| format!("{} [{}]", f.phenomenon, f.confidence))
            .collect::<Vec<_>>()
            .join("\n")
    )
}
