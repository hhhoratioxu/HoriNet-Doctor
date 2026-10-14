$ErrorActionPreference='Stop'
$taskRoot=Split-Path -Parent $PSScriptRoot
$taskManifest=Get-Content -LiteralPath (Join-Path $taskRoot 'desktop\src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$taskVersion=$taskManifest.version
$taskOut=Join-Path $taskRoot 'release-artifacts'
New-Item -ItemType Directory -Path $taskOut -Force | Out-Null
$taskInstallers=@(Get-ChildItem -LiteralPath (Join-Path $taskRoot 'target\x86_64-pc-windows-msvc\release\bundle\nsis') -Filter '*-setup.exe')
if ($taskInstallers.Count -ne 1) { throw 'Expected exactly one NSIS installer' }
$taskCli=Join-Path $taskRoot 'target\x86_64-pc-windows-msvc\release\horinet.exe'
$taskDesktop=Join-Path $taskRoot 'target\x86_64-pc-windows-msvc\release\horinet-doctor.exe'
foreach ($taskFile in @($taskInstallers[0].FullName,$taskCli,$taskDesktop)) {
    if (!(Test-Path -LiteralPath $taskFile) -or (Get-Item -LiteralPath $taskFile).Length -le 0) { throw "Missing or empty binary: $taskFile" }
}
Copy-Item -LiteralPath $taskInstallers[0].FullName -Destination (Join-Path $taskOut "HoriNet-Doctor_${taskVersion}_windows-x64-setup.exe")
Copy-Item -LiteralPath $taskCli -Destination (Join-Path $taskOut "horinet-cli_${taskVersion}_windows-x64.exe")
$taskProvenance=@{ version=$taskVersion; commit=$env:GITHUB_SHA; repository=$env:GITHUB_REPOSITORY; run_id=$env:GITHUB_RUN_ID; target='x86_64-pc-windows-msvc'; signed=$false; prerelease=$true; rust=(rustc -V); node=(node --version); installer_bytes=$taskInstallers[0].Length; built_at_utc=[DateTime]::UtcNow.ToString('o'); validation='fmt, workspace clippy/tests, TypeScript, ESLint, Vite, Windows NSIS build; physical-device runtime acceptance is separate' }
$taskProvenance | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'BUILD-windows-x64.json') -Encoding utf8NoBOM
$taskHashes=Get-ChildItem -LiteralPath $taskOut -File | Where-Object Name -ne 'SHA256SUMS.txt' | Sort-Object Name | ForEach-Object { '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(),$_.Name }
$taskHashes | Set-Content -LiteralPath (Join-Path $taskOut 'SHA256SUMS.txt') -Encoding utf8NoBOM
