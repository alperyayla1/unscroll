# Builds the Unscroll installer (NSIS .exe).
# Requirements: Rust (MSVC), WebView2 (preinstalled on Win 10/11), and:
#   cargo install tauri-cli --version "^2" --locked
# Run from the project root:  powershell -ExecutionPolicy Bypass -File scripts\build.ps1
#   -Dev   builds a debug host and starts the app in dev mode (run from an ADMIN terminal)

param([switch]$Dev)
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

$triple = ((rustc -vV) | Select-String "^host:").ToString().Split(" ")[1]
$profile = if ($Dev) { "debug" } else { "release" }

Write-Host "Building native host ($profile)..."
if ($Dev) { cargo build -p unscroll-host } else { cargo build --release -p unscroll-host }
New-Item -ItemType Directory -Force src-tauri\binaries | Out-Null
Copy-Item "target\$profile\unscroll-host.exe" "src-tauri\binaries\unscroll-host-$triple.exe" -Force

Push-Location src-tauri
try {
  if ($Dev) { cargo tauri dev } else { cargo tauri build }
} finally { Pop-Location }

if (-not $Dev) {
  Write-Host "`nInstaller:" -ForegroundColor Green
  Get-ChildItem target\release\bundle\nsis\*.exe | ForEach-Object { Write-Host "  $($_.FullName)" }
}
