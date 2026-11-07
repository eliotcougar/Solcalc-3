# Download the signed official compiler and extract it in portable mode.
param([string]$Curl)
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
$version='6.7.4'
$expected='A549929319F9120716677C685535CA4671E4B937B5FDC87E2585F32D8EB30E8C'
$destination=Join-Path $projectRoot ".tools\inno-$version"
if (Test-Path -LiteralPath "$destination\ISCC.exe") { Write-Host "$destination\ISCC.exe"; exit 0 }
if (-not $Curl) {
    $candidates=Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\cURL.cURL*\curl-*-win64-mingw\bin\curl.exe" -ErrorAction SilentlyContinue
    $Curl=$candidates | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $Curl) { throw 'Pass -Curl with the path to the OpenSSL curl-mingw executable.' }
$downloads=Join-Path $projectRoot '.tools\downloads'
New-Item -ItemType Directory -Force $downloads | Out-Null
$archive=Join-Path $downloads "innosetup-$version.exe"
if (-not (Test-Path -LiteralPath $archive)) {
    & $Curl --fail --location --silent --show-error --max-time 90 "https://github.com/jrsoftware/issrc/releases/download/is-6_7_4/innosetup-$version.exe" -o $archive
    if ($LASTEXITCODE) { throw 'Compiler download failed' }
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expected) { throw 'Compiler SHA-256 does not match the pinned official release' }
$signature=Get-AuthenticodeSignature -LiteralPath $archive
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Pyrsys B.V.') { throw 'Compiler signature could not be verified' }
$log=Join-Path $downloads 'inno-portable.log'
$process=Start-Process -FilePath $archive -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/CURRENTUSER','/PORTABLE=1','/NOICONS','/SP-',"/DIR=`"$destination`"","/LOG=`"$log`"" -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath "$destination\ISCC.exe")) { throw "Portable compiler extraction failed. See $log" }
Write-Host "$destination\ISCC.exe"
