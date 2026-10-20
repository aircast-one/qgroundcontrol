#Requires -Version 5
param(
  [ValidateSet("arm64", "x64")] [string]$Arch = "arm64",
  [string]$Dir = "C:\gstreamer",
  [string]$Config = (Join-Path $PSScriptRoot "..\.github\build-config.json")
)
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$pins = Get-Content $Config -Raw | ConvertFrom-Json
$version = $pins.gstreamer.version.windows
$sha = $pins.gstreamer.checksums.$version.(@{ arm64 = "windows_msvc_arm64"; x64 = "windows_msvc_x64" }[$Arch])
if (-not $sha) { throw "build-config.json pins no checksum for GStreamer $version $Arch" }
$file = "gstreamer-1.0-msvc-$(@{ arm64 = 'arm64'; x64 = 'x86_64' }[$Arch])-$version.exe"
$exe = Join-Path $env:TEMP $file

$fetched = "https://gstreamer.freedesktop.org/data/pkg/windows/$version/msvc/$file",
  "https://qgroundcontrol.s3.us-west-2.amazonaws.com/dependencies/gstreamer/windows/$file" |
  Where-Object {
    try {
      Invoke-WebRequest $_ -OutFile $exe -UseBasicParsing
      (Get-FileHash $exe -Algorithm SHA256).Hash -eq $sha
    } catch { $false }
  } | Select-Object -First 1
if (-not $fetched) { throw "no mirror served $file with SHA256 $sha" }
Write-Host "GStreamer $version $Arch from $fetched (SHA256 verified)"

$log = Join-Path $env:TEMP "gstreamer-$Arch-install.log"
$run = Start-Process $exe -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/SP-", "/NORESTART", "/LOG=`"$log`"", "/DIR=`"$Dir`"" -Wait -PassThru
if ($run.ExitCode -ne 0) { throw "GStreamer installer exited $($run.ExitCode); see $log" }

$pc = Get-ChildItem $Dir -Recurse -Filter "gstreamer-1.0.pc" | Where-Object { $_.Directory.Name -eq "pkgconfig" } | Select-Object -First 1
if (-not $pc) { throw "no lib\pkgconfig\gstreamer-1.0.pc under $Dir after install" }
$pc.Directory.Parent.Parent.FullName
