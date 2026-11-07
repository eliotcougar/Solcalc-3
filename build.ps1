param([switch]$Check)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
. "$PSScriptRoot\tools\env.ps1"
if ($Check) {
    cargo fmt --check
    if ($LASTEXITCODE) { throw 'Formatting check failed' }
    cargo clippy --locked --all-targets -- -D warnings
    if ($LASTEXITCODE) { throw 'Clippy failed' }
    cargo test --locked
    if ($LASTEXITCODE) { throw 'Tests failed' }
}
cargo build --locked --release
if ($LASTEXITCODE) { throw 'Build failed' }
New-Item -ItemType Directory -Force "$PSScriptRoot\dist" | Out-Null
Copy-Item -LiteralPath "$PSScriptRoot\target\release\solcalc.exe" -Destination "$PSScriptRoot\dist\Solcalc.exe"
Write-Host "Built $PSScriptRoot\dist\Solcalc.exe"
