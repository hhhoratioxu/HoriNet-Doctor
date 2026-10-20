HoriNet Doctor **v0.1.0-beta.2** adds real NAT behavior discovery to the unsigned Windows x64 beta.

- **NAT Doctor:** IPv4 UDP STUN / RFC 5780, separate mapping and filtering, evidence and confidence; conservative full-cone / restricted-cone / port-restricted / symmetric labels only when supported by observations.
- **Explicit consent:** configurable STUN target (default `stun.miwifi.com:3478`), no STUN traffic before consent; optional one-click integration defaults off. NAT does not affect the health score.
- **Usable workflow:** live probes, cancel/timeouts, endpoint masking, local history, redacted JSON/Markdown and summary exports. Original 16-check diagnosis and network tools remain available; consented runs add a 17th check.
- **Protocol checks:** 22 new offline/loopback tests plus the original 29 scenarios. CI runs all 51 tests, formatting, strict workspace Clippy, npm ci, TypeScript, ESLint, production build and MSVC NSIS packaging before publishing.

Download `HoriNet-Doctor_0.1.0-beta.2_windows-x64-setup.exe`. Windows 10/11 x64, WebView2 Runtime, current-user installation. **Unsigned prerelease**: no signing certificate; SmartScreen may warn. Verify `SHA256SUMS.txt`; `BUILD-windows-x64.json` identifies the source commit, target and build run.

NAT results describe one IPv4 UDP socket/path/time. Unsupported STUN servers or inconsistent observations remain unknown. Restrictive filtering is inferred from finite timeouts with low confidence. No console NAT ratings, CGNAT/double-NAT proof, P2P guarantee, router/firewall changes or IPv6 NAT discovery. Public STUN services may change. The current machine reached a complete 8-probe sequence; this is not acceptance across physical routers/carriers. See [NAT guide](https://github.com/hhhoratioxu/HoriNet-Doctor/blob/main/docs/NAT.md) and [validation](https://github.com/hhhoratioxu/HoriNet-Doctor/blob/main/docs/VALIDATION.md).

No telemetry, ads or report upload. Detailed history stays local without additional encryption; default exports redact endpoints and private evidence. ICMP timeouts/unknown hops are inconclusive; DNS differences do not prove interception. macOS/Linux native adapters, automatic repair and PAC/authenticated-proxy execution remain outside this beta.

[MIT source and documentation](https://github.com/hhhoratioxu/HoriNet-Doctor). Please report reproducible issues without unredacted network information.
