# HoriNet Doctor delivery plan

Version: 0.1.0-beta.1. Independent implementation; no private HoriLink sources used.

## Progress

- [x] Confirm GitHub identity hhhoratioxu and absence of target repository.
- [x] Create independent public repository and local checkout.
- [x] Stage 1: native adapters, addresses, DHCP, routes, DNS, proxy, ICMP and connectivity.
- [x] Stage 2: evidence-based rules; offline tests for all requested fault scenarios.
- [x] Stage 3: React/Tauri UI, bounded monitoring, DNS/Proxy Doctor, traceroute, cancellation.
- [x] Stage 4: local history, redacted JSON/Markdown reports, settings and error handling.
- [ ] Stage 5: checks, Windows NSIS installer, docs, CI and beta release.
- [ ] Download/hash-check/install/launch released Windows build; record limits honestly.

## Environment and recovery

- Workspace: this repository only. GitHub: https://github.com/hhhoratioxu/HoriNet-Doctor
- Local Windows Rust stable 1.97.1, Node 24.18.0; MSVC linker absent at initial inspection.
- Use Windows GNU for local core verification. Official desktop installer uses MSVC on GitHub Actions.
- Never commit real diagnostic reports, addresses, local application data, tokens or caches.
- Update this file and docs/VALIDATION.md at stage boundaries and before interruptions.

## Acceptance boundaries

No synthetic network readings in the application. ICMP timeouts are inconclusive. DNS differences do not prove interference. All tests are explicitly distinguished from real network and desktop acceptance. Release remains unsigned and marked prerelease.

## Current checkpoint

Core now has 29 passing tests. Workspace Clippy, frontend typecheck/lint/build and desktop compile passed. Real CLI and desktop diagnosis completed all 16 checks. Desktop monitoring ran 70 real samples, paused at 39 then resumed; DNS Doctor ran both types/transports against two displayed endpoints. Native loopback IPv4/IPv6 traces and public UDP/TCP DNS ran. Google DoH succeeded; Cloudflare DoH failed with TLS handshake EOF on this host. All code is pushed to public main; Windows CI 38533306087 is running. History/export and official MSVC installer acceptance are next; see docs/VALIDATION.md.

First MSVC CI 38533306087 succeeded at 25d2aba. Installer/CLI/provenance downloaded and all SHA-256 verified; Authenticode status NotSigned. Native desktop JSON save dialog produced a 16-check report with no private IPv4/MAC values; summary copy verified in clipboard. Post-acceptance hardening adds partial-cancel/no-score, paused-task deadline, settings recovery, asynchronous history listing and validation tightening. Re-run final CI before tagging.
