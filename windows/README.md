# Aircast QGC — Windows head

WinUI 3 (C#, .NET 9) UI over the Rust `groundstation` core, ARM-native (`win-arm64` + `aarch64-pc-windows-msvc`).
Feature parity target: the Android head. See `NATIVE_WINDOWS_REWRITE.md`.

## Build (on the Windows ARM64 box)

```powershell
# core DLL only
.\build-core.ps1

# video DLL + bundled GStreamer runtime only
.\build-video.ps1 -OutDir C:\aircast-head\AircastQGC\runtime

# core + video + head (stages to a local dir, dodging the VirtioFS share)
.\build-app.ps1
```

Requirements (all present on the `winvm` build box): Rust `1.95.0-aarch64-pc-windows-msvc`,
VS 2022 with the VC ARM64 toolset, LLVM/clang (a core C dep needs it), .NET SDK 9,
GStreamer MSVC arm64 at the version `.github/build-config.json` pins (`gstreamer.version.windows`).
The Windows App SDK is pulled from NuGet on first restore.

GStreamer install (elevated, silent; `C:\gstreamer` keeps `GSTREAMER_1_0_ROOT_MSVC_ARM64` valid):

```powershell
$v = '1.28.4'
Invoke-WebRequest "https://gstreamer.freedesktop.org/data/pkg/windows/$v/msvc/gstreamer-1.0-msvc-arm64-$v.exe" -OutFile gst.exe
(Get-FileHash gst.exe).Hash   # must equal checksums."1.28.4".windows_msvc_arm64 in build-config.json
.\gst.exe /VERYSILENT /SUPPRESSMSGBOXES /SP- /NORESTART /ALLUSERS /TYPE=devel /DIR=C:\gstreamer
```

Every 1.28.x arm64 installer shares one AppId: uninstall a different version (`C:\gstreamer\unins000.exe /VERYSILENT`)
first. `build-video.ps1` refuses to build against any version other than the pin.

The arm64 SDK ships no gst-plugins-rs, so `whepsrc` (WHEP video) is built from source once per box, elevated:

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
  build-core.ps1          groundstation -> groundstation.dll (aarch64)
  build-video.ps1         ../src/Bridge/QGCVideoC.cc -> qgc_video.dll (arm64) + the GStreamer runtime it needs
  build-whep.ps1          gst-plugins-rs webrtchttp -> gstwebrtchttp.dll (whepsrc) in the GStreamer root
  build-app.ps1           core + video + dotnet build, staged locally
  src/AircastQGC/
    Core/                 NativeMethods, CoreHost, Bridge, BridgeWatch  (the core binding)
                          VideoNative, VideoDriver                      (the video binding)
    App.xaml(.cs)         boots the core, opens MainWindow
    MainWindow.xaml(.cs)  shell: title bar, nav pane, Fly view composed from Controls/
    Controls/             one UserControl per Fly surface, each watching its own view.* paths
```

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
