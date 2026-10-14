# Validation record

Date: 2026-10-14 (Asia/Taipei). Never interpret build success as full runtime or physical-device acceptance.

## Executed locally

- Windows stable Rust 1.97.1 GNU, Node 24.18.0/npm 11.16.0.
- Core: 26 tests passed, zero failures at the initial stage boundary; covers all ten required scenarios plus IPv6-only, route/portal/Wi-Fi/variation rules, malformed DNS, real loopback sockets/ICMP, proxy negotiation, settings/history and score behavior.
- Workspace Clippy with `-D warnings`: passed after correcting a rule helper signature and test initialization.
- TypeScript typecheck, frontend ESLint (zero warnings) and Vite production build: passed.
- Tauri desktop `cargo check` with Windows GNU: passed.
- Real one-click CLI diagnosis: all 16 checks completed in about 6.1 seconds. Native adapter/route/gateway, IPv4 TCP, system DNS, UDP DNS, HTTP, HTTPS and TLS succeeded on this host. IPv6 TCP failed; DHCP/proxy/WLAN evidence was partly unavailable and marked unknown. This describes one environment, not universal availability. No private addresses or actual snapshot are committed.
- Real public DNS `example.com` at `1.1.1.1:53`: UDP and TCP each returned an A record. Optional Cloudflare DoH transport failed on this host and is **not marked as passed**.
- Native IPv4 and IPv6 loopback traceroute each received the destination at hop 1. This is not public-path traceroute acceptance.

## In progress / pending final verification

- Desktop interaction, cancellation, pause/resume, native export dialogs and installed MSVC build.
- Windows MSVC CI/tests/NSIS packaging and release download/hash/install/launch.

## Not verified or not implemented

- No Windows 10 physical-machine acceptance, diverse physical Wi-Fi/portals/enterprise CA/VPN matrix, overnight/24-hour monitor test, SmartScreen reputation/signing certification or independent security audit.
- No macOS/Linux native implementation or runtime acceptance, automatic repair, PAC execution, authenticated proxy testing or DNSSEC validation.

Generated network reports remain in ignored local artifacts/application data and must not be added to Git. Later sections will record actual completion of pending checks.
