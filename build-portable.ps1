param([switch]$Check, [switch]$SkipBuild)
$ErrorActionPreference='Stop'
Set-Location $PSScriptRoot
if (-not $SkipBuild) { & "$PSScriptRoot\build.ps1" -Check:$Check }
$manifest=Get-Content "$PSScriptRoot\Cargo.toml" -Raw
$appVersion=[regex]::Match($manifest,'(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if (-not $appVersion) { throw 'Could not read the application version' }
$exe=Join-Path $PSScriptRoot 'dist\Solcalc.exe'
if ((Get-Item -LiteralPath $exe).VersionInfo.FileVersion -ne $appVersion) { throw 'Executable version does not match Cargo.toml. Rebuild first.' }
$archive=Join-Path $PSScriptRoot "dist\Solcalc-$appVersion-windows-x64.zip"
# Explicit public inputs: never include a working profile, test reports or caches.
Compress-Archive -LiteralPath $exe,"$PSScriptRoot\README.md","$PSScriptRoot\docs","$PSScriptRoot\data","$PSScriptRoot\THIRD-PARTY-NOTICES.txt","$PSScriptRoot\assets\Solcalc-Readme.txt" -DestinationPath $archive -CompressionLevel Optimal -Force
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip=[IO.Compression.ZipFile]::OpenRead($archive)
try {
    $entry=$zip.GetEntry('Solcalc.exe')
    if (-not $entry) { throw 'Portable ZIP has no root executable' }
    $stream=$entry.Open()
    $hasher=[Security.Cryptography.SHA256]::Create()
    try { $payloadHash=[BitConverter]::ToString($hasher.ComputeHash($stream)).Replace('-','') }
    finally { $hasher.Dispose(); $stream.Dispose() }
    if ($payloadHash -ne (Get-FileHash -LiteralPath $exe).Hash) { throw 'Portable ZIP payload mismatch' }
    if ($zip.Entries | Where-Object { $_.FullName -match '(^|/)(qa|\.tools|\.local)/|user-substances|settings\.ini|VALIDATION\.md' }) { throw 'Private or generated test data in portable ZIP' }
} finally { $zip.Dispose() }
Write-Host "Built and verified $archive"
