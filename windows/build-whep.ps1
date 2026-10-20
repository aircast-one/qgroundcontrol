#Requires -Version 5
param(
  [string]$GstRoot = [Environment]::GetEnvironmentVariable("GSTREAMER_1_0_ROOT_MSVC_ARM64", "Machine"),
  [string]$Work = "C:\aircast-whep",
  [string]$Config = (Join-Path $PSScriptRoot "..\.github\build-config.json"),
  [string]$ToolchainFile = (Join-Path $PSScriptRoot "..\groundstation\rust-toolchain.toml"),
  [int]$Jobs = [Environment]::ProcessorCount
)
$ErrorActionPreference = "Stop"

$pin = (Get-Content $Config -Raw | ConvertFrom-Json).gstreamer.version.windows
$found = (Select-String -Path (Join-Path $GstRoot "lib\pkgconfig\gstreamer-1.0.pc") -Pattern '^Version:\s*(\S+)').Matches[0].Groups[1].Value
if ($found -ne $pin) { throw "GStreamer at $GstRoot is $found, the repo pins $pin (.github/build-config.json)" }
$rust = (Select-String -Path $ToolchainFile -Pattern 'channel\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$tag = "gstreamer-$pin"
$target = "aarch64-pc-windows-msvc"

$src = Join-Path $Work "gst-plugins-rs-$tag"
if (-not (Test-Path (Join-Path $src "Cargo.lock"))) {
  cmd /c "git clone -q --depth 1 --branch $tag https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs.git `"$src`" 2>&1"
  if ($LASTEXITCODE -ne 0) { throw "cloning gst-plugins-rs $tag failed" }
}

$env:PATH = "$GstRoot\bin;C:\Program Files\LLVM\bin;$env:PATH"
$env:PKG_CONFIG = Join-Path $GstRoot "bin\pkg-config.exe"
$env:PKG_CONFIG_PATH = Join-Path $GstRoot "lib\pkgconfig"
$env:CARGO_TARGET_DIR = Join-Path $Work "target"
Push-Location $src
try {
  cmd /c "cargo +$rust build --locked --release -p gst-plugin-webrtchttp --target $target -j $Jobs 2>&1"
  if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
} finally { Pop-Location }

$dll = Join-Path $env:CARGO_TARGET_DIR "$target\release\gstwebrtchttp.dll"
$bytes = [IO.File]::ReadAllBytes($dll)
if ([BitConverter]::ToUInt16($bytes, [BitConverter]::ToInt32($bytes, 0x3C) + 4) -ne 0xAA64) { throw "$dll is not ARM64" }
Copy-Item $dll (Join-Path $GstRoot "lib\gstreamer-1.0\gstwebrtchttp.dll") -Force

cmd /c "gst-inspect-1.0 whepsrc >nul 2>&1"
if ($LASTEXITCODE -ne 0) { throw "gst-inspect-1.0 cannot load whepsrc from $GstRoot" }
"OK: gst-plugins-rs $tag webrtchttp ($rust) -> $GstRoot\lib\gstreamer-1.0\gstwebrtchttp.dll"
