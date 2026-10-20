#Requires -Version 5
param(
  [string]$GstRoot = [Environment]::GetEnvironmentVariable("GSTREAMER_1_0_ROOT_MSVC_ARM64", "Machine"),
  [string]$OutDir = (Join-Path $PSScriptRoot "src\AircastQGC\runtime"),
  [string]$Source = (Join-Path $PSScriptRoot "..\src\Bridge"),
  [string]$Config = (Join-Path $PSScriptRoot "..\.github\build-config.json")
)
$ErrorActionPreference = "Stop"

$pins = Get-Content $Config -Raw | ConvertFrom-Json
$pin = $pins.gstreamer.version.windows
$found = (Select-String -Path (Join-Path $GstRoot "lib\pkgconfig\gstreamer-1.0.pc") -Pattern '^Version:\s*(\S+)').Matches[0].Groups[1].Value
if ($found -ne $pin) { throw "GStreamer at $GstRoot is $found, the repo pins $pin (.github/build-config.json)" }

$vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * `
  -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64 -property installationPath
if (-not $vs) { throw "no Visual Studio with the ARM64 C++ tools" }

$tmp = Join-Path $env:TEMP "qgc-video-build"
New-Item -ItemType Directory -Force -Path $tmp, $OutDir | Out-Null
Copy-Item (Join-Path $Source "QGCVideoC.cc"), (Join-Path $Source "QGCVideoC.h") $tmp -Force

$exports = Select-String -Path (Join-Path $tmp "QGCVideoC.h") -Pattern '\b(qgc_video_\w+)\s*\(' -AllMatches |
  ForEach-Object { $_.Matches } | ForEach-Object { "/EXPORT:$($_.Groups[1].Value)" } | Sort-Object -Unique
$dll = Join-Path $OutDir "qgc_video.dll"
$includes = "include\gstreamer-1.0", "include\glib-2.0", "lib\glib-2.0\include" | ForEach-Object { "/I`"$(Join-Path $GstRoot $_)`"" }
$libs = "gstreamer-1.0.lib gstapp-1.0.lib gstvideo-1.0.lib gobject-2.0.lib glib-2.0.lib"
$cl = "cl /nologo /LD /O2 /MD /EHsc /std:c++20 /utf-8 /W3 /DQGC_GST_STREAMING /I`"$tmp`" $($includes -join ' ') " +
  "`"$tmp\QGCVideoC.cc`" /Fo`"$tmp\\`" /Fe`"$dll`" /link /IMPLIB:`"$tmp\qgc_video.lib`" /LIBPATH:`"$GstRoot\lib`" $libs $($exports -join ' ')"
$vcvars = Join-Path $vs "VC\Auxiliary\Build\vcvarsarm64.bat"
cmd /c "call `"$vcvars`" >nul && $cl && dumpbin /nologo /headers `"$dll`" | findstr /c:`"machine`"" | Tee-Object -Variable built
if ($LASTEXITCODE -ne 0) { throw "qgc_video.dll build failed ($LASTEXITCODE)" }
if (-not ($built -match 'AA64 machine \(ARM64\)')) { throw "qgc_video.dll is not ARM64" }

$gst = Join-Path $OutDir "gstreamer"
robocopy (Join-Path $GstRoot "bin") (Join-Path $gst "bin") *.dll /R:2 /W:1 /NFL /NDL /NP /NJH /NJS | Out-Null
robocopy (Join-Path $GstRoot "libexec\gstreamer-1.0") (Join-Path $gst "libexec\gstreamer-1.0") /E /R:2 /W:1 /NFL /NDL /NP /NJH /NJS | Out-Null
$plugins = @($pins.gstreamer.plugins.common) + @($pins.gstreamer.plugins.windows) | ForEach-Object { "gst$_.dll" } |
  Where-Object { Test-Path (Join-Path $GstRoot "lib\gstreamer-1.0\$_") }
robocopy (Join-Path $GstRoot "lib\gstreamer-1.0") (Join-Path $gst "lib\gstreamer-1.0") $plugins /R:2 /W:1 /NFL /NDL /NP /NJH /NJS | Out-Null
if ($LASTEXITCODE -ge 8) { throw "staging the GStreamer runtime failed (robocopy $LASTEXITCODE)" }
$global:LASTEXITCODE = 0
"OK: $dll + GStreamer $found runtime ($($plugins.Count) plugins) -> $gst"
