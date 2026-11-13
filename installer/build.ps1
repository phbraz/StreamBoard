# Builds the current UI into the release exe, then compiles TheBoard-Setup.exe.
# Pass -InstallerOnly when the release exe is already built (the GitHub workflow does this).
# A full build needs Node, Rust, and Inno Setup 6. -InstallerOnly needs Inno Setup 6.
# This script does not install the app.

param(
    [switch]$InstallerOnly
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $InstallerOnly) {
    Push-Location (Join-Path $root 'web')
    try {
        npm ci
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        npm run build
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    finally {
        Pop-Location
    }

    cargo build --release --locked -p soundboard-server
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

$candidates = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "$env:LocalAppData\Programs\Inno Setup 6\ISCC.exe"
)
$iscc = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) {
    throw "Inno Setup 6 is not installed. Install it, then run this script again."
}

$cargo = Get-Content -Raw (Join-Path $root 'server\Cargo.toml')
if ($cargo -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "Could not read the version from server\Cargo.toml"
}
$version = $Matches[1]

& $iscc "/DMyAppVersion=$version" (Join-Path $root 'installer\TheBoard.iss')
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Write-Host "Wrote $(Join-Path $root 'dist\TheBoard-Setup.exe')"
