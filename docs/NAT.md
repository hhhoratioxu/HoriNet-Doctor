# NAT Doctor

NAT Doctor measures **IPv4 UDP** using an independently implemented STUN client in Rust. It separates mapping and filtering according to [RFC 5780](https://www.rfc-editor.org/rfc/rfc5780.html), with the modern STUN wire format from [RFC 8489](https://www.rfc-editor.org/rfc/rfc8489.html). RFC 5780 is experimental; its observations describe the current client port and server path, not every application or future connection.

## 使用 / Usage

1. Open **NAT Doctor**. Review or change the single `hostname:port` / `IPv4:port` target. The default is `stun.miwifi.com:3478`; public services can change or become unavailable. There is no automatic fallback to an undisclosed provider.
2. Read and check the consent box, then select **检测 NAT 类型 / Detect NAT behavior**. STUN contacts that server and its advertised alternate endpoints and discovers the server-observed mapped IP/port. Recipients see the request's source IP. No diagnostic report is uploaded.
3. Watch the individual probes. Cancel stops pending async I/O and releases the socket; partial probes remain visible without a final classification. Completed results are saved only in local history. Addresses stay masked until you select **显示地址 / Reveal addresses**.
4. Review mapping, filtering, confidence and limitations. Use JSON/Markdown export or copy the summary; all endpoints and nested evidence are redacted by default. Detailed export is an explicit choice.
5. To include NAT in every **one-click diagnosis**, open Settings, review the STUN server, explicitly enable the NAT consent option and save. It is **off by default**, including when upgrading old settings. Changing that server resets the consent checkbox. Enabled runs have 17 checks; ordinary runs retain 16. NAT does not change the health score.

Standalone cancelled probes are visible only in the current session and are not saved as a completed NAT history record. Open a completed **NAT Doctor** row in History to return to this page, including after restarting the app. Reopening a history record resets detailed export to unchecked. Cancelling the Windows Save As dialog writes no report. The installed beta.2 detection/cancel/history/JSON/Markdown/clipboard flows were [exercised on Windows 11](NAT_DESKTOP_ACCEPTANCE.md); this does not cover every router, carrier or Windows version.

CLI:

```text
horinet nat stun.miwifi.com:3478 --consent
horinet nat YOUR-STUN-HOST:3478 --consent --detailed
```

`--consent` is required before even resolving the server name. The first command exports redacted JSON; `--detailed` deliberately includes sensitive endpoints. Ctrl+C cancels.

## 结果解释 / Interpretation

| Mapping | Filtering | Displayed classic label |
| --- | --- | --- |
| Same as local endpoint | Any / unknown | No translation observed; inbound firewall behavior is separate |
| Endpoint independent | Endpoint independent | Full cone, observed on this path |
| Endpoint independent | Address dependent | Address restricted cone, inferred |
| Endpoint independent | Address and port dependent | Port restricted cone, inferred |
| Address dependent or address and port dependent | Any / unknown | Symmetric mapping, observed |
| Insufficient or inconsistent evidence | Any | Unknown; keep available binding evidence |

Most basic STUN servers only reveal the mapped endpoint. **A basic Binding response cannot establish full-cone NAT.** Full mapping/filtering discovery needs two distinct server IPs/ports, `OTHER-ADDRESS`, consistent `RESPONSE-ORIGIN` and honored `CHANGE-REQUEST`. A server without these capabilities produces a useful binding result with unknown type. No-response can also reflect server failure or packet loss.

Restricted filtering is inferred only after bounded change-response timeouts and successful controls. Before inferring port-dependent filtering, both alternate listeners must be reachable. Baseline checks run before and after mapping comparisons; instability withdraws the type judgment. Classification confidence is at most medium, and timeout-dependent filtering is low confidence.

These are **not** PlayStation/Nintendo NAT 1/2/3 or Xbox Open/Moderate/Strict ratings. The test cannot prove CGNAT, double NAT, inbound TCP reachability, port forwarding, UPnP status, hairpin support, NAT lifetime, IPv6 NAT or successful P2P communication. A router, firewall, VPN, TUN or UDP proxy may all affect the observed result. STUN is unauthenticated; results are diagnostic evidence, not a security assertion. Do not disable a firewall or enable DMZ from this result alone.

## Implementation and limits

- One IPv4 source socket, filtering tests **before** sending to alternate destinations, then mapping comparisons and a final baseline check.
- At most 8 sequential transactions, 3 datagrams per transaction, paced at least 110 ms apart. Each probe uses the configured timeout clamped to 0.5–5 seconds; the entire operation, including any wait for the NAT slot, is capped at 60 seconds. No keepalives or scanning.
- One global NAT session and the desktop's shared job slot; no background probe threads. Cancellation interrupts DNS/socket waits and releases resources.
- Validate transaction ID, magic cookie, class/method, lengths, padding, bounded attribute count, mapped address family, actual response source and optional fingerprint. Ignore unrelated transactions. Do not follow STUN redirects or support credentials.
- A public server cannot direct an alternate probe into private, loopback, link-local, shared or multicast ranges. Private/custom server results describe only that selected path, not the Internet edge.
- Original settings and reports still deserialize; the optional `nat` object extends report schema 1. Raw local history contains detailed endpoints and is protected by the Windows account, without additional encryption.
- Protocol fixtures in `crates/doctor-core/tests/nat.rs` are explicitly synthetic and use loopback sockets. They are never loaded into the app. See [validation](VALIDATION.md) for separately recorded real observations.
