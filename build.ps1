[CmdletBinding()]
param([switch]$DebugBuild, [string]$LinuxBinary = "", [string]$WslDistribution = "")
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
    if (-not $LinuxBinary) {
        # Use an existing WSL toolchain only. These commands never install targets or packages.
        $wslArgs = @()
        if ($WslDistribution) { $wslArgs += @('--distribution', $WslDistribution) }
        $linuxRoot = (& wsl.exe @wslArgs --exec wslpath -a -u $PSScriptRoot).Trim()
        if ($LASTEXITCODE) { throw 'WSL path conversion failed; provide -LinuxBinary with an existing Linux x64 musl build.' }
        $linuxBuildArgs = @()
        if (-not $DebugBuild) { $linuxBuildArgs += '--release' }
        & wsl.exe @wslArgs --cd $linuxRoot --exec sh scripts/build-linux.sh @linuxBuildArgs
        if ($LASTEXITCODE) { throw 'Linux static build failed. Check existing cargo, musl target and linker; no tools were installed.' }
        $LinuxBinary = Join-Path $PSScriptRoot "target/linux/x86_64-unknown-linux-musl/$profile/tether"
    }
    if (-not (Test-Path $LinuxBinary)) { throw "Linux x64 backend missing: $LinuxBinary" }
    $elf = [IO.File]::ReadAllBytes((Resolve-Path $LinuxBinary))
    if ($elf.Length -lt 64 -or $elf[0] -ne 127 -or $elf[1] -ne 69 -or $elf[2] -ne 76 -or $elf[3] -ne 70 -or $elf[4] -ne 2 -or $elf[5] -ne 1 -or [BitConverter]::ToUInt16($elf,18) -ne 62) { throw 'LinuxBinary must be a Linux x64 ELF executable.' }
    $phoff = [BitConverter]::ToUInt64($elf,32)
    $phsize = [BitConverter]::ToUInt16($elf,54)
    $phcount = [BitConverter]::ToUInt16($elf,56)
    for ($i = 0; $i -lt $phcount; $i++) {
        $offset = $phoff + $i * $phsize
        if ($offset + 4 -gt $elf.Length -or [BitConverter]::ToUInt32($elf,[int]$offset) -eq 3) { throw 'LinuxBinary must be statically linked (no ELF interpreter).' }
    }
    $bundle = Join-Path $PSScriptRoot 'target/bundle'
    New-Item -ItemType Directory -Force $bundle | Out-Null
    Copy-Item $LinuxBinary (Join-Path $bundle 'tether-linux-x64') -Force
    Copy-Item "target/windows/$profile/tether.exe",'plugin.json','oriel.ps1','USER_GUIDE.md','LICENSE' $bundle -Force
    if (Test-Path (Join-Path $bundle 'dist')) { Remove-Item (Join-Path $bundle 'dist') -Recurse -Force }
    Copy-Item 'dist' $bundle -Recurse -Force
    Write-Host "Standalone bundle: $bundle"
} finally { Pop-Location }
