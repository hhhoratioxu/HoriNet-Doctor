# HoriNet Doctor delivery plan

Version: 0.1.0-beta.1. Independent implementation; no private HoriLink sources used.

## Progress

- [x] Confirm GitHub identity hhhoratioxu and absence of target repository.
- [x] Create independent public repository and local checkout.
- [x] Stage 1: native adapters, addresses, DHCP, routes, DNS, proxy, ICMP and connectivity.
- [x] Stage 2: evidence-based rules; offline tests for all requested fault scenarios.
- [x] Stage 3: React/Tauri UI, bounded monitoring, DNS/Proxy Doctor, traceroute, cancellation.
- [x] Stage 4: local history, redacted JSON/Markdown reports, settings and error handling.
- [x] Stage 5: checks, Windows NSIS installer, docs, CI and beta release.
- [x] Download/hash-check/install/launch released Windows build; record limits honestly.

## Environment and recovery

- Workspace: this repository only. GitHub: https://github.com/hhhoratioxu/HoriNet-Doctor
- Local Windows Rust stable 1.97.1, Node 24.18.0; MSVC linker absent at initial inspection.
- Use Windows GNU for local core verification. Official desktop installer uses MSVC on GitHub Actions.
- Never commit real diagnostic reports, addresses, local application data, tokens or caches.
- Update this file and docs/VALIDATION.md at stage boundaries and before interruptions.

## Acceptance boundaries

No synthetic network readings in the application. ICMP timeouts are inconclusive. DNS differences do not prove interference. All tests are explicitly distinguished from real network and desktop acceptance. Release remains unsigned and marked prerelease.

## Current checkpoint

Delivery completed on 2026-10-14. Public release: https://github.com/hhhoratioxu/HoriNet-Doctor/releases/tag/v0.1.0-beta.1

- Released code: `a9d1bcf20b9f250cdfc9246f2721d55fa1d9c5a0`; tag is immutable in this task.
- Final CI 38535015886 and Release 38535978289 both succeeded. 29 tests passed with zero failures; full frontend/Rust checks and MSVC NSIS packaging passed.
- Downloaded all 4 Release assets; every SHA-256 matched. Installer: 3,749,375 bytes; Authenticode NotSigned.
- Installed final Release with exit code 0 to `%LOCALAPPDATA%\Programs\HoriNet Doctor`, launched on Windows 11 Pro x64, and completed another real 16/16 diagnosis in 4,492 ms. App remains responsive and running; language/theme restored to zh-CN/dark.
- Real monitor: 70 samples, pause at 39, resume, stop; DNS Doctor: 8 queries; native IPv4/IPv6 loopback traces and TCP/HTTP/TLS tests; custom-target GUI traceroute, TCP, proxy failure, history, JSON save, summary copy, English/light theme were exercised.
- Google DoH succeeded; Cloudflare DoH had TLS EOF on this host and is recorded as failed, not passed. Windows 10 physical-device and broader network acceptance remain unverified.
- Release files and actual diagnostic records are ignored local artifacts/application data. Only privacy-safe genuine screenshots are committed. See docs/VALIDATION.md for exact boundaries and docs/ROADMAP.md for future work.
