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
- [x] Every nav page hosted (`Pages/PageRegistry`): Fly, Plan, Analyze, Vehicle Setup, Settings. Merged build smoke on `winvm` (2026-10-20): each page opens with no vehicle, the mock ArduCopter and the mock PX4; a plan uploaded from Plan shows on Fly with the vehicle and traffic; a PX4 calibration still blocks the nav pane and Back
- [x] Second merged build (2026-10-20, video, settings, plan and setup work plus the core fixes below) smoke-tested on `winvm`: every page opens with no vehicle, the mock ArduCopter and the mock PX4; PiP and the camera swap, Video sources add, the Fly RC layer, the offline map picker, the Plan mid-leg +, Follow vehicle and presets, live parameter rows and a UI Automation tuning write, the power calculator and the Add link type list all work together

## Screens (Penpot "06 Screens", 54 boards @ 1440×900)

Status: ▢ todo · ◐ in progress · ✓ done

### Fly

- ✓ fly · ◐ fly — flight mode (refused-change notice untested: the mock accepts every mode) · ◐ fly — camera (frames ✓, picture-in-picture ✓, video fit ✓; WHEP not seen in the shipped build: `build-whep.ps1` put `gstwebrtchttp.dll` in `C:\gstreamer`, but the stage's bundled runtime predates it until `build-video.ps1` is rerun) · ✓ fly — map context menu
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

Picture-in-picture and video fit on `winvm` (2026-10-20): two UDP H.264 senders on 5711/5712 added as sources through Settings >
Video sources. The PiP toggle in the video tile writes `multiViewEnabled`. The 160×90 PiP tile (`Controls/PipTile`, channel 1) reports
`video.setPipShown`, and its frames moved alongside the main channel. Pressing the tile makes that camera the main one
(`video.setActiveVideoSource`), and the core moves the old main into the PiP. Turning the toggle off clears `pipPipeline`.
`settings.videoSettings.videoFit` (Fit Width, Fit Height, Fill, No Crop) is applied to the main view as Android `videoContentSize` does,
checked with 1280×360 and 720×720 streams; the PiP always draws No Crop, like Android's second pane. Re-checked in the merged build.

On-screen RC controls (`Controls/RcControlsLayer`, on Fly while connected) from `view.rcControls`, `view.flyState` and the Fly view camera
channels: slider, button, three-way switch and momentary controls plus Tilt/Pan/Zoom sliders, Record, Light, Recenter and Give back. It
sends `vehicle.setRcChannelOverride` at most every 100 ms with a final send. Against the mock ArduCopter a UI Automation slider set gave
pwm 1800 held in `view.rcControls`, `vehicle.rcChannelOverrides` `{6:1800}` and `rcOverride` true; Give back emptied the overrides. Not
driven: holding a Momentary control and pointer slider drags. Overrides stay held when you leave Fly (FlyContent is collapsed, not
unloaded); Android releases them when the layer leaves composition. Android's gimbal-manager controls in RcCameraControls are not ported
(no gimbal indicator yet).

### Plan

- ◐ plan (map tap-to-add and real pointer drags not driven: no pointer input on the shared VM; each drag handle's drop handler was driven in-process) · ✓ plan — replace plan · ✓ plan — upload warning · ✓ plan — terrain profile
- ✓ plan — fence & rally · ✓ plan — item editor · ✓ plan — command picker

One root page (`Pages/Plan/PlanPage`) on a second `MapView` in plan mode. Every `MapView` now owns its BridgeWatch ids
(`mapview.<guid>.<path>`) and stops them on Unloaded, so the Plan map cannot take over or stop the Fly map's watches. Driven on `winvm`
with no vehicle, the mock PX4 and the mock ArduCopter (2026-10-20) through UI Automation and the debug API: add (rail, centre, debug
`mission.insert`), select, edit altitude, change command, edit position, delete and undo items; survey template, size to view and hover
capture; fence polygon, circle, breach point, corner edit, remove vertex and undo; rally add, edit and remove; terrain profile and its
toggle; defaults and transform; save, open, export KML and import boundary through the WinAppSDK 1.8 pickers (unpackaged); the vehicle
connected and disconnected prompts; replace, clear and upload-warning dialogs (firmware mismatch, far from aircraft); upload to the green
Fly pill and on to Fly, which still draws the vehicle, the uploaded route, pins and live ADS-B traffic. Not driven: map tap, pin and handle
drags, terrain-chart marker taps.

Added and driven on `winvm` (2026-10-20): a mid-leg + on the leg into the selected item inserts a waypoint at its midpoint (UIA Invoke;
re-checked in the merged build against the mock PX4). The landing pattern is drawn (approach path, loiter ring), with Approach/Touchdown
handles on the selected landing. A selected fence circle gets a Radius handle; fence polygons and survey/structure areas get a Move centre
handle. Each handle's drop handler was driven in-process and its write read back: `finalApproachCoordinate`/`landingCoordinate`,
`circles.N.radius`, clear+`appendVertices`. Pattern presets (save with blank and "/" refused, apply, delete with confirmation) go through
`core.plan.savePreset`/`applyPreset`/`deletePreset` and `view.patternPresets`. Centre > Follow vehicle follows the flying mock and any
re-centre turns it off. Delete, Escape, Ctrl+Z/Y/S/O were sent as window messages posted to the app's own input window, and are ignored
while a dialog is open. Not driven: map tap, real pointer drags, terrain-chart marker taps. Not ported: Centre > My location (Windows has
no ground-station position source). In plan mode a double-click adds a waypoint instead of zooming. Landing handles appear only while that
landing is selected (Android draws them always). The centre handle moves a shape by its vertex average, not Android's area centroid.
`view.missionItems` is still polled every 700 ms, as on Android, but off the UI thread; it skips parse and redraw when the payload is
unchanged, and the overlay re-projects only when the map view moves (`MapView.ViewChanged` now fires only on a real change). With a
~4,200-item hover-capture survey (140 KB), the worst idle UI stall fell from 51 to 28 ms and opening the plan menu from 242 to 94 ms. The
core still needs ~240 ms to rebuild that view after each edit, and that rebuild runs on the UI thread for user edits.

### Vehicle setup

- ✓ vehicle-setup · ◐ setup — sensors calibration (CompassMot not run; ESC calibration never gets past "Starting ESC calibration..." on the PX4 mock, and the APM mock disables its Calibrate button) · ◐ setup — radio (calibration started and cancelled, not run to the end; Spektrum/CRSF bind not run) · ✓ setup — flight modes
- ✓ setup — power (the core's mock now reports battery ids 0 and 1 like the firmware; the voltage calculator reads 12.60 V on the APM mock; amps per volt shows no reading because the mock draws 0 A at rest) · ✓ setup — safety · ✓ setup — motors · ✓ setup — frame
- ✓ setup — parameters · ✓ setup — parameter editor · ✓ setup — flight behavior
- ✓ setup — remote support · ▢ setup — camera (the mocks expose no camera setup page) · ▢ setup — lights (the mocks expose no lights component) · ✓ setup — tuning

One root page (`Pages/Setup/SetupPage`: two panes at 840 px and wider, one pane below with "Back to Aircraft setup", like Analyze and Settings) over `view.setup`, reusing the `Forms/` engine (which now raises `Rejected` so
the parameter editor can offer "Force save (dangerous!)" after a refused write, as Android does). Airframe pictures, Android's setup section
images and its calibration position pictures ship as `Assets/Airframe/*.svg`, `Assets/SetupSections/*.svg` (recoloured to the text
colour, <style> rules inlined) and `Assets/Calibration/*.png`, loaded by `Forms/AssetImages`. Driven on `winvm` with no vehicle, the mock ArduCopter and the mock PX4 (2026-10-20) through UI Automation and
the debug API: the no-vehicle state and "Set up connection" to Settings > Links; the summary with each component's line; parameters by
group, search, the "Changed" filter (PX4), edit and read back, range refusal then force save, manual entry for an option list, Tools
(Refresh, Reset all confirmation cancelled); APM accelerometer calibration run to "YOU MUST REBOOT"; APM compass calibration (dialog, result prompt with fitness bars); PX4 ESC
calibration dialog opened; parameter file Save to file... and Load from file for review... through the Windows pickers (file written, one
changed parameter reviewed and restored); PX4 RC to param dialog sent; PX4 gyro calibration started, the
nav pane, Back and the setup list refused with "Complete or cancel the current calibration first" until the link dropped (the PX4 mock
never answers a cancel); radio Mode 1/2 and the calibration start/cancel; flight mode, battery capacity, RTL altitude, frame class/type,
servo range, PX4 return altitude and responsiveness edits, each read back and restored; motor test gates (props-off switch, refused
while armed); PX4 Airframe Apply confirmation (cancelled); APM basic/advanced tuning with restore from clipboard, PX4 PID tuning. Not
checked: PX4 Actuators (no ActuatorComponent on the mock), CompassMot, ESC calibration to completion, joystick with a real gamepad
(including the live calibration monitor, which now builds a fresh host each render), slider drags and keys (no pointer or keyboard input on
the shared VM), Sub/Rover-only pages. Visible parameter rows, form sliders and tuning sliders follow vehicle pushes, as Android rows do
(re-checked in the merged build: ATC_RAT_RLL_P 0.135 → 0.15 → 0.135 pushed through the debug API showed in the row). Sliders write on
pointer release, on key up, or 400 ms after a UI Automation `RangeValue` set, one write per settled value (counted on the MAVLink
inspector; in the merged build a set to 0.26 wrote ATC_RAT_RLL_P). Setup form controls (`vehicleConfig(...)`) cannot be read back from the
core, so their sliders watch `FormControl.LivePath`, the underlying parameter. The core now names each component's `page` and `screen` in
`view.setup` (PX4 Airframe → Frame/px4Airframe, PID Tuning → Tuning/px4Tuning), but `SetupModel` still uses its own name table.

### Analyze

- ✓ analyze — mavlink inspector · ✓ analyze — mavlink console · ✓ analyze — telemetry log
- ✓ analyze — log download · ◐ analyze — geotag images (a successful tag was not seen: no sample log with camera events) · ✓ analyze — vibration

One root page (`Pages/Analyze/AnalyzePage`) with Android's list and detail, plus Messages and Firmware, driven on `winvm` against the mock
ArduCopter and the mock PX4 (2026-10-20). Seen working: every tool with no vehicle (gated tools show "Requires a connected vehicle");
the list's live subtitles and badges (vibration level, messages/s, unread count); inspector filter, component and system chips with two
vehicles, live field values, fields charted on Plot 1 and 2, the time scale read back from `view.inspectorCharts`; console `ver`, an
unknown command and history up/down on the PX4 mock (the ArduPilot mock has no shell); messages from the PX4 mock, reset on open, Clear;
log list, select, download into `Documents\Aircast QGC\Logs`, erase all with Cancel and with Erase All, the list refreshed after
`vehicles.setActive`; geotag file and folder pickers (WinAppSDK 1.8, unpackaged), time offset and preview mode written to `geoTag`, a
preview run listing each image as skipped; vibration bars, clipping and the unsafe caveat; firmware source choice written to
`settings.firmwareUpgradeSettings`, advanced sources and the beta warning; the telemetry-log bar on Fly and Analyze loading `mav.tlog`
through the picker, playing, 5× speed, seek to 75 %, close. Navigation is refused while the log-download lock is held (nav pane and
switching tool), with a notice that clears itself; the lock was held by a scratch hook because the mock's 1000-byte log
downloads within one poll. An Analyze page narrower than 840 px shows the list or one tool with a back button, as on Android.
Not checked: flashing (no USB board on the VM), a message-rate change taking effect (the mock never reports MESSAGE_INTERVAL).
`Controls/Px4LogTransfer` is hosted as Settings > PX4 log transfer (verified there against the PX4 mock).

### Settings

- ✓ settings · ✓ settings — general · ✓ settings — fly view (on-screen RC controls editor; the Fly layer is described under Fly) · ✓ settings — plan view
- ✓ settings — video · ✓ settings — maps (Add new set opens a map picker; dragging its handles was not driven: no pointer input on the shared VM) · ✓ settings — connections
- ◐ settings — add link (Wi-Fi or network, USB telemetry radio, Network server (TCP), Replay a flight log and Simulated vehicle are offered and the 19 Windows baud rates listed; no COM port seen: the VM has none; Bluetooth is not offered: no Windows Bluetooth host; Aircast Cloud is in `linkTypeIds` but the head never offers it)
- ✓ settings — comm links · ◐ settings — mavlink (signing Disable not seen: the mock never completes signing) · ✓ settings — flight modes · ✓ settings — adsb server
- ◐ settings — packet radio (no Wi-Fi adapter on the VM: only "Unavailable" and the Automatic adapter seen) · ◐ settings — remote id (the GCS position block never shows: Windows has no ground-station position source) · ✓ settings — rtk gps
- ◐ settings — firmware upgrade (Analyze's FirmwareTool; flashing not checked, no USB board on the VM) · ✓ settings — 3d viewer · ✓ settings — about

One root page (`Pages/Settings/SettingsPage`, code-only) over `view.settings`, with the shared form engine in `Forms/` (also exposed as
`FormView.SetupPage` for Vehicle Setup). Driven on `winvm` with no vehicle and against the mock ArduCopter and PX4 (2026-10-20). Seen working:
every page renders; toggle, number, choice, units (Metric/Imperial/Custom) and text writes read back with `bridge/get`; enabled/disabled
reasons follow their switch; inline refusal ("Value must be 23 characters or less"); Reset all settings with Cancel, OK and Cancel reset;
search with page and setting hits and no-hit text; the back stack; below 840 px the page shows the list or one page with a back button, and
search results stay in the list pane. Links: add UDP (port validation), TCP (missing host refused), edit port/name/connect-on-start, connect,
disconnect, delete with Cancel and Remove, the mock's auto link and Stop, the Advanced auto-connect form; "Open connections" and the title-bar
link pill land on Links. MAVLink: add a passphrase key (length gate), Enable with the send-key confirmation, Export to the clipboard, Delete
with confirmation, live link status. Maps: the tile cache, Add new set with a live estimate, download with live progress, details, rename,
cancel, resume, delete, export and import of `.qgctiledb`. NTRIP connect, the reconnect countdown and stop; PX4 log transfer with the PX4
mock; MAVLink actions file choice and `<None>`; OSM file picker; Console live tail, level/category/text/regex filters (invalid regex flagged),
categories dialog, Save as .txt, Clear; About's version tap toggles the advanced UI. Settings now live in
`%LOCALAPPDATA%\Aircast\Aircast QGC.ini` (they were written to the launch folder, e.g. `C:\Windows\System32\.config`) and the map tile cache in
`%LOCALAPPDATA%\Aircast\QGCMapCache` (offline maps had no cache before).

Added on `winvm` (2026-10-20): Video sources (`Pages/Settings/CamerasPanel`) is Android's CamerasEditor: live status dots, On screen, a
grouped type chooser, fixed presets added directly, Name/Address/Tilt with the core's refusals inline, Remove with a 6 s Undo that restores
the position through `cameras.move`. Add, edit, remove, undo and the active source were read back through `view.cameras` and
`settings.videoSettings.cameras`. Not checked: the narrow (<840 px) layout of this page; Android's 2 s settle guard after a write is not
ported (the list is re-read after each change); there is no reorder control (as on Android). Fly view has Android's on-screen RC controls
editor (`Pages/Settings/RcControlsEditor`): channel range and clash refusals, the four control types written to
`settings.flyViewSettings.rcControls`, Remove with Undo, rename keeping index and type. Maps > Add new set opens a pushed map picker
(`Pages/Settings/OfflineSetEditor`): a plan-mode `MapView` in the chosen map type, a rectangle with corner and middle handles, Fit area to
view, zoom previews and the live estimate; it starts on the vehicle, then this computer's position, then the last Fly map view (kept in
memory only). About's Open waits for `Launcher.LaunchUriAsync` and shows a red caption if Windows refuses; one link opened the default
browser. Disabled Accent buttons (`PreflightLook`) now use Fluent's AccentFillColorDisabled and TextOnAccentFillColorDisabled colours.

Add link and the power calculator need the core fixes built into `C:\aircast-core\groundstation.dll` (serial ports and baud rates on
Windows, `links` fields kept when the native host answers null, mock battery ids 0/1, `page`/`screen` per setup component). They are not
yet landed in `groundstation`.

## Parity backlog

Audit against the Android head (2026-10-20): the Windows head covers the 54 Penpot boards but little of what Android's Fly screen adds on
top of them. Effort: S under a day, M 1–3 days, L more. Items closed since the audit: the camera switcher and PiP tile, the Video sources
editor, video fit, the RC controls layer and editor, the offline map picker, pattern presets, the leg-split + and landing/radius/centre
handles, Follow vehicle, the `view.missionItems` poll moved off the UI thread, live parameter rows, and in core the Windows serial ports and
`linkTypeIds`, the mock battery ids and `page`/`screen` per setup component.

### P0: bugs and real-use blockers

- Enforced checklist never passes: Windows never writes `vehicle.checkListState`, so with Enforce checklist on the core refuses Arm
  forever; the automatic checklist popup is missing too (S).
- Host notices are never shown: core banners, critical vehicle errors and app dialogs (Reboot vehicle, Open Setup, Use radio) from
  `view.hostNotices` (M).
- No vehicle message banner or messages sheet on Fly; messages live only in Analyze (S).
- No local record button: `video.startRecording` works only through the debug API (S).
- USB/serial links: the core now lists COM ports on Windows, but no real device has been seen; Bluetooth needs a WinRT RFCOMM host (L).

### P1: core flying parity

- Fly prompts: voice alerts (Android's 92 clips through `MediaPlayer`, `SpeechSynthesis` fallback; M), automatic mission popups,
  mission-complete dialog, close guard (`view.closeChecks`), connecting card, traffic banner and list (S each).
- Indicators: clickable battery/GPS/RC/telemetry/link details (M); flight-time, RC-signal, RC-override and RSSI cells (S); vehicle
  status sheet with hold-to-arm and sensor health (M).
- Multi-vehicle: fleet card, active-vehicle picker, every vehicle on the map, handover notice (L); mission progress card (S); tap a Fly
  pin to set the current waypoint (S).
- Video: WHEP in the shipped runtime (rerun `build-video.ps1`, then check against a SFU); gimbal indicator and control sheet (M).
- Settings and Plan: pilot settings rows built from parameters (S–M); waypoint camera section from `view.itemCamera` (S–M).
- Aircast device (aircastd): device home and the 11 device pages over `view.device` (L).

### P2: advanced and secondary

- Platform: ground-station position from `Geolocator` (unlocks the operator dot, Remote ID GCS location, Follow Me, Centre > My
  location; M), Aircast Cloud link and sign-in (M), GCS battery cell and keep-display-awake (S each).
- Fly overlays: ESC, Remote ID, GPS resilience, RTK, Aircast link and joystick cells; VTOL cell; gripper bays; orbit readout; proximity
  radar; obstacle arc and map layer; gimbal wedge and camera beam; gimbal on-screen aim; virtual joystick; operator-control prompt;
  terrain-data warning; offline status sheet; resume-failed prompt; disarm summary toast; first-run dialog (S–M each).
- Video: grid lines, aspect ratio, full-screen and zoom gestures; camera details (tracking, thermal, photo-lapse, format, stream); video
  stats pill; AR overlay; detection boxes from aircastd; synthetic view (needs a Windows attach in the core's wgpu renderer; L).
- Settings: app font size and the light palette (M); SD-card flasher (needs a Windows raw-disk backend in core; L).
- Plan: survey camera-calc block (brand/model, overlap, distance mode; S); the remaining ~240 ms core rebuild of `view.missionItems` on
  the UI thread after each edit on large surveys.

### P3: polish or low value on desktop

- 3D viewer as a Fly view (L), overlay layout editing, telemetry display editor (M each).
- Heading tape, guided-value presets, mission identity confirm, ROI tap sheet, lens measure, USB-radio indicator, support-forwarding
  cell, `aircast://` deep-link prompt, Plan double-click zoom, sensor-check and setup rows in the Settings tabs (S each).

Not applicable on Windows: the phone Wi-Fi SSID, RadioMaster AX12 built-in sticks, portrait Fly, the notification permission, the
Android foreground service and the phone camera as a video source.
