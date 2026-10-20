# NAT Doctor installed desktop acceptance

Executed on **2026-10-20, Asia/Taipei**, against the installed **v0.1.0-beta.2** public release, source `620de573068906b2dbd7cb40ac5bf79fc1b7209d`. Environment: Windows 11 Pro x64 10.0.26200, 125% display scale, Simplified Chinese/dark theme. The installed executable was 14,789,120 bytes, product version 0.1.0-beta.2, Authenticode **NotSigned**.

This is real native-window interaction with the installed Tauri application. Windows input, actual screenshots, application-history files, native Save As output and the clipboard were checked. No browser debugging port, injected results, mock IPC or frontend preview was used. The screenshots and reports contain local/private data and are kept outside the source checkout; this document contains only sanitized observations.

## Reproduction and observed results

| Flow | Action and expected behavior | Actual result |
| --- | --- | --- |
| Consent gate | Uncheck consent and click Start | Passed: Start disabled; no task or new history record |
| Real detection | Consent to the displayed `stun.miwifi.com:3478` target, then Start | Passed: live probes appeared, full result and local history were produced; first run 8 probes / 7,854 ms |
| Running cancellation | Start another run, cancel while I/O is pending | Passed: one completed probe remained visible, cancellation text explicitly withheld final NAT classification, Start re-enabled; history count unchanged |
| Run after cancellation | Start again after the cancellation feedback | Passed: full 8-probe run / 7,824 ms, new completed history record |
| History selection | Open the newly created NAT row | Passed: NAT Doctor opened with the selected classification, duration and probes |
| Persistence | Close normally, restart installed executable, open the same history row | Passed: same record available; file hash unchanged |
| Default JSON | Leave detail unchecked, save using native dialog and a Chinese filename | Passed: 9,228-byte valid JSON, selected report ID and all 8 probes retained, endpoint values redacted, enum fields intact |
| Default Markdown | Save the same record using native dialog | Passed: 8,837-byte Markdown, selected source timestamp and NAT classification retained, endpoint values redacted |
| Summary copy | Click Copy Summary, inspect the resulting clipboard | Passed: success feedback, 488 characters, selected timestamp/NAT content, no source IP values |
| Cancel export | Enter a unique path in Save As, then Escape | Passed: no file, no export-success notice, application responsive; next export succeeded |
| Detailed opt-in | Explicitly check detail, export another local JSON | Passed: 9,618 bytes, selected ID and exact local/mapped/server endpoints retained; file kept local only |
| Privacy reset | Reopen the record from History | Passed: detailed option unchecked and displayed addresses masked |
| Cancel record deletion | Delete a newly created test record, cancel confirmation | Passed: row and file remained; count stayed 8 |
| Confirm record deletion | Delete that same test-created record and confirm | Passed: corresponding row/file removed; count changed 8 to 7 |
| Cancel Clear All | Open Clear All and cancel confirmation | Passed: count stayed 7; all four pre-existing records remained |

Four NAT runs completed during this session; the first and post-cancellation source records were individually checked to contain 8 actual STUN probes. Two attempts to click Cancel arrived after a run had already completed; those count as completed detections, not successful cancellations. A subsequent request was cancelled while running and followed by a successful new detection.

The selected complete result observed endpoint-independent mapping and address-and-port-dependent filtering. The UI correctly described the classic label as **port-restricted cone, inferred, low confidence**. Two change-response probes timed out after bounded retries, while primary/alternate controls and final stability checks replied. This describes one IPv4 UDP socket/path/time and is not a whole-network failure verdict or proof of router/P2P behavior.

Privacy validation compared each default JSON, Markdown and copied summary with all **four distinct actual IPv4 values** extracted from the source record, plus the original NAT endpoint strings. No original IP or endpoint value remained. Detailed export was separately checked for exact endpoint retention. Opening history and exporting did not change the stored source record.

## Outcome and boundaries

No application defect was observed in these flows. The follow-up updates documentation only, with no replacement of the beta.2 installer or tag. The original four history records were preserved; only one record created by this acceptance session was deleted. The app remained responsive after export, cancellation and restart. A local evidence manifest records artifact sizes and SHA-256 hashes without uploading the reports. A separate read-only verifier of these actual output files and retained history completed **12 checks with zero failures**; those checks are distinct from the manual GUI observations above and the prior Rust suite.

Actual Clear All deletion was not exercised against existing user records. Windows 10, other scaling/theme/language combinations, keyboard/screen-reader coverage, diverse physical routers/carriers/VPNs, game-console NAT ratings, CGNAT proof and real P2P application acceptance remain outside this test. This follow-up did not rerun the earlier 51 Rust tests or build checks because application sources were unchanged; their separate evidence remains in [VALIDATION.md](VALIDATION.md).
