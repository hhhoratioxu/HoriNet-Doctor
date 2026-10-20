# Roadmap and known gaps

## Follow-up beta work

- Extend NAT evidence across real routers, mobile/carrier/VPN paths, more RFC 5780 servers and IPv6. IPv4 UDP NAT Doctor is implemented in beta.2; console ratings, CGNAT proof, hairpin/lifetime tests and router modification are not implemented.

- Broader Windows 10/11 physical-device acceptance across Wi-Fi, enterprise CA, NRPT/split DNS, IPv6-only networks, portals, VPNs and authenticated proxies.
- Scoped IPv6 interface-aware probes, more traceroute attempts and optional reverse lookup.
- Structured error codes and deeper effective WinHTTP/NRPT inspection; keep application internals private.
- Optional import of system enterprise trust roots with explicit behavior/documentation, DNSSEC validation and automatic DNS TCP fallback.
- Improve full bilingual technical evidence and screen reader/focus behavior across native dialogs.
- Code-signing when a certificate and secure signing process are available. Independent security review.

## Later

- macOS and Linux platform implementations and installers with separate runtime acceptance.
- Longer observation summaries and network-change detection.
- Repair design only after explicit per-action confirmation, instructions and risk explanation; no automatic DNS/adapter/route/firewall/Winsock writes in 0.1.

No unsupported platform, paid API, telemetry, broad network scanning or device fingerprint collection is planned. This file describes future work, not existing capabilities.
