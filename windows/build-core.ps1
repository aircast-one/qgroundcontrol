#Requires -Version 5
param(
  [string]$Target = "aarch64-pc-windows-msvc",
  [string]$OutDir = (Join-Path $PSScriptRoot "src\AircastQGC\runtime")
)
$ErrorActionPreference = "Stop"
if ($env:AIRCAST_SKIP_CORE -eq "1") { exit 0 }

$llvm = "C:\Program Files\LLVM\bin"
if (Test-Path $llvm) { $env:PATH = "$llvm;$env:PATH" }

$repo = Resolve-Path (Join-Path $PSScriptRoot "..")
$core = Join-Path $repo "groundstation"

if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $env:USERPROFILE "gs-target" }

$params = Join-Path $env:USERPROFILE "gs-params"
if (-not (Test-Path (Join-Path $params "Copter-4.5"))) {
  $src = Get-ChildItem (Join-Path $repo ".cache\CPM\ardupilotparams") -Directory -ErrorAction SilentlyContinue |
    Where-Object { Test-Path (Join-Path $_.FullName "Copter-4.5") } | Select-Object -First 1
  if ($src) { robocopy $src.FullName $params /E /XD .git .github /R:5 /W:2 /NFL /NDL /NP /NJH /NJS | Out-Null }
}
if (Test-Path (Join-Path $params "Copter-4.5")) { $env:QGC_APM_PARAMS_DIR = $params }

Push-Location $core
try {
  cargo rustc --lib --release --features native-host,synthetic --crate-type cdylib --target $Target
  if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
} finally { Pop-Location }

$dll = Join-Path $env:CARGO_TARGET_DIR "$Target\release\groundstation.dll"
if (-not (Test-Path $dll)) { throw "groundstation.dll not produced at $dll" }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Copy-Item $dll (Join-Path $OutDir "groundstation.dll") -Force
"OK: $dll -> $OutDir ($((Get-Item $dll).Length) bytes)"
