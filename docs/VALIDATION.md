# Validation record

Date: 2026-10-14 (Asia/Taipei). Never interpret build success as full runtime or physical-device acceptance.

## Executed locally

- Windows 11 Pro x64 (10.0.26200), stable Rust 1.97.1 GNU, Node 24.18.0/npm 11.16.0.
- Core: 29 tests passed, zero failures; covers all ten required scenarios plus IPv6-only, route/portal/Wi-Fi/variation rules, malformed DNS, real loopback sockets/ICMP, HTTP and SOCKS5 negotiation, settings/history, partial-cancellation scoring and cancellation while monitor is paused. IPv4/IPv6 loopback reply addresses are verified as well as echo success.
- Workspace Clippy with `-D warnings`: passed after correcting a rule helper signature and test initialization.
- TypeScript typecheck, frontend ESLint (zero warnings) and Vite production build: passed.
- Tauri desktop `cargo check` with Windows GNU: passed.
- Real one-click CLI diagnosis: all 16 checks completed in about 6.1 seconds. Native adapter/route/gateway, IPv4 TCP, system DNS, UDP DNS, HTTP, HTTPS and TLS succeeded on this host. IPv6 TCP failed; DHCP/proxy/WLAN evidence was partly unavailable and marked unknown. This describes one environment, not universal availability. No private addresses or actual snapshot are committed.
- Real public DNS `example.com` at `1.1.1.1:53`: UDP and TCP each returned an A record. Google `https://dns.google/dns-query` DoH returned 2 A records. Optional Cloudflare DoH failed with TLS handshake EOF on this host and is **not marked as passed**. OS VPN/TUN routing may influence these observations; they are not claims about resolver authenticity.
- Native IPv4 and IPv6 loopback traceroute each received the destination at hop 1. This is not public-path traceroute acceptance.
- Actual locally built Windows Tauri desktop launched. One-click diagnosis completed 16/16 and saved history. Monitor produced 70 real ICMP/TCP samples: paused at 39 (count remained 39), resumed (count grew), stopped at 70; chart/statistics remained visible. DNS Doctor completed 8 UDP/TCP A/AAAA queries across two displayed endpoints, with A and NODATA represented distinctly. No synthetic data was injected. `docs/images/desktop-*.png` are genuine screenshots without private source addresses/SSIDs.
- Native JSON save dialog exported the actual 16-check record (11,781 bytes); output contained redaction markers and no observed private IPv4/MAC values. Summary copy was verified in the clipboard without private IPv4 values. Unit tests also cover Markdown redaction and explicit detailed opt-in.
- Complete workspace GNU tests (including desktop harness) passed: 29 core tests, zero failures. GNU linker emits a duplicate-manifest warning; official MSVC validation is separate. An initial local workspace retry while the desktop was running hit a locked WebView2Loader DLL; closing that development instance and retrying succeeded.

## GitHub Windows MSVC validation

- First full CI [38533306087](https://github.com/hhhoratioxu/HoriNet-Doctor/actions/runs/38533306087) passed at `25d2aba893dcca65e3f53435342db236cf4f6278`: fmt, workspace Clippy/tests (26 at that revision), npm ci/typecheck/lint/build, x64 NSIS build and release packaging. Runner Rust 1.99.0, Node 24.21.0.
- CI installer (3,741,502 bytes), CLI, hashes and provenance downloaded; every hash matched. Windows Authenticode reported `NotSigned`, consistent with release metadata.
- Final code CI [38535015886](https://github.com/hhhoratioxu/HoriNet-Doctor/actions/runs/38535015886) passed at `a9d1bcf20b9f250cdfc9246f2721d55fa1d9c5a0`: **29 tests, zero failures**, workspace Clippy, formatting, frontend checks/build and MSVC NSIS packaging.
- Tag-triggered [Release 38535978289](https://github.com/hhhoratioxu/HoriNet-Doctor/actions/runs/38535978289) reran the full checks/build successfully and published [v0.1.0-beta.1](https://github.com/hhhoratioxu/HoriNet-Doctor/releases/tag/v0.1.0-beta.1). Verified public, non-draft and prerelease; target is the exact full code commit above.

## Installed release acceptance

- Downloaded the actual Release assets (installer, companion CLI, provenance, checksums), verified all hashes and checked provenance commit/run/target. Final installer is **3,749,375 bytes**, unsigned (`NotSigned`). SHA-256: `d1959e5c37d58fc6841c72d5ebe5d1a60a634d73ef3f23fa84e73e9803cb875e`.
- NSIS silent current-user install exited **0**. Final executable exists (14,431,232 bytes), launched from `%LOCALAPPDATA%\Programs\HoriNet Doctor\horinet-doctor.exe`, exposed a real native window and remained responsive.
- The installed final Release completed **16/16 real checks in 4,492 ms**, saved structured history and displayed findings. This is completion evidence, not a claim that all measured checks were normal (IPv6 target failed, some fields remained unknown).
- The prior MSVC CI installer also installed/launched, completed diagnosis, displayed local history across restarts, correctly reported a refused user-selected proxy endpoint, displayed custom-target `127.0.0.1` traceroute hop 1 / 0.0 ms / Echo reply, and displayed a real TCP result (0.6 ms). Both address families were also exercised through the MSVC companion CLI on loopback.
- English/light theme was selected through the installed UI, saved, visually verified and checked in persisted settings. Defaults were restored to Simplified Chinese/dark before launching the final Release. Genuine screenshots show the final Release empty state and installed English/light settings, with only public default probe targets visible.
- The running final app and all source code are retained locally. No raw reports, settings, tokens, generated caches or binary downloads are tracked in Git. Post-release documentation/screenshot commits do not change the immutable release's code commit.

## Not verified or not implemented

- No Windows 10 physical-machine acceptance, diverse physical Wi-Fi/portals/enterprise CA/VPN matrix, overnight/24-hour monitor test, SmartScreen reputation/signing certification or independent security audit.
- Corrupt-settings recovery and every native dialog/keyboard/screen-reader combination have not been exhaustively exercised; passing compilation/unit checks are not presented as exhaustive GUI acceptance.
- No macOS/Linux native implementation or runtime acceptance, automatic repair, PAC execution, authenticated proxy testing or DNSSEC validation.

Generated network reports remain in ignored local artifacts/application data and must not be added to Git. Known provider/network failures above remain visible and are not counted as successful connectivity tests.
