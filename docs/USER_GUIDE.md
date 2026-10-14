# 使用说明 / User guide

1. **网络概览 / Dashboard**：首次没有评分。点击「开始诊断」主动运行默认目标，或先去设置调整目标。评分和判断必须结合覆盖数及原始证据阅读。
2. **智能诊断 / Diagnose**：查看 16 个检查进度，随时取消；展开每一项看测量、错误、时间。完成或取消后的记录保存在本机。故障规则展示证据、可能原因、置信度和建议。关系图仅为诊断依赖。
3. **质量监测 / Monitor**：输入单个 IP/域名和 TCP 端口。每次同时测试 ICMP/TCP，提供曲线与统计。可暂停、继续、停止。ICMP 无应答率包含过滤/超时，不能直接当作物理链路丢包；TCP 成功可作为独立证据。解析只在开始时执行，默认可能优先使用排序后的 IPv4 地址；只想测 IPv6 时请输入 IPv6 字面量。
4. **DNS Doctor**：点击读取配置查看系统 DNS（不包含全部 NRPT/分流策略）。输入域名、逗号分隔的 `IP:端口` / `[IPv6]:端口`，最多 3 个。A/AAAA 分别用 UDP/TCP 查询，可勾选 DoH。每个结果显示时间、记录或错误；NODATA 与 NXDOMAIN、超时和连接错误区分。DoH 默认使用系统 DNS 进行引导解析。不会修改系统 DNS。
5. **Proxy Doctor**：主动读取系统静态代理、经典 WinHTTP、PAC/WPAD 与可能虚拟网卡。自定义监听端点进行 HTTP CONNECT/SOCKS5 无认证测试，目标为设置中的 HTTPS 地址；成功只代表隧道协商。可将最近诊断设为基线，手动切换代理后再诊断比较。程序不自动启停代理，不执行 PAC，不读取其他软件私密配置。
6. **网络工具 / Tools**：TCP 单 IP:端口测试；Ping 使用 Monitor；Traceroute 选择 IPv4/IPv6 和最多 30 跳，可取消、复制和导出。未知跳点是未响应，不是已确认线路故障。
7. **历史 / History**：打开旧记录后可在诊断页导出 JSON/Markdown、复制摘要。默认脱敏；「包含详细网络信息」会显式输出地址等敏感信息。支持删除单条或清空；界面会要求确认。原始本机历史没有额外加密。
8. **设置 / Settings**：配置简中/英文、深浅主题、各检测目标、单项超时（500–10,000 ms）、采样间隔（1,000–60,000 ms）、历史上限（1–500）。保存后生效，正在运行的任务使用启动时设置。设置不影响系统网络配置。
9. **关于 / About**：版本、MIT、GitHub、安全与平台限制。

All measurements are real and user initiated. A browser preview cannot run native probes. The Windows desktop provides the actual functionality. Probe targets can see ordinary request source IPs; diagnostic reports are not uploaded. Public-IP discovery is not implemented, so there is no hidden IP lookup service.

## Common questions

- **Ping 失败但网站可打开？** ICMP 可能被限速或过滤，比较 TCP、HTTPS 和不同目标。
- **IPv6 失败？** 某些网络没有 IPv6。单个端点失败不证明整个 IPv6 网络不可用，换目标复测。
- **代理开着但直连失败？** 本程序 HTTP/TLS 默认直连；代理 Doctor 独立做端口/协议测试。操作系统 TUN/VPN 路由仍会影响直连。
- **Wi-Fi 信号未知？** 检查是否连接 Wi-Fi、WLAN 服务及 Windows 位置权限。本程序不会帮你切换系统权限。
- **HTTPS/TLS 证书失败？** 检查时间和目标。企业私有根 CA 未导入公共 WebPKI 根集合。不要忽略证书错误或直接宣称遭到劫持。
- **为什么正常网络不是 100 分？** 未知项目计 0，警告计 50；评分刻画本次检查结果及覆盖度，不是全网可用率。没有可判断结果时不评分。

## CLI

```text
horinet snapshot
horinet diagnose
horinet tcp 1.1.1.1:443
horinet ping example.com
horinet dns example.com 1.1.1.1:53 udp
horinet dns example.com 1.1.1.1:53 tcp
horinet dns example.com https://cloudflare-dns.com/dns-query doh
horinet trace example.com 4
horinet trace example.com 6
```

CLI diagnosis JSON is always redacted. `snapshot` prints counts only. `Ctrl+C` cancels. The desktop additionally provides user-selected detailed export, monitoring, history and settings.
