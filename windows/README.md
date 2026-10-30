# Aircast QGC — Windows head

WinUI 3 (C#, .NET 9) UI over the Rust `groundstation` core, built for `win-arm64` (`aarch64-pc-windows-msvc`) and
`win-x64` (`x86_64-pc-windows-msvc`).
Feature parity target: the Android head. See `NATIVE_WINDOWS_REWRITE.md`.

## Build (on the Windows ARM64 box)

```powershell
# core DLL only
.\build-core.ps1

# video DLL + bundled GStreamer runtime only
.\build-video.ps1 -OutDir C:\aircast-head\AircastQGC\runtime

# core + video + head (stages to a local dir, dodging the VirtioFS share); -Arch x64 on an x64 box
.\build-app.ps1 -Arch arm64
```

Requirements (all present on the `winvm` build box): Rust `1.95.0-aarch64-pc-windows-msvc`,
VS 2022 with the VC ARM64 (or x64) toolset, LLVM/clang (a core C dep needs it), the .NET SDK `global.json` pins,
GStreamer MSVC arm64 (or x86_64) at the version `.github/build-config.json` pins (`gstreamer.version.windows`).
The Windows App SDK is pulled from NuGet on first restore.

GStreamer install (elevated, silent): `install-gstreamer.ps1` downloads the pinned installer (S3 mirror as fallback),
checks it against the build-config checksum, installs it and prints the SDK root:

```powershell
.\install-gstreamer.ps1 -Arch arm64 -Dir C:\gstreamer   # C:\gstreamer keeps GSTREAMER_1_0_ROOT_MSVC_ARM64 valid
```

Every 1.28.x arm64 installer shares one AppId: uninstall a different version (`C:\gstreamer\unins000.exe /VERYSILENT`)
first. `build-video.ps1` refuses to build against any version other than the pin.

The arm64 SDK ships no gst-plugins-rs (the x64 SDK does), so on arm64 `whepsrc` (WHEP video) is built from source once
per box, elevated:

```powershell
.\build-whep.ps1        # add -Jobs 2 on a shared box
```

It clones gst-plugins-rs at `gstreamer-<pin>` (the tag cerbero ships with that GStreamer; 1.28.4 -> 0.15.2, commit
`e2e244c3`), builds `gst-plugin-webrtchttp` for `aarch64-pc-windows-msvc` with the `groundstation` Rust pin
(`cargo build --locked --release -p gst-plugin-webrtchttp`, rustls TLS, no OpenSSL) against the SDK's pkg-config, and
installs `gstwebrtchttp.dll` into `<GStreamer root>\lib\gstreamer-1.0`, where the x64 SDK keeps it. `webrtchttp` is in the
build-config plugin list, so `build-video.ps1` bundles it with its `webrtc`/`nice`/`dtls`/`srtp`/`sctp`/`rtpmanager` deps.

## Layout

```text
windows/
  global.json             .NET SDK pin
  install-gstreamer.ps1   pinned, checksum-verified GStreamer MSVC SDK install
  build-core.ps1          groundstation -> groundstation.dll (-Target)
  build-video.ps1         ../src/Bridge/QGCVideoC.cc -> qgc_video.dll (-Arch) + the GStreamer runtime it needs
  build-whep.ps1          gst-plugins-rs webrtchttp -> gstwebrtchttp.dll (whepsrc) in the GStreamer root
  build-app.ps1           core + video + dotnet build for -Arch arm64|x64, staged locally
  src/AircastQGC/
    Core/                 NativeMethods, CoreHost, Bridge, BridgeWatch  (the core binding)
                          VideoNative, VideoDriver                      (the video binding)
    App.xaml(.cs)         boots the core, opens MainWindow
    MainWindow.xaml(.cs)  shell: title bar, nav pane, Fly view composed from Controls/
    Controls/             one UserControl per Fly surface, each watching its own view.* paths
```

## Release builds

`.github/workflows/windows-native.yml` builds both architectures (`windows-11-arm`, `windows-2022`), boots each app to
an idle window and uploads `AircastQGCNative-<version>-windows-<arch>.zip`. `aircast-release.yml` calls it for every
release tag; it can also be run by hand (`gh workflow run windows-native.yml -f ref=main`).

## Notes

- Build off a local `C:` path, not the `Z:\` VirtioFS share — deep reads flake with OS error 53.
  `build-core.ps1` keeps `CARGO_TARGET_DIR` local and copies the ArduPilot param set locally
  (`QGC_APM_PARAMS_DIR`); `build-app.ps1` stages the project to `C:\aircast-head`.
- Do not set `AppWindow.TitleBar.PreferredTheme`: on WindowsAppSDK 1.8 it blanks the whole window to black
  (UI tree alive, nothing presented). The dark app theme already themes the caption buttons.
- WinUI clamps `CornerRadius` per axis, so `RadiusPill` (999) draws an ellipse on anything not square.
  Give pills an explicit height and a radius of half of it.
- Video is the same C library every head uses (`src/Bridge/QGCVideoC.cc`, interface version 4).
  `VideoDriver` runs on its own thread every 500 ms, like the iOS driver: it plays `view.video.nativePipeline` on
  channel 0 and `pipPipeline` on channel 1, honours `qgc_core_video_take_restart`, follows `nativeRecording`, and calls
  `video.reportNative` / `video.reportRecording`. `VideoSurface(Channel)` copies BGRA frames into a `WriteableBitmap`
  at most every 33 ms (CPU path, like macOS). `VideoThumbnail` shows it once `view.video.decoding` is true.
- The bundled runtime sits next to the exe: `qgc_video.dll`, `gstreamer\bin` (all SDK DLLs), `gstreamer\lib\gstreamer-1.0`
  (only the build-config plugins), `gstreamer\libexec`. `VideoNative` puts `gstreamer\bin` first on `PATH`, points the
  plugin path and scanner there and keeps its own registry in `%LOCALAPPDATA%\AircastQGC`. Without the DLLs video is off
  and the app still runs. The first start scans the plugins (about 10 s on the VM); later starts use the registry.
- `build-app.ps1 -Version 5.6.5-beta.21` (and `deploy-vm.sh -Version …`) stamps the assembly; the head passes it to the core
  as `--app-version`, which drives the in-app update check against the GitHub releases. A build without it reports `dev`
  and never checks. The update installs the downloaded `-setup.exe /S`, which relaunches the app; a copy running outside
  the per-user install folder (the portable zip) is only pointed at the release page.
- `deploy-vm.sh` never rebuilds the native DLLs: it keeps the stage's `runtime\`. Run `build-video.ps1` (or `build-app.ps1`)
  after changing `QGCVideoC.cc` or the GStreamer pin.
- The app passes `--save-path Documents\Aircast QGC` (as iOS does); without it the core finds no save folder on Windows
  (it looks for `HOME`) and refuses to record. It also passes `--settings %LOCALAPPDATA%\Aircast\Aircast QGC.ini` and
  `--map-cache %LOCALAPPDATA%\Aircast\QGCMapCache\qgcMapCache.db`: without them the settings land under the launch folder and there is
  no tile cache, so offline maps are unavailable.
- WHEP (cloud/SFU video) plays through the `whepsrc` that `build-whep.ps1` builds; verified on the VM against a local
  MediaMTX. RTP/UDP H.264 is verified on the VM; RTSP, H.265 and MPEG-TS plugins are bundled but untested. The VM has no
  DXVA, so it decodes with `avdec_*`.
- Unpackaged, self-contained WinUI 3 (no MSIX yet). MSIX/Store packaging is a later step
  (`../deploy/windows` has the older Qt plan for reference only).
