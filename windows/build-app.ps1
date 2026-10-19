#Requires -Version 5
param(
  [string]$Config = "Release",
  [string]$Target = "aarch64-pc-windows-msvc",
  [string]$Rid = "win-arm64",
  [string]$Stage = "C:\aircast-head\AircastQGC"
)
$ErrorActionPreference = "Stop"

$proj = Join-Path $PSScriptRoot "src\AircastQGC"
"=== stage project locally (VirtioFS is unreliable for the many-file dotnet build) ==="
robocopy $proj $Stage /E /XD bin obj runtime /R:5 /W:2 /NFL /NDL /NP /NJH /NJS | Out-Null
"robocopy exit: $LASTEXITCODE"

"=== build core DLL -> stage runtime ==="
& (Join-Path $PSScriptRoot "build-core.ps1") -Target $Target -OutDir (Join-Path $Stage "runtime")

"=== dotnet build head ==="
Push-Location $Stage
try {
  dotnet build (Join-Path $Stage "AircastQGC.csproj") -c $Config -r $Rid --self-contained
  if ($LASTEXITCODE -ne 0) { throw "dotnet build failed ($LASTEXITCODE)" }
} finally { Pop-Location }

$exe = Join-Path $Stage "bin\$Config\net9.0-windows10.0.22621.0\$Rid\AircastQGC.exe"
if (Test-Path $exe) { "APP_OK: $exe" } else { "APP_EXE_NOT_FOUND (check build output)" }
