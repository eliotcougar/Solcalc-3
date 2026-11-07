# Session-local tool discovery. Nothing is installed or added to the user's PATH.
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $rustCandidates = @(
        "$env:USERPROFILE\.cargo\bin",
        (Join-Path (Split-Path -Parent $projectRoot) 'Wayrill\.tools\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin')
    )
    $rustBin = $rustCandidates | Where-Object { Test-Path (Join-Path $_ 'cargo.exe') } | Select-Object -First 1
    if (-not $rustBin) { throw 'Install Rust with the x86_64-pc-windows-msvc toolchain, then run build.ps1 again.' }
    $env:PATH = "$rustBin;$env:PATH"
}
if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    $portableMsvc = if ($env:SOLCALC_MSVC_ROOT) { $env:SOLCALC_MSVC_ROOT } else {
        Join-Path (Split-Path -Parent $projectRoot) 'Wayrill\.tools\msvc'
    }
    if (Test-Path $portableMsvc) {
        $vc = Get-ChildItem (Join-Path $portableMsvc 'VC\Tools\MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
        $sdk = Join-Path $portableMsvc 'Windows Kits\10'
        $sdkVersion = Get-ChildItem (Join-Path $sdk 'Lib') -Directory | Sort-Object Name -Descending | Select-Object -First 1
        $env:PATH = "$($vc.FullName)\bin\Hostx64\x64;$sdk\bin\$($sdkVersion.Name)\x64;$env:PATH"
        $env:LIB = "$($vc.FullName)\lib\x64;$sdk\Lib\$($sdkVersion.Name)\um\x64;$sdk\Lib\$($sdkVersion.Name)\ucrt\x64"
        $env:INCLUDE = "$($vc.FullName)\include;$sdk\Include\$($sdkVersion.Name)\ucrt;$sdk\Include\$($sdkVersion.Name)\shared;$sdk\Include\$($sdkVersion.Name)\um"
        $env:RC = "$sdk\bin\$($sdkVersion.Name)\x64\rc.exe"
    } else {
        $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
        if (Test-Path $vswhere) {
            $vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
            if ($vs) {
                $devShell = Join-Path $vs 'Common7\Tools\Launch-VsDevShell.ps1'
                & $devShell -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
            }
        }
        if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) { throw 'MSVC and Windows SDK not found. Run in a VS x64 Native Tools shell or set SOLCALC_MSVC_ROOT.' }
    }
}
$env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
New-Item -ItemType Directory -Force $env:CARGO_HOME | Out-Null
