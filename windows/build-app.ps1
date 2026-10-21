#Requires -Version 5
param(
  [string]$Config = "Release",
  [ValidateSet("arm64", "x64")] [string]$Arch = "arm64",
  [string]$Stage = "C:\aircast-head\AircastQGC",
  [string]$GstRoot = "",
  [int]$Jobs = 4
)
$ErrorActionPreference = "Stop"

$target = @{ arm64 = "aarch64-pc-windows-msvc"; x64 = "x86_64-pc-windows-msvc" }[$Arch]
$rid = "win-$Arch"
$proj = Join-Path $PSScriptRoot "src\AircastQGC"
"=== stage project locally (VirtioFS is unreliable for the many-file dotnet build) ==="
robocopy $proj $Stage /E /XD bin obj runtime /R:5 /W:2 /NFL /NDL /NP /NJH /NJS | Out-Null
"robocopy exit: $LASTEXITCODE"
Copy-Item (Join-Path $PSScriptRoot "global.json") $Stage -Force

"=== build core DLL -> stage runtime ==="
& (Join-Path $PSScriptRoot "build-core.ps1") -Target $target -OutDir (Join-Path $Stage "runtime") -Jobs $Jobs

"=== build video DLL + GStreamer runtime -> stage runtime ==="
$video = @{ Arch = $Arch; OutDir = (Join-Path $Stage "runtime") } + $(if ($GstRoot) { @{ GstRoot = $GstRoot } } else { @{} })
& (Join-Path $PSScriptRoot "build-video.ps1") @video

"=== dotnet build head ==="
Push-Location $Stage
try {
  dotnet build (Join-Path $Stage "AircastQGC.csproj") -c $Config -r $rid --self-contained
  if ($LASTEXITCODE -ne 0) { throw "dotnet build failed ($LASTEXITCODE)" }
} finally { Pop-Location }

$exe = Join-Path $Stage "bin\$Config\net9.0-windows10.0.22621.0\$rid\AircastQGC.exe"
if (-not (Test-Path $exe)) { throw "AircastQGC.exe not produced at $exe" }
"APP_OK: $exe"
