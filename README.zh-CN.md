# HoriNet Doctor

**Network Diagnostics & Troubleshooting — 网络故障诊断与排查**

免费、开源、本地运行的 **Windows 10 / Windows 11 x64** 网络诊断桌面应用。真实检测由 Rust 实现，Tauri 2 + React 展示证据、可能原因和排查步骤。默认简体中文，支持 English、深浅主题。

[English](README.md) · [下载](https://github.com/hhhoratioxu/HoriNet-Doctor/releases) · [使用说明](docs/USER_GUIDE.md) · [本地编译](docs/BUILD.md) · [架构](docs/ARCHITECTURE.md)

![Windows 桌面实际运行的初始空状态](docs/images/desktop-empty.png)

上图为真实 Windows 桌面运行截图，未开始检测，不含虚构结果或私密网络信息。

## 下载与安装

前往 [v0.1.0-beta.1 预发布](https://github.com/hhhoratioxu/HoriNet-Doctor/releases/tag/v0.1.0-beta.1)，下载 Windows x64 `-setup.exe`。NSIS 安装到当前用户目录，应用默认无需管理员权限。需要 Microsoft Edge WebView2 Runtime；缺失时安装器提供微软引导安装，需要联网。

**首版为未签名 Beta，不是稳定版。** 没有代码签名证书，SmartScreen 可能提示；不声称通过签名认证。请使用 `Get-FileHash 安装包路径 -Algorithm SHA256`，与发布页 `SHA256SUMS.txt` 比较。`BUILD-windows-x64.json` 提供构建提交、目标平台和 CI 编号。也提供配套 CLI。

## 已实现功能

| 模块 | 真实能力 |
| --- | --- |
| 一键诊断 | 16 项检查：原生网卡、类型、IPv4/IPv6、DHCP/APIPA、网关与默认路由、DNS 配置、网关 ICMP、外部 IP TCP、系统解析、UDP DNS、HTTP/HTTPS、TLS 握手与证书、静态代理端口、Wi-Fi 信号及短期 RTT 采样；支持进度、取消和超时。 |
| 故障定位 | 网卡断连、DHCP 异常、默认路由缺失、IP 可达但 DNS 失败、特定站 HTTPS 失败、IPv4/IPv6 不对称、代理端口不可用、可能的代理冲突/门户认证、弱 Wi-Fi、延迟或波动偏高；结论包含证据、原因、置信度和建议。 |
| 质量监测 | ICMP 与 TCP 并列实时图，最小/最大/平均 RTT、ICMP 无应答率、相邻 RTT 波动、TCP 耗时、次数和时长；暂停/继续/停止。单目标，默认 1.5 秒，最多 10,000 样本或 24 小时，图表显示最近 180 次。 |
| DNS Doctor | 系统 DNS 识别、自定义域名和最多 3 个 DNS 端点；真实 UDP、TCP、可选 DoH 的 A/AAAA 查询；显示耗时、记录、截断和明确错误。 |
| Proxy Doctor | Windows 系统静态代理、PAC/WPAD 信息、经典 WinHTTP 配置、TUN/VPN 等适配器线索；主动 HTTP CONNECT/SOCKS5 无认证隧道测试；设置诊断基线，手动切换后对比。 |
| 网络工具 | IPv4/IPv6 原生 ICMP Traceroute、最多 30 跳、每跳 RTT/未知、取消、复制/导出；单 IP:端口 TCP 测试。Ping 在监测页。 |
| 历史和报告 | 每轮诊断保存结构化本机记录；浏览、删除、清空、保留数量设置；JSON/Markdown 导出及摘要复制，默认脱敏。 |
| 桌面界面 | 九个页面，深浅玻璃主题，简中/英文操作界面，系统标题栏与键盘焦点；系统背景材质效果取决于 Windows 版本，应用内部同时使用 CSS backdrop blur。 |

初始状态没有网络评分或伪造曲线。关系视图是**诊断依赖关系，不是物理网络拓扑**。评分公式为正常 100、警告 50、异常/未知 0，取已收集项目平均，同时显示可判断覆盖数；全部未知时不评分。该分数不表示全网可用率。技术原始证据保留 API / 协议英文描述。

## 安全、隐私与默认目标

- 只读，无遥测、无广告、不上传诊断报告、不使用付费 API、不调用公网 IP 查询服务，不自动修复系统。
- 检测由用户主动触发，可在设置查看和更改：IPv4 TCP `1.1.1.1:443`、IPv6 TCP `[2606:4700:4700::1111]:443`、解析域名 `example.com`、HTTP 门户测试 `www.msftconnecttest.com/connecttest.txt`、HTTPS `example.com`。DNS Doctor 默认展示 `1.1.1.1:53` / `8.8.8.8:53`，可选 DoH 为 Cloudflare。接收方能看到正常网络请求的来源 IP 和所查询域名。
- HTTP 用于观察门户重定向，为不含凭据的明文请求；不跟随重定向。HTTPS/TLS 校验证书，使用公共 WebPKI 根，绕过系统/应用 HTTP 代理；操作系统的 VPN/TUN 路由仍然生效。
- 不执行 PAC，不读取代理软件账号、订阅、密码或令牌；只检测用户指定或系统配置的代理端口，不扫描本机端口。
- 原始历史在 `%LOCALAPPDATA%\io.github.hhhoratioxu.horinet-doctor\history`，设置位于同级。历史包含敏感网络信息，依赖 Windows 账户权限，**未额外加密**。可以随时删除。
- 导出默认在所有层级隐藏公网/内网 IP、MAC、SSID、用户名、适配器名称及代理地址；勾选详细导出才包含原始信息。分享前仍应检查报告。
- 最小 Tauri Capabilities、自定义命令允许列表、无 Shell IPC、无前端网络访问、无自动 DNS/网卡/路由/防火墙/Winsock 修改。

## 结果解释与已知限制

- ICMP 超时可能只是过滤；无应答率不是确定的链路丢包率；未知跳点不证明线路中断。RTT 波动为相邻成功样本差值的平均绝对值。
- 单个 IP、端口、网站不可达不能证明全网故障。不同 DNS 的答案可因 CDN、地理位置、缓存和分流而变化，不能据此断言污染或劫持。
- Wi-Fi 未连接、WLAN 服务不可用或位置权限被拒绝时，SSID/信号显示未知；程序不调整系统隐私权限。DHCP 检查使用原生配置/地址证据，不续租。
- WinHTTP 仅读取经典默认配置，不能代表所有应用的有效代理；不执行 PAC，不支持需凭据的代理验证，不读取代理内部状态，不自动切换代理。
- 公共根证书不包含企业私有 CA；DoH 引导仍依赖系统 DNS，失败可能是连接、TLS、解析或服务本身问题。DNS 未实现 DNSSEC 验证、缓存和自动 TCP 回退，TCP 独立测试。
- 暂不支持输入带 scope 的 link-local IPv6；支持全局 IPv6 与回环。Traceroute 每跳一次，不做反向解析或地理定位。
- macOS/Linux 原生适配与安装包尚未实现。Windows 10 真机、不同 VPN、企业证书、各类门户等环境验收须与成功构建分开记录。

## 开发与开源

参见 [编译说明](docs/BUILD.md)、[实际验证记录](docs/VALIDATION.md)、[架构说明](docs/ARCHITECTURE.md)、[后续计划](docs/ROADMAP.md)。CI 包含格式、workspace Clippy/测试、依赖安装、TypeScript、lint、生产构建及 Windows x64 NSIS。标签发布会重新执行检查、生成校验和与溯源，再发布预发布版本。项目独立编写，不含私人 HoriLink 未公开源码。

[MIT 许可证](LICENSE) · [参与贡献](CONTRIBUTING.md) · [行为准则](CODE_OF_CONDUCT.md) · [安全报告](SECURITY.md)。请勿在公开 Issue 上传未脱敏诊断记录。
