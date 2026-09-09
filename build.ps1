# Toykit v2 firmware build entry-point (PowerShell).
#
#  .\build.ps1             -> release build of all 3 bins
#  .\build.ps1 debug       -> debug build
#  .\build.ps1 check       -> cargo check
#  .\build.ps1 uf2         -> release + objcopy + hex-to-uf2
#
# Always sets the three env vars the project needs.

$ErrorActionPreference = 'Stop'

Set-Location $PSScriptRoot

$env:CARGO_TARGET_DIR = 'C:\build\toykit-target'
$env:LIBCLANG_PATH    = 'C:\QMK_MSYS\mingw64\bin'
$env:PATH             = 'C:\QMK_MSYS\mingw64\bin;' + $env:PATH

$Mode = if ($args.Count -gt 0) { $args[0] } else { 'release' }

switch ($Mode) {
    'debug'    { $Cmd = 'build'; $Flag = '--debug' }
    'release'  { $Cmd = 'build'; $Flag = '--release' }
    'check'    { $Cmd = 'check'; $Flag = '' }
    'uf2'      { $Cmd = 'build'; $Flag = '--release' }
    default {
        Write-Host "usage: .\build.ps1 [debug|release|check|uf2]"
        exit 1
    }
}

Write-Host ">>> rustc: $(rustc --version)"
Write-Host ">>> target: $($env:CARGO_TARGET_DIR)"
Write-Host ">>> profile: $Mode"
Write-Host ""

foreach ($Bin in @('central','peripheral','peripheral2')) {
    Write-Host ">>> $Cmd $Bin ($Mode)"
    cargo $Cmd $Flag --bin $Bin
}

if ($Mode -eq 'uf2') {
    Write-Host ""
    Write-Host ">>> generating hex + uf2"
    $RL = Join-Path $env:CARGO_TARGET_DIR 'thumbv7em-none-eabihf\release'
    rust-objcopy -O ihex "$RL\central"     rmk-central.hex
    rust-objcopy -O ihex "$RL\peripheral"  rmk-peripheral.hex
    rust-objcopy -O ihex "$RL\peripheral2" rmk-peripheral2.hex
    cargo hex-to-uf2 --input-path rmk-central.hex     --output-path rmk-central.uf2     --family nrf52840
    cargo hex-to-uf2 --input-path rmk-peripheral.hex  --output-path rmk-peripheral.uf2  --family nrf52840
    cargo hex-to-uf2 --input-path rmk-peripheral2.hex --output-path rmk-peripheral2.uf2 --family nrf52840
}

Write-Host ""
Write-Host ">>> artifacts:"
Get-ChildItem "$env:CARGO_TARGET_DIR\thumbv7em-none-eabihf\release\central",
              "$env:CARGO_TARGET_DIR\thumbv7em-none-eabihf\release\peripheral",
              "$env:CARGO_TARGET_DIR\thumbv7em-none-eabihf\release\peripheral2" -ErrorAction SilentlyContinue |
    Format-Table Name, Length, LastWriteTime