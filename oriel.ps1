[CmdletBinding()]
param([string]$DataDir = "", [ValidateSet("serve", "check")][string]$Action = "serve")
$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$OutputEncoding = [Console]::OutputEncoding
$exe = Join-Path $PSScriptRoot 'tether.exe'
if (-not (Test-Path $exe)) { $exe = Join-Path $PSScriptRoot 'target/windows/release/tether.exe' }
if (-not (Test-Path $exe)) { $exe = Join-Path $PSScriptRoot 'target/windows/debug/tether.exe' }
if (-not (Test-Path $exe)) { throw 'Tether executable missing. Build the Windows executable first.' }
$arguments = @($Action)
if ($Action -eq 'serve') { $arguments += '--oriel-stdio' }
if ($DataDir) { $arguments += @('--data-dir', $DataDir) }
# Keep inherited stdin/stdout streaming. No Start-Process, detached daemon, install or update.
& $exe @arguments
exit $LASTEXITCODE
