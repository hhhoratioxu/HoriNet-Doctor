# HoriNet Doctor delivery plan

Version: 0.1.0-beta.2. Independent implementation; no private HoriLink sources used.

## Native NAT desktop acceptance — completed 2026-10-20

- [x] Exercise the installed NAT page's consent gate and real STUN detection.
- [x] Cancel a running probe, verify partial state and run again.
- [x] Open the saved NAT record from History, including after restarting.
- [x] Export JSON and Markdown through native save dialogs; inspect actual files and privacy.
- [x] Verify summary copy, export-dialog cancellation, explicit detailed export and privacy reset.
- [x] Verify deletion/cancel using a record created by this test; cancel Clear All and preserve all four pre-existing records.
- [x] Record exact application/source version and evidence. No application defect observed; no new binary release needed.

Acceptance screenshots and reports stay under a dedicated local-app-data test directory, outside the synced source checkout. Do not equate CLI or browser preview checks with these native desktop flows.

Installed beta.2 (released source `620de573068906b2dbd7cb40ac5bf79fc1b7209d`) completed four real NAT runs. The first took 7,854 ms and the post-cancellation run took 7,824 ms, each with 8 probes verified in its stored record. Cancellation retained one partial probe, no final classification and no completed history record. Restart/history/export/clipboard checks passed. Default JSON (9,228 bytes), Markdown (8,837 bytes) and copied summary contained none of the four actual IPv4 values from the selected source record; explicit detailed JSON (9,618 bytes) retained the selected endpoints. [Native acceptance record](docs/NAT_DESKTOP_ACCEPTANCE.md).

## NAT Doctor follow-up — 2026-10-20

- [x] Recover relocated checkout; verify clean Git state and GitHub owner/permissions.
- [x] Review RFC 8489 / RFC 5780; keep mapping, filtering and classic NAT labels separate.
- [x] Implement bounded, cancellable UDP STUN and conservative behavior discovery in Rust.
- [x] Add explicit consent, configurable server, NAT Doctor UI, optional diagnosis integration, history and redacted exports.
- [x] Test local STUN fixtures, malformed packets, classification, cancellation, timeouts and privacy; 51 core tests and frontend checks passed locally (GNU).
- [x] Exercise real STUN, build Windows installer, verify CI and publish the next beta without changing beta.1.
- [x] Download/check/install the new release and document verified versus unverified results.

Beta.2 delivered: https://github.com/hhhoratioxu/HoriNet-Doctor/releases/tag/v0.1.0-beta.2
Released source: `620de573068906b2dbd7cb40ac5bf79fc1b7209d`.
Final CI `39435459052` and Release `39435605254` succeeded with 51 tests, zero failures.
All four assets downloaded, checksums/provenance verified; installer 3,831,684 bytes, NotSigned.
Install exit 0; installed product version 0.1.0-beta.2, real native window responsive.
Released MSVC CLI completed 8 real STUN probes in 7,770 ms, port-restricted-cone inference with low confidence; standalone core acceptance completed 17 checks and local history/export.
The earlier NAT native UI acceptance gap is now closed for the flows recorded above, using ordinary Windows window input and screenshots with the installed release. The earlier debugging-port attempt remained blocked and was not retried. No diverse physical-router, carrier or Windows 10 acceptance is claimed. See docs/VALIDATION.md.

The checkout moved to `D:\iCloud\iCloudDrive\Documents\CodeX\Hori开源\HoriNet-Doctor`.
NAT discovery sends no traffic until explicit user consent. A failed STUN request is not an Internet outage; unsupported behavior discovery remains unknown.

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

## Historical beta.1 checkpoint

Delivery completed on 2026-10-14. Public release: https://github.com/hhhoratioxu/HoriNet-Doctor/releases/tag/v0.1.0-beta.1

- Released code: `a9d1bcf20b9f250cdfc9246f2721d55fa1d9c5a0`; tag is immutable in this task.
- Final CI 38535015886 and Release 38535978289 both succeeded. 29 tests passed with zero failures; full frontend/Rust checks and MSVC NSIS packaging passed.
- Downloaded all 4 Release assets; every SHA-256 matched. Installer: 3,749,375 bytes; Authenticode NotSigned.
- Installed final Release with exit code 0 to `%LOCALAPPDATA%\Programs\HoriNet Doctor`, launched on Windows 11 Pro x64, and completed another real 16/16 diagnosis in 4,492 ms. App remains responsive and running; language/theme restored to zh-CN/dark.
- Real monitor: 70 samples, pause at 39, resume, stop; DNS Doctor: 8 queries; native IPv4/IPv6 loopback traces and TCP/HTTP/TLS tests; custom-target GUI traceroute, TCP, proxy failure, history, JSON save, summary copy, English/light theme were exercised.
- Google DoH succeeded; Cloudflare DoH had TLS EOF on this host and is recorded as failed, not passed. Windows 10 physical-device and broader network acceptance remain unverified.
- Release files and actual diagnostic records are ignored local artifacts/application data. Only privacy-safe genuine screenshots are committed. See docs/VALIDATION.md for exact boundaries and docs/ROADMAP.md for future work.
