param([switch]$Check, [switch]$SkipBuild, [string]$Iscc)
$ErrorActionPreference='Stop'
Set-Location $PSScriptRoot
if (-not $SkipBuild) { & "$PSScriptRoot\build.ps1" -Check:$Check }
$manifest=Get-Content "$PSScriptRoot\Cargo.toml" -Raw
$appVersion=[regex]::Match($manifest,'(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if (-not $appVersion) { throw 'Could not read the application version' }
$exe=Join-Path $PSScriptRoot 'dist\Solcalc.exe'
if ((Get-Item -LiteralPath $exe).VersionInfo.FileVersion -ne $appVersion) { throw 'Executable version does not match Cargo.toml. Rebuild first.' }
if (-not $Iscc) {
    $command=Get-Command ISCC.exe -ErrorAction SilentlyContinue
    $candidates=@($env:SOLCALC_ISCC, "$PSScriptRoot\.tools\inno-6.7.4\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe")
    if ($command) { $candidates=@($command.Source)+$candidates }
    $Iscc=$candidates | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) } | Select-Object -First 1
}
if (-not $Iscc) { throw 'Inno Setup 6.7+ is required. Set SOLCALC_ISCC or pass -Iscc. tools/bootstrap-installer.ps1 creates a local portable compiler.' }
& $Iscc /Qp "/DAppVersion=$appVersion" "/DAppExe=$exe" "$PSScriptRoot\installer\Solcalc.iss"
if ($LASTEXITCODE) { throw 'Installer compilation failed' }
Write-Host "Built $PSScriptRoot\dist\Solcalc-$appVersion-x64-setup.exe"
