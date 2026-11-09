# This mutates installation/registry state. Use only on a disposable Actions VM.
$ErrorActionPreference='Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw 'Installer smoke tests require a disposable GitHub-hosted runner.' }
if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Both-scope smoke tests require an elevated disposable runner.' }
$projectRoot=Split-Path -Parent $PSScriptRoot
$testRoot=Join-Path $projectRoot '.tools\installer-smoke'
$appVersion=[regex]::Match((Get-Content "$projectRoot\Cargo.toml" -Raw),'(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if (-not $appVersion) { throw 'Could not read the application version' }
$installer=Join-Path $projectRoot "dist\Solcalc-$appVersion-x64-setup.exe"
$keyName='{E9341401-CA59-4BF4-9CDB-E183C883CA31}_is1'
$roots=@('HKCU:','HKLM:')
foreach ($root in $roots) {
    if (Test-Path "$root\Software\Microsoft\Windows\CurrentVersion\Uninstall\$keyName") { throw 'Refusing to change an existing installation.' }
}
$profileDirectory=Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Solcalc'
if (Test-Path -LiteralPath $profileDirectory) { throw 'Refusing to use an existing profile.' }
New-Item -ItemType Directory -Force $testRoot,$profileDirectory | Out-Null
# Opaque profile fixtures let the installer prove it does not interpret or
# rewrite user data. Storage-format/migration behavior is tested by Rust.
Copy-Item -LiteralPath "$projectRoot\data\bundled\main.en.sqlite" -Destination "$profileDirectory\user-substances.sqlite"
Set-Content -LiteralPath "$profileDirectory\settings.ini" -Value "theme=dark`nlanguage=ru" -Encoding ascii
Copy-Item -LiteralPath "$profileDirectory\user-substances.sqlite" -Destination "$profileDirectory\user-substances.pre-v1-test.sqlite"
$profileHashes=@{}
Get-ChildItem -LiteralPath $profileDirectory -File | ForEach-Object { $profileHashes[$_.FullName]=(Get-FileHash -LiteralPath $_.FullName).Hash }
function Assert-Profile {
    foreach ($path in $profileHashes.Keys) {
        if ((Get-FileHash -LiteralPath $path).Hash -ne $profileHashes[$path]) { throw "Profile changed: $path" }
    }
}
$results=@()
foreach ($scope in @('CurrentUser','AllUsers')) {
    $appDirectory=Join-Path $testRoot "$scope\app"
    if (Test-Path -LiteralPath $appDirectory) { throw 'Test destination already exists.' }
    $root=if ($scope -eq 'AllUsers') { 'HKLM:' } else { 'HKCU:' }
    $otherRoot=if ($scope -eq 'AllUsers') { 'HKCU:' } else { 'HKLM:' }
    $registration="$root\Software\Microsoft\Windows\CurrentVersion\Uninstall\$keyName"
    for ($attempt=1; $attempt -le 2; $attempt++) {
        $arguments=@('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/NOICONS','/SP-',"/$($scope.ToUpperInvariant())",'/LANG=english',"/DIR=`"$appDirectory`"","/LOG=`"$testRoot\$scope-install-$attempt.log`"")
        $process=Start-Process -FilePath $installer -ArgumentList $arguments -WindowStyle Hidden -Wait -PassThru
        if ($process.ExitCode -ne 0) { throw "$scope installation failed: $($process.ExitCode)" }
        if ((Get-Item -LiteralPath "$appDirectory\Solcalc.exe").VersionInfo.FileVersion -ne $appVersion) { throw 'Installed version mismatch' }
        if ((Get-FileHash -LiteralPath "$appDirectory\Solcalc.exe").Hash -ne (Get-FileHash -LiteralPath "$projectRoot\dist\Solcalc.exe").Hash) { throw 'Installed payload mismatch' }
        $entry=Get-ItemProperty -LiteralPath $registration
        if ($entry.DisplayVersion -ne $appVersion -or $entry.InstallLocation.TrimEnd('\') -ne $appDirectory) { throw 'Uninstall registration mismatch' }
        if (Test-Path "$otherRoot\Software\Microsoft\Windows\CurrentVersion\Uninstall\$keyName") { throw 'Unexpected registration in the other scope' }
        Assert-Profile
    }
    Set-Content -LiteralPath "$appDirectory\unrelated-test-file.txt" -Value 'Keep files not installed by setup.'
    $process=Start-Process -FilePath "$appDirectory\unins000.exe" -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',"/LOG=`"$testRoot\$scope-uninstall.log`"" -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "$scope uninstall failed: $($process.ExitCode)" }
    $deadline=(Get-Date).AddSeconds(15)
    while ((Test-Path -LiteralPath $registration) -or (Test-Path -LiteralPath "$appDirectory\unins000.exe")) {
        if ((Get-Date) -gt $deadline) { throw 'Uninstall did not finish cleanup' }
        Start-Sleep -Milliseconds 200
    }
    foreach ($name in @('Solcalc.exe','Solcalc-Readme.txt','Solcalc-Third-Party-Notices.txt','ChEBI-LICENSE.txt','unins000.dat')) {
        if (Test-Path -LiteralPath "$appDirectory\$name") { throw "Installed file remains: $name" }
    }
    if (-not (Test-Path -LiteralPath "$appDirectory\unrelated-test-file.txt")) { throw 'Unrelated file was removed' }
    Assert-Profile
    $results+=@{scope=$scope;version=$appVersion;installAndReinstall=$true;payloadMatches=$true;registrationRemoved=$true;profilePreserved=$true;unrelatedFileRetained=$true}
}
$results | ConvertTo-Json | Set-Content -LiteralPath "$testRoot\result.json" -Encoding utf8
$results | ConvertTo-Json
