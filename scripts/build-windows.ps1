# Builds the Windows installer (.exe) on a Windows machine and stages it in
# release/windows/ together with its SHA-256. Run from PowerShell:
#   powershell -ExecutionPolicy Bypass -File scripts/build-windows.ps1
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    npm ci
    npm run build
    npx --yes "@tauri-apps/cli@^2" build --bundles nsis

    $version = (Get-Content package.json -Raw | ConvertFrom-Json).version
    $installer = "src-tauri/target/release/bundle/nsis/SeedFinder_${version}_x64-setup.exe"
    if (-not (Test-Path $installer)) {
        throw "Installer not found: $installer"
    }

    New-Item -ItemType Directory -Force release/windows | Out-Null
    Copy-Item $installer release/windows/
    $name = Split-Path $installer -Leaf
    $hash = (Get-FileHash "release/windows/$name" -Algorithm SHA256).Hash.ToLower()
    "$hash  $name" | Set-Content release/windows/SHA256SUMS-windows.txt

    Write-Host "Windows installer: release/windows/$name"
} finally {
    Pop-Location
}
