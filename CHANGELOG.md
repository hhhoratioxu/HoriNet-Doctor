# Changelog

## 0.1.0-beta.2 — 2026-10-20

- Add NAT Doctor with real IPv4 UDP STUN Binding and RFC 5780 mapping/filtering discovery, evidence, conservative classic NAT labels and confidence.
- Require explicit consent before any STUN lookup/traffic; server is configurable, and optional one-click NAT discovery defaults off. Changing the configured server resets consent in the UI.
- Show live probe progress, bounded retransmissions, cancellation, masked endpoint display, local NAT history and redacted JSON/Markdown/summary exports. NAT behavior is excluded from network health scoring.
- Detect unsupported servers, ignored CHANGE-REQUEST, 420 errors, missing alternate listeners and unstable baselines without inventing a NAT type or an Internet outage.
- Add 22 offline/loopback protocol, classification, timeout/cancellation, ordering, migration, scoring and privacy tests; retain the original 29 scenarios.

IPv4 UDP only. Restricted filtering is inferred from finite timeouts and remains low confidence. Results describe one socket/path/time, do not establish CGNAT/double NAT, and are not console NAT 1/2/3 ratings or P2P guarantees. Unsigned Windows beta.

## 0.1.0-beta.1 — 2026-10-14

Initial independent Windows x64 beta.

- Rust native adapters/routes/WLAN/proxy/IPv4+IPv6 ICMP and real connectivity/TLS tests.
- Evidence-based diagnosis rules with confidence, causes, suggestions and offline scenarios.
- Tauri 2 / React desktop with nine pages, live bounded ICMP/TCP monitoring, UDP/TCP/DoH DNS Doctor, Proxy Doctor, native traceroute and TCP tools.
- Local JSON history/settings, cancellation/progress, redacted JSON/Markdown/summary/trace export.
- Simplified Chinese and English controls, dark/light glass styling and keyboard focus.
- MIT source, contribution/security policies, Windows checks and NSIS beta release with SHA-256/provenance.

Unsigned prerelease. macOS/Linux native implementations, automatic repair, PAC execution, authenticated proxy testing, DNSSEC and physical topology discovery are not included. See [validation](docs/VALIDATION.md) and [roadmap](docs/ROADMAP.md) for the exact acceptance boundaries.
