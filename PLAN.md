# HoriNet Doctor delivery plan

Version: 0.1.0-beta.1. Independent implementation; no private HoriLink sources used.

## Progress

- [x] Confirm GitHub identity hhhoratioxu and absence of target repository.
- [x] Create independent public repository and local checkout.
- [x] Stage 1: native adapters, addresses, DHCP, routes, DNS, proxy, ICMP and connectivity.
- [x] Stage 2: evidence-based rules; offline tests for all requested fault scenarios.
- [ ] Stage 3: React/Tauri UI, bounded monitoring, DNS/Proxy Doctor, traceroute, cancellation.
- [ ] Stage 4: local history, redacted JSON/Markdown reports, settings and error handling.
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

Core initial 26 tests, workspace Clippy, frontend typecheck/lint/build and desktop compile passed. Real CLI diagnosis and desktop diagnosis completed all 16 checks. Native loopback IPv4/IPv6 traces and public UDP/TCP DNS ran; Cloudflare DoH failed on this host and remains a recorded failed transport test. Desktop GUI acceptance is in progress. Official MSVC CI/release is next; see docs/VALIDATION.md.
