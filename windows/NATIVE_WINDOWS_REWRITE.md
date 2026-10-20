# Native Windows head — parity ledger

WinUI 3 (C#, .NET 9) UI over the Rust `groundstation` core, feature parity with the Android head.
Same contract as every other head: the UI is thin and renders JSON views addressed by path, pushed
through `qgc_bridge_*` (see `src/AircastQGC/Core/`). ARM-native: core `aarch64-pc-windows-msvc` DLL,
app `win-arm64`.

## Architecture

| Layer | Where | Mirrors |
| --- | --- | --- |
| Core (Rust) → `groundstation.dll` | `../groundstation`, built by `build-core.ps1` | android `libgroundstation.so`, macOS staticlib |
| P/Invoke surface | `Core/NativeMethods.cs` | Swift `QGCBridgeC`, Kotlin `QGCBridge` |
| Bridge (get/set/invoke) | `Core/Bridge.cs` | macOS `Bridge.swift` |
| Watch + event dispatch | `Core/BridgeWatch.cs` | macOS `BridgeWatch.swift` |
| Lifecycle (`qgc_start`/shutdown) | `Core/CoreHost.cs` | `nativehost.rs`, macOS `AppShell` |
| Video (`qgc_video.dll` from `src/Bridge/QGCVideoC.cc`, GStreamer 1.28.4 arm64) | `Core/VideoNative.cs`, `Core/VideoDriver.cs`, `Controls/VideoSurface.cs` | iOS `VideoDriver`, macOS `NativeVideoView` |

Boot: `qgc_start(argc, argv)` spawns the core pump thread and returns; the UI never calls `qgc_run`
(headless-only). Data flows through `view.*` paths; writes via `qgc_bridge_set`; actions via
`qgc_bridge_invoke`; live updates via `qgc_bridge_watch_client` + the event handler.

## Foundation status

- [x] Core builds for `aarch64-pc-windows-msvc` (`build-core.ps1`) — 28 MB DLL
- [x] P/Invoke bridge + lifecycle (`Core/`)
- [x] App boots, `qgc_start` returns 0, window renders (win-arm64, runs stable)
- [x] Design tokens from Penpot → `Theme/Tokens.xaml` (dark-first, Fluent-aligned) + `Theme/Styles.xaml`
- [x] Deploy loop: `scp` files → `buildrun.ps1` (kill-before-build) on `winvm`
- [x] Shared components as UserControls in `Controls/` — `ShellTitleBar`, `NavPane`; MainWindow only composes and wires them
- [x] Fly view composed from core-driven controls (map, instruments, guided, video/camera, flight modes, checklist, no-vehicle), verified on `winvm` against a mock ArduCopter

## Screens (Penpot "06 Screens", 54 boards @ 1440×900)

Status: ▢ todo · ◐ in progress · ✓ done

### Fly

- ✓ fly · ◐ fly — flight mode (refused-change notice untested: the mock accepts every mode) · ◐ fly — camera (frames ✓; the PiP tile is not built and WHEP sources cannot play: no `whepsrc` in the arm64 SDK) · ✓ fly — map context menu
- ✓ fly — armed, confirm return · ✓ fly — checklist · ✓ fly — no vehicle

Driven through the UI on `winvm` against the mock ArduCopter (2026-10-20): slide-to-confirm with real mouse input, arm, disarm,
takeoff to a chosen height, change altitude, title-bar mode change, ROI + cancel ROI, go-to, set heading, RTL, land to
auto-disarm; the context menu and the confirm card close on Escape and the menu on an outside click. Orbit is not offered
for ArduCopter by the core.

Video on `winvm` (2026-10-20): the mock ArduCopter's camera (`camera=1`) advertises `udp://127.0.0.1:5600`, fed by a
`videotestsrc` → `x264enc` → `rtph264pay` sender. The core's `nativePipeline` played through the bundled GStreamer 1.28.4,
`view.video` went to decoding at 1280×720 in about 10 s (first plugin scan), and the moving test ball was seen in the
thumbnail and, after the swap, in the full-size view. Killing the sender showed "Waiting for a stream. The stream from
127.0.0.1:5600 stopped."; restarting it brought the moving picture back. `video.startRecording` wrote a growing MP4 under
`Documents\Aircast QGC\Video` (6.9 s, 1280×720 H.264 by `gst-discoverer`) and `view.video.recording` followed
`video.reportRecording`. Not checked: H.265, RTSP, MPEG-TS, hardware decode (the VM has no DXVA).

### Plan

- ▢ plan · ▢ plan — replace plan · ▢ plan — upload warning · ▢ plan — terrain profile
- ▢ plan — fence & rally · ▢ plan — item editor · ▢ plan — command picker

### Vehicle setup

- ▢ vehicle-setup · ▢ setup — sensors calibration · ▢ setup — radio · ▢ setup — flight modes
- ▢ setup — power · ▢ setup — safety · ▢ setup — motors · ▢ setup — frame
- ▢ setup — parameters · ▢ setup — parameter editor · ▢ setup — flight behavior
- ▢ setup — remote support · ▢ setup — camera · ▢ setup — lights · ▢ setup — tuning

### Analyze

- ▢ analyze — mavlink inspector · ▢ analyze — mavlink console · ▢ analyze — telemetry log
- ▢ analyze — log download · ▢ analyze — geotag images · ▢ analyze — vibration

### Settings

- ▢ settings · ▢ settings — general · ▢ settings — fly view · ▢ settings — plan view
- ▢ settings — video · ▢ settings — maps · ▢ settings — connections · ▢ settings — add link
- ▢ settings — comm links · ▢ settings — mavlink · ▢ settings — flight modes · ▢ settings — adsb server
- ▢ settings — packet radio · ▢ settings — remote id · ▢ settings — rtk gps
- ▢ settings — firmware upgrade · ▢ settings — 3d viewer · ▢ settings — about
