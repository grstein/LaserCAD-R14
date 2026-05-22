#Requires -Version 5.1
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

<#
.SYNOPSIS
    Build the LaserCAD Windows MSI installer.

.DESCRIPTION
    Compiles lasercad.exe in release mode for x86-64 Windows, then uses cargo-wix
    to compile wix/main.wxs into an MSI via WiX Toolset v3, and copies the result
    to dist\lasercad-x86_64.msi.

.PREREQUISITES
    - Rust 1.88+ with the x86_64-pc-windows-msvc target installed
    - WiX Toolset v3 (candle.exe + light.exe) on PATH
    - cargo-wix installed:  cargo install cargo-wix

.USAGE
    .\scripts\build-msi.ps1
    (run from the repository root)
#>

# ---------------------------------------------------------------------------
# Resolve repository root — the script must be run from the repo root, but
# guard against accidental invocations from sub-directories.
# ---------------------------------------------------------------------------
$RepoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $RepoRoot

Write-Host "==> LaserCAD MSI build starting in: $RepoRoot" -ForegroundColor Cyan

# ---------------------------------------------------------------------------
# Step 1: Compile the release binary for x86-64 Windows MSVC.
# ---------------------------------------------------------------------------
Write-Host "==> cargo build --release --target x86_64-pc-windows-msvc" -ForegroundColor Cyan
cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo build failed (exit code $LASTEXITCODE)."
}

# ---------------------------------------------------------------------------
# Step 2: Compile wix/main.wxs with cargo-wix (skips the Rust build step
#         because we already built above, so --no-build is passed).
# ---------------------------------------------------------------------------
Write-Host "==> cargo wix --no-build --nocapture --target x86_64-pc-windows-msvc" -ForegroundColor Cyan
cargo wix --no-build --nocapture --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo wix failed (exit code $LASTEXITCODE)."
}

# ---------------------------------------------------------------------------
# Step 3: Locate the freshly-produced MSI and copy it to dist\.
# ---------------------------------------------------------------------------
$WixOutputDir = Join-Path $RepoRoot 'target\wix'
$SourceMsi = Get-ChildItem -Path $WixOutputDir -Filter '*.msi' -ErrorAction SilentlyContinue |
             Sort-Object LastWriteTime |
             Select-Object -Last 1

if ($null -eq $SourceMsi) {
    Write-Error "No .msi file found under $WixOutputDir after cargo wix completed."
}

$DistDir = Join-Path $RepoRoot 'dist'
if (-not (Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir | Out-Null
}

$DestMsi = Join-Path $DistDir 'lasercad-x86_64.msi'

# Remove any pre-existing artifact so stale files cannot accumulate.
if (Test-Path $DestMsi) {
    Remove-Item $DestMsi -Force
}

Copy-Item -Path $SourceMsi.FullName -Destination $DestMsi -Force
Write-Host "==> MSI copied to: $DestMsi" -ForegroundColor Green

$DestSize = (Get-Item $DestMsi).Length
Write-Host "==> Build complete. dist\lasercad-x86_64.msi ($DestSize bytes)" -ForegroundColor Green
