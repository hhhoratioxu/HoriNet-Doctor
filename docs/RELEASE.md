HoriNet Doctor's first unsigned **Windows x64 beta**: real local network diagnostics with Rust and Tauri 2.

- 16-check diagnosis with explainable findings, progress/cancel/timeouts.
- Live ICMP/TCP monitoring, UDP/TCP/DoH DNS Doctor, Proxy Doctor, IPv4/IPv6 traceroute and TCP tools.
- Local history/settings, redacted JSON/Markdown/summary exports, Simplified Chinese/English, dark/light interface.

Download `HoriNet-Doctor_0.1.0-beta.1_windows-x64-setup.exe`. Requires Windows 10/11 x64 and Microsoft Edge WebView2 Runtime. Current-user installation; no administrator privileges for diagnosis.

**Unsigned prerelease:** no signing certificate, SmartScreen may warn. Verify `SHA256SUMS.txt`; `BUILD-windows-x64.json` identifies the exact source/build. CI validates formatting, workspace Clippy/tests, TypeScript/lint/build and MSVC NSIS packaging. See [validation](https://github.com/hhhoratioxu/HoriNet-Doctor/blob/main/docs/VALIDATION.md) for separate real runtime evidence.

ICMP timeouts/unknown hops are inconclusive; DNS differences do not prove interception. Direct HTTPS/TLS bypasses HTTP proxies and uses public roots. PAC/authenticated proxy execution, scoped link-local IPv6 inputs, automatic system repair and macOS/Linux installers are not included. WLAN data can be unavailable without location permission. No public-IP lookup service, telemetry or report upload. Raw local history is not additionally encrypted; exports default to redacted.

Please report reproducible issues without sharing unredacted network data. [MIT source and usage/build documentation](https://github.com/hhhoratioxu/HoriNet-Doctor).
