# Rebuild the native small-icon frames from vector artwork. ImageMagick is
# only an asset-authoring dependency; building/running Solcalc does not use it.
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$output = Join-Path $projectRoot 'qa\icons'
New-Item -ItemType Directory -Force $output | Out-Null
$frames = @(foreach ($size in @(16,20,24,32,40,48,64)) {
    $frame = Join-Path $output "small-$size.png"
    magick -background none -density ($size * 6) (Join-Path $projectRoot 'assets\icon-small.svg') -resize "${size}x${size}!" $frame
    if ($LASTEXITCODE) { throw 'Could not render icon frame.' }
    $frame
})
magick @frames (Join-Path $projectRoot 'assets\icon-small.ico')
if ($LASTEXITCODE) { throw 'Could not package icon frames.' }
