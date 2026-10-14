# Security policy

## Supported version

Only the latest `0.1.0-beta.*` prerelease receives fixes at this stage. This is unsigned beta software and has not had an independent security audit.

## Report a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/hhhoratioxu/HoriNet-Doctor/security/advisories/new) if enabled. Otherwise contact the maintainer using the contact options on [the GitHub profile](https://github.com/hhhoratioxu) to arrange a private channel. Do not post exploit details, credentials or unredacted diagnostic reports in public issues. No guaranteed response time or bounty is promised.

Include affected version/commit, minimal reproduction, expected/actual behavior and impact. Use fictitious addresses in published reproductions. Coordinate disclosure after a fix is available.

## Security model

- Local trusted Windows user; detailed history is protected by account permissions, not application encryption. A compromised account can read it.
- Rust-only probes with explicit host/endpoint/URL validation. No shell execution, privileged repair or secrets from third-party applications.
- Single active network job; globally bounded blocking native calls; native ICMP timeout at most 10 seconds, cancellation and finite sampling/hops/packet sizes.
- Untrusted DNS wire packets are length-checked, question/transaction-checked, compression-hop-limited and read without `unsafe`. UDP sockets are connected to their selected server. No DNSSEC guarantee.
- HTTP redirects are not followed; TLS validation is enabled. Proxy CONNECT tests do not send credentials or execute PAC.
- Explicit Tauri command capability for the bundled main window, restricted CSP, no remote UI, no frontend shell/filesystem/network plugin.
- Exports default to recursive redaction, including free-text evidence/errors. This cannot promise to identify every possible custom secret; users should review before sharing.
- Build hashes help verify file integrity; unsigned releases do not authenticate an executable through a code-signing identity.

No system DNS/adapter/route/firewall/Winsock changes exist in this beta. Future repair actions require a separate design and explicit user confirmation.
