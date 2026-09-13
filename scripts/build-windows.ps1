# Build a zip with the Windows executable. Run from a Windows MSVC toolchain.
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    Write-Error "This packaging command must run on Windows."
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Profile = "release"
if ($args.Count -gt 0) {
    switch ($args[0]) {
        "--debug" { $Profile = "debug" }
        default {
            Write-Error "Usage: powershell -File scripts/build-windows.ps1 [--debug]"
        }
    }
}

Set-Location $Root
$Version = cargo pkgid --manifest-path (Join-Path $Root "Cargo.toml")
$Version = ($Version -split '[@#]')[-1]
$Arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    "AMD64" { "x86_64" }
    "ARM64" { "aarch64" }
    default { $env:PROCESSOR_ARCHITECTURE.ToLower() }
}
$Name = "3D-Canon-$Version-windows-$Arch"
$Dist = Join-Path $Root "dist\windows"
$Stage = Join-Path $Dist $Name
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null

$CargoArgs = @("build", "--locked", "--bin", "canon-3d", "--features", "game", "--target-dir", (Join-Path $Root "target"))
if ($Profile -eq "release") { $CargoArgs += "--release" }
cargo @CargoArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Copy-Item (Join-Path $Root "target\$Profile\canon-3d.exe") (Join-Path $Stage "3D Canon.exe")

@"
3D Canon $Version ($Arch Windows)

Requires a DirectX 12 or Vulkan capable GPU and a 64-bit Windows 10/11 install.

Run:
  "3D Canon.exe"
  "3D Canon.exe" --local
  "3D Canon.exe" --relay=HOST:3478
"@ | Set-Content -Path (Join-Path $Stage "README.txt") -Encoding utf8

$Zip = Join-Path $Dist "$Name.zip"
if (Test-Path $Zip) { Remove-Item -Force $Zip }
Compress-Archive -Path (Join-Path $Stage "*") -DestinationPath $Zip -CompressionLevel Optimal
Write-Host ""
Write-Host "Built $Arch ($Profile):"
Write-Host "  $Zip"
Write-Host "Extract the archive and run 3D Canon.exe."
