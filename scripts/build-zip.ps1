#Requires -Version 7
# scripts/build-zip.ps1 — Build the portable Windows .zip for LaserCAD.
#
# Usage (from any directory, on Windows with pwsh 7+):
#   ./scripts/build-zip.ps1
#
# Uses target/release/lasercad.exe when it already exists (CI downloads it
# from the build job); otherwise runs `cargo build --release` first.
#
# Output: dist/lasercad-<version>-windows-x86_64.zip holding lasercad.exe,
# LICENSE-APACHE, LICENSE-MIT and FIRST-RUN.txt. The app is unsigned; the
# first-run steps live in docs/install.md (named by FIRST-RUN.txt).

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

# Version from Cargo.toml at runtime — never hardcoded.
$Match = Select-String -Path 'Cargo.toml' -Pattern '^version = "(.+)"$' | Select-Object -First 1
if ($null -eq $Match) { throw 'cannot read version from Cargo.toml' }
$Version = $Match.Matches[0].Groups[1].Value
Write-Host "==> VERSION=$Version"

$Exe = 'target/release/lasercad.exe'
if (-not (Test-Path $Exe)) {
    Write-Host '==> cargo build --release'
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit code $LASTEXITCODE)" }
}

$Name = "lasercad-$Version-windows-x86_64"
$Staging = Join-Path 'build' $Name
$Output = Join-Path 'dist' "lasercad-$Version-windows-x86_64.zip"

Write-Host "==> Staging $Staging"
if (Test-Path $Staging) { Remove-Item $Staging -Recurse -Force }
New-Item -ItemType Directory -Path $Staging -Force | Out-Null
Copy-Item $Exe (Join-Path $Staging 'lasercad.exe')
Copy-Item 'LICENSE-APACHE' $Staging
Copy-Item 'LICENSE-MIT' $Staging
Copy-Item 'assets/FIRST-RUN.txt' $Staging

Write-Host "==> Creating $Output"
New-Item -ItemType Directory -Path 'dist' -Force | Out-Null
if (Test-Path $Output) { Remove-Item $Output -Force }
Compress-Archive -Path (Join-Path $Staging '*') -DestinationPath $Output

Write-Host "==> Done: $Output"
