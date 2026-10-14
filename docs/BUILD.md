# Build and test / 本地编译

## Official Windows path

- Windows 10/11 x64, Microsoft Edge WebView2 Runtime.
- Rust Stable (edition 2021; manifest MSRV 1.88; validated compiler versions are recorded in validation/provenance).
- Visual Studio 2022/2026 Build Tools with **Desktop development with C++**, x64 MSVC compiler/linker and Windows SDK. Verify `link.exe` is available in the Developer PowerShell environment.
- Node.js 24 LTS and npm. Git. Internet for initial dependency/NSIS/WebView2 bootstrap downloads.

```powershell
git clone https://github.com/hhhoratioxu/HoriNet-Doctor.git
cd HoriNet-Doctor
npm --prefix desktop ci
npm --prefix desktop run typecheck
npm --prefix desktop run lint
npm --prefix desktop run build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm --prefix desktop run tauri -- dev
```

`cargo test` with default workspace members tests the core only. Use `--workspace` to compile/test the desktop as well. `cargo run -p horinet-core --bin horinet -- snapshot` prints only native counts. `diagnose` emits redacted JSON; no default output file is created. `Ctrl+C` cancels the CLI. The CLI is not an arbitrary command runner.

Build the installer:

```powershell
npm --prefix desktop run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis -- --locked
cargo build -p horinet-core --bin horinet --release --target x86_64-pc-windows-msvc --locked
```

NSIS output is in `target\x86_64-pc-windows-msvc\release\bundle\nsis`. The CLI is `target\x86_64-pc-windows-msvc\release\horinet.exe`. `scripts/package-release.ps1` requires nonempty installer/desktop/CLI, copies release assets, records build provenance and hashes. The release workflow validates `v<manifest version>` and only permits beta publication initially. Never change an existing tag to point to new code.

## Current development-host limitation

The initial development host lacked `link.exe`. Local core tests and desktop checks use `stable-x86_64-pc-windows-gnu`; official installable artifacts use MSVC on GitHub Actions. GNU builds are a development fallback, not the published target. Some GNU dependencies cannot link under non-ASCII output paths; move **only the cache**, for example:

```powershell
$env:CARGO_TARGET_DIR="$env:LOCALAPPDATA\HoriNetDoctor-build"
cargo +stable-x86_64-pc-windows-gnu test -p horinet-core --locked
```

Do not commit machine-specific cache paths. macOS/Linux native inspection returns an explicit unsupported error; there are no supported installers yet.

## Frontend preview

`npm --prefix desktop run dev` opens a local Vite development server on port 1420. A normal browser has **no native IPC**, so probe actions are disabled and an explanatory banner is shown. It displays no fake network readings. Use `tauri dev` or the desktop executable for functional acceptance.

## Release checklist

1. Run all checks and inspect actual app interaction, history, cancellation and exports.
2. Ensure Git working tree contains no `.env`, diagnostic reports, SSIDs, MACs, IP snapshots, token files or caches.
3. Update PLAN/CHANGELOG/VALIDATION and review the exact source commit.
4. Push source; wait for Windows CI success at that commit.
5. Push `v0.1.0-beta.1`. Release reruns checks/build and publishes a prerelease, hashes and provenance.
6. Download, verify hashes, install and launch; record runtime evidence separately from CI. Signing remains false unless an actual certificate/signing process is introduced.
