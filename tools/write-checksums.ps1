$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
$manifest=Get-Content "$projectRoot\Cargo.toml" -Raw
$appVersion=[regex]::Match($manifest,'(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if (-not $appVersion) { throw 'Could not read the application version' }
$lines=foreach ($name in @('Solcalc.exe',"Solcalc-$appVersion-windows-x64.zip","Solcalc-$appVersion-x64-setup.exe")) {
    $hash=(Get-FileHash -LiteralPath "$projectRoot\dist\$name" -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $name"
}
Set-Content -LiteralPath "$projectRoot\dist\SHA256SUMS.txt" -Value $lines -Encoding ascii
Write-Host ($lines -join "`n")
