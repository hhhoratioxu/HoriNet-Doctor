use crate::models::*;

fn state(checks: &[Check], id: &str) -> Status {
    checks
        .iter()
        .find(|c| c.id == id)
        .map_or(Status::Unknown, |c| c.status)
}
pub fn evaluate(snapshot: &Snapshot, checks: &[Check]) -> Vec<Finding> {
    let mut out = Vec::new();
    let s = |id| state(checks, id);
    let mut add = |code: &str,
                   severity,
                   text: &str,
                   ids: &[&str],
                   causes: &[&str],
                   confidence: &str,
                   steps: &[&str]| {
        out.push(Finding {
            code: code.into(),
            severity,
            phenomenon: text.into(),
            evidence: checks
                .iter()
                .filter(|c| ids.contains(&c.id.as_str()))
                .flat_map(|c| {
                    std::iter::once(format!("{}: {:?} — {}", c.id, c.status, c.summary))
                        .chain(c.evidence.clone())
                })
                .collect(),
            possible_causes: causes.iter().map(|s| (*s).into()).collect(),
            confidence: confidence.into(),
            steps: steps.iter().map(|s| (*s).into()).collect(),
        })
    };
    if s("adapters") == Status::Error {
        add(
            "adapter_disconnected",
            Status::Error,
            "未发现已连接的网络适配器 / No connected network adapter",
            &["adapters", "addresses"],
            &["网线、无线开关或适配器状态 / Cable, radio or adapter state"],
            "high",
            &["检查网线、Wi-Fi 开关与系统适配器状态 / Check cable, Wi-Fi and adapter state"],
        );
    }
    if s("dhcp") == Status::Error {
        add("dhcp_failed", Status::Error, "DHCP 地址异常 / DHCP address unavailable", &["dhcp", "addresses"], &["DHCP 服务器不可达或租约获取失败 / DHCP server unreachable or lease failed"], "high", &["确认路由器 DHCP 服务与地址池 / Check router DHCP service and address pool", "重新连接网络后再次诊断；本程序不修改配置 / Reconnect and diagnose again; this app does not change settings"]);
    }
    if s("routes") == Status::Error {
        add("route_missing", Status::Error, "默认路由缺失 / Default route unavailable", &["routes"], &["网关配置缺失、VPN 路由或网络尚未就绪 / Missing gateway, VPN routing or interface not ready"], "high", &["检查系统网络设置中的网关与 VPN 路由 / Check gateway and VPN routes"]);
    }
    if s("ipv4") == Status::Normal && s("system_dns") == Status::Error {
        add("dns_failed", Status::Error, "IP 测试成功但系统域名解析失败 / IP test succeeds, system DNS fails", &["ipv4", "system_dns", "dns_udp"], &["系统 DNS 不可达、域名本身不存在或解析服务异常 / DNS unreachable, domain absent or resolver failure"], "medium", &["在 DNS Doctor 对比多个域名与服务器 / Compare multiple domains and resolvers in DNS Doctor", "不要仅凭结果差异判断劫持 / Differences alone do not establish interception"]);
    }
    if (s("ipv4") == Status::Normal || s("ipv6") == Status::Normal)
        && s("system_dns") == Status::Normal
        && s("https") == Status::Error
    {
        add("target_https_failed", Status::Warning, "目标网站 HTTPS 请求失败 / Target website HTTPS failed", &["system_dns", "https", "tls"], &["目标站点故障、证书、过滤或站点路由问题 / Target outage, certificate, filtering or destination route"], "medium", &["替换目标网站并比较 TLS 与 HTTP 证据 / Try another target and compare TLS/HTTP evidence", "单个站点失败不代表整体断网 / One failing site does not establish an Internet outage"]);
    }
    if s("ipv4") == Status::Normal && s("ipv6") == Status::Error {
        add("ipv6_unavailable", Status::Warning, "IPv4 可达，IPv6 测试失败 / IPv4 reachable, IPv6 test failed", &["ipv4", "ipv6", "routes"], &["网络可能未提供 IPv6，或该目标/端口不可达 / IPv6 not offered or this target/port unreachable"], "medium", &["确认 ISP 与路由器支持 IPv6；更换 IPv6 目标复测 / Check ISP/router IPv6 and retest another target"]);
    }
    if s("ipv6") == Status::Normal && s("ipv4") == Status::Error {
        add("ipv4_unavailable", Status::Warning, "IPv6 可达，IPv4 测试失败 / IPv6 reachable, IPv4 test failed", &["ipv4", "ipv6"], &["IPv6-only 网络、IPv4 路由或目标端口问题 / IPv6-only network, IPv4 route or target port"], "medium", &["检查 IPv4 地址和默认路由，并换目标复测 / Check IPv4 address/routes and retest"]);
    }
    if s("proxy") == Status::Error {
        add("proxy_unreachable", Status::Error, "已配置代理的端口无法连接 / Configured proxy port unreachable", &["proxy"], &["代理程序未运行、端口更改或被防火墙阻止 / Proxy stopped, port changed or firewall blocked"], "high", &["检查代理程序与 Windows 代理设置；本程序不会自动修改 / Check proxy app and Windows settings; no automatic changes"]);
    }
    if (snapshot.proxy.system_enabled
        || snapshot.adapters.iter().any(|a| a.up && a.virtual_adapter))
        && s("proxy") == Status::Normal
        && s("https") == Status::Error
    {
        add("proxy_conflict_possible", Status::Warning, "可能存在代理或路由配置冲突 / Possible proxy or routing conflict", &["proxy", "https", "routes"], &["代理端口可连接，但本程序直连失败；尚未证明因果关系 / Proxy port reachable but direct test fails; causality unproven"], "low", &["主动保存代理切换前后两次结果进行对比 / Save two results before/after a manual proxy change", "检查 PAC、TUN 与系统代理是否符合预期 / Review PAC, TUN and system proxy settings"]);
    }
    if s("http") == Status::Warning {
        add("portal_possible", Status::Warning, "可能需要门户认证或存在 HTTP 重定向 / Possible captive portal or HTTP redirect", &["http", "https"], &["HTTP 响应被重定向或已知测试内容不符；正常重定向也可能触发 / Redirect or unexpected connectivity body; legitimate redirects can also trigger"], "low", &["在浏览器打开网络登录页并检查地址 / Open the network sign-in page and verify its address", "不要忽略 TLS 证书警告 / Do not bypass TLS certificate errors"]);
    }
    if s("wifi") == Status::Warning {
        add(
            "wifi_weak",
            Status::Warning,
            "Wi-Fi 信号较弱 / Weak Wi-Fi signal",
            &["wifi"],
            &["距离、遮挡或无线干扰 / Distance, obstacles or interference"],
            "medium",
            &["靠近接入点或使用有线网络对比 / Move closer or compare with Ethernet"],
        );
    }
    if s("quality") == Status::Warning {
        add("quality_unstable", Status::Warning, "采样延迟或波动偏高 / Elevated sample latency or variation", &["quality"], &["短时拥塞、目标限速或无线波动 / Congestion, target rate limiting or wireless variation"], "low", &["在 Monitor 延长观察并对比其他目标 / Monitor longer and compare another target", "ICMP 不响应不等于网络故障 / Missing ICMP replies do not establish an outage"]);
    }
    if s("adapters") == Status::Error && s("ipv4") == Status::Error && s("ipv6") == Status::Error {
        add("offline_evidence", Status::Error, "多项证据表明当前无可用连接 / Multiple checks indicate no usable connection", &["adapters", "ipv4", "ipv6", "routes"], &["所有非回环适配器断开且两种 IP 测试失败 / No connected non-loopback adapter and both IP tests failed"], "high", &["恢复一个网络适配器的连接后重试 / Connect an adapter and retry"]);
    }
    out
}
pub fn score(checks: &[Check]) -> (Option<u8>, String) {
    // NAT behavior affects peer traversal, not general Internet health.
    let checks: Vec<_> = checks.iter().filter(|c| c.id != "nat").collect();
    let known = checks
        .iter()
        .filter(|c| c.status != Status::Unknown)
        .count();
    if known == 0 {
        return (None, "没有可判断的检测结果 / No conclusive checks".into());
    }
    let points: usize = checks
        .iter()
        .map(|c| match c.status {
            Status::Normal => 100,
            Status::Warning => 50,
            _ => 0,
        })
        .sum();
    (Some((points / checks.len()) as u8), format!("正常=100，警告=50，异常/未知=0；{known}/{} 项可判断。表示本轮检查覆盖与结果，不是全网可用率 / normal=100, warning=50, error/unknown=0; coverage {known}/{}; not Internet availability",checks.len(),checks.len()))
}
