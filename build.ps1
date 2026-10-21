[CmdletBinding()]
param([switch]$DebugBuild)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    npm ci --ignore-scripts
    if ($LASTEXITCODE) { throw 'npm ci failed' }
    npm run build
    if ($LASTEXITCODE) { throw 'UI build failed' }
    $env:CARGO_TARGET_DIR = Join-Path $PSScriptRoot 'target/windows'
    $profile = if ($DebugBuild) { 'debug' } else { 'release' }
    if ($DebugBuild) { cargo build --locked } else { cargo build --locked --release }
    if ($LASTEXITCODE) { throw 'Rust build failed' }
    $bundle = Join-Path $PSScriptRoot 'target/bundle'
    New-Item -ItemType Directory -Force $bundle | Out-Null
    Copy-Item "target/windows/$profile/tether.exe",'plugin.json','oriel.ps1' $bundle -Force
    Copy-Item 'dist' $bundle -Recurse -Force
    Write-Host "Standalone bundle: $bundle"
} finally { Pop-Location }
