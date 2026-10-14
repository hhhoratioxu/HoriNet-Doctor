# Contributing

Thanks for improving HoriNet Doctor. Discuss substantial changes in an issue first. Use the issue templates and include OS version, app version and reproduction steps. **Never publish unredacted diagnostics, SSIDs, addresses, credentials or private proxy settings.**

## Development

Follow [docs/BUILD.md](docs/BUILD.md). Network logic belongs in Rust. Keep OS FFI under `platform`, pure findings under `rules`, and React focused on interaction/presentation. Do not copy unpublished/private code. Keep the MIT license compatible.

Before submitting:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm --prefix desktop ci
npm --prefix desktop run typecheck
npm --prefix desktop run lint
npm --prefix desktop run build
```

Add offline scenarios for meaningful behavior changes. Use bounded local responders for socket tests; don't rely on the public Internet in unit tests. Mark real network, runtime and CI-only verification separately. Never claim a test you did not run. The Windows packaging CI must pass.

No automatic system repairs, broad scans, telemetry, unaudited remote content or arbitrary shell IPC. Request consent before adding any public-IP discovery service. Preserve cancellation, time/size/concurrency limits and default report redaction. Review new `unsafe` with ownership, alignment, lifetime and release comments.

PRs should state the problem, resulting behavior, validation actually performed and remaining limitations. Use conventional, descriptive commits. Contributors agree their contribution is licensed under MIT.
