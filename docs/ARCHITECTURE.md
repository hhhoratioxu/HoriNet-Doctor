# Architecture

```text
React/TypeScript views
  → validated, allowlisted Tauri commands
  → single active job + cancellation token + progress events
  → Rust diagnostics / DNS / monitor / proxy / traceroute
  → native Windows platform boundary + bounded Tokio sockets
  → typed checks → pure rules → findings/score
  → local structured history → recursive redaction → user-chosen report file
```

## Layout

- `crates/doctor-core/src/models.rs`: serializable contracts; mirrored explicitly in `desktop/src/models.ts`.
- `diagnostics.rs`: 16-check orchestration, real snapshot, timing, progress, partial cancellation record.
- `rules.rs`: pure deterministic findings and score; does not perform network I/O. Offline tests provide synthetic inputs only inside tests.
- `network/connectivity.rs`: bounded system resolution/TCP, direct HTTP(S), standalone TLS verification.
- `network/dns.rs`: bounded DNS A/AAAA wire codec, real UDP/TCP sockets and optional HTTPS wire-format DoH.
- `network/icmp.rs`: native ICMP blocking boundary, global semaphore of 2. Permits live inside native closures so cancellation cannot spawn unlimited in-flight blocking calls.
- `network/monitor.rs`: one sequential sample loop, simultaneous ICMP/TCP per sample, missed intervals skipped, pause/resume and finite limits.
- `network/traceroute.rs`: native TTL/hop limit, sequential max 30, one echo per hop, unknown responses retained.
- `network/proxy.rs`: explicit HTTP CONNECT / SOCKS5 negotiation and safe parsing of a configured static proxy endpoint.
- `network/stun.rs`: bounded RFC 8489 UDP transaction/attribute codec, 96-bit transaction IDs, optional fingerprint validation, source checks, finite retransmission and cancellation. No credentials or redirects.
- `network/nat.rs`: consent gate before DNS/socket creation; RFC 5780 filtering before mapping, alternate-listener controls and stable baseline checks, conservative labels/confidence. One socket, at most 8 transactions and a 60-second total bound. [NAT design and interpretation](NAT.md).
- `platform/windows.rs`: isolated Windows FFI. Adapter output is aligned and retained during traversal; ICMP/WLAN/MIB handles and API allocations are released through RAII, WinHTTP strings through `GlobalFree`. No native mutable network API or shell fallback.
- `reports.rs`: recursive string redaction across snapshots/checks/findings/errors; JSON, Markdown and summary. Detailed export is opt-in.
- `storage.rs`: application-owned UUID-addressed JSON history, bounded reads/retention, atomic writes, validated settings.
- `desktop/src-tauri/src/commands.rs`: IPC input limits, one shared job slot, event correlation by UUID, pause/cancel, history and native save dialogs. Events never enter public logs.
- `desktop/src`: React state, real empty states, charting, relationship view, history and settings. No critical probe logic or synthetic measurements.

## Native sources

Adapter/address/DHCP/gateway/DNS data comes from [GetAdaptersAddresses](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses). Default `/0` route rows come from [GetIpForwardTable2](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getipforwardtable2); displayed metric is the row's route metric, not a claim about the OS-selected full path. WLAN attributes use [WlanQueryInterface](https://learn.microsoft.com/en-us/windows/win32/api/wlanapi/nf-wlanapi-wlanqueryinterface). ICMP uses `IcmpCreateFile`/`IcmpSendEcho` and [Icmp6SendEcho2](https://learn.microsoft.com/en-us/windows/win32/api/icmpapi/nf-icmpapi-icmp6sendecho2). Current-user proxy metadata uses [WinHttpGetIEProxyConfigForCurrentUser](https://learn.microsoft.com/en-us/windows/win32/api/winhttp/nf-winhttp-winhttpgetieproxyconfigforcurrentuser); classic WinHTTP defaults use `WinHttpGetDefaultProxyConfiguration`.

## Boundaries and interpretation

Timeouts wrap DNS/TCP/HTTP/TLS. Dropped async socket futures release sockets. Native ICMP has its own at-most-10-second timeout; a cancelled native call may finish after UI cancellation while retaining its semaphore permit. Native snapshot calls share a separate semaphore of 1 and a 10-second async boundary; if the OS call is unusually slow it retains that slot until it returns.

System DNS can use cache, hosts files and policy. Direct DNS is one configured resolver, and split policy is not fully enumerated. Tests are direct with no reqwest environment/system proxy. Windows VPN/TUN routes still apply. Proxy protocol success means a tunnel was accepted, not that target TLS/content was verified through it.

No callback depends on a frontend-mounted page: one root listener receives UUID-correlated progress; navigation does not stop a job. App exit cancels the current job. UI charts are bounded to the latest 180 samples and keep at most 10,000 actual samples. History has a user-chosen bound of 1–500 and stays local.

See the [Tauri capability model](https://v2.tauri.app/security/capabilities/) and [NSIS packaging documentation](https://v2.tauri.app/distribute/windows-installer/). Only the bundled main window receives the custom command allowlist. Export paths are selected by native save dialogs; no arbitrary file-path write IPC exists.
