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

- ◐ plan (map tap-to-add and pin/handle drags not driven: no pointer input on the shared VM) · ✓ plan — replace plan · ✓ plan — upload warning · ✓ plan — terrain profile
- ✓ plan — fence & rally · ✓ plan — item editor · ✓ plan — command picker

One root page (`Pages/Plan/PlanPage`) on a second `MapView` in plan mode. Every `MapView` now owns its BridgeWatch ids
(`mapview.<guid>.<path>`) and stops them on Unloaded, so the Plan map cannot take over or stop the Fly map's watches. Driven on `winvm`
with no vehicle, the mock PX4 and the mock ArduCopter (2026-10-20) through UI Automation and the debug API: add (rail, centre, debug
`mission.insert`), select, edit altitude, change command, edit position, delete and undo items; survey template, size to view and hover
capture; fence polygon, circle, breach point, corner edit, remove vertex and undo; rally add, edit and remove; terrain profile and its
toggle; defaults and transform; save, open, export KML and import boundary through the WinAppSDK 1.8 pickers (unpackaged); the vehicle
connected and disconnected prompts; replace, clear and upload-warning dialogs (firmware mismatch, far from aircraft); upload to the green
Fly pill and on to Fly, which still draws the vehicle, the uploaded route, pins and live ADS-B traffic. Not driven: map tap, pin and handle
drags, terrain-chart marker taps, the Delete/Escape/Ctrl+Z/Y/S/O shortcuts. Not ported: the leg-split '+' that inserts mid-leg, drag
handles for landing approach/touchdown, fence circle radius and pattern centre, pattern presets, Centre > Follow vehicle / My location;
in plan mode a double-click adds a waypoint instead of zooming. The page polls `view.missionItems` every 700 ms (as Android); a
~3700-item hover-capture survey makes that ~140 KB per poll and the plan menu then took over 1 s to open.

### Vehicle setup

- ✓ vehicle-setup · ◐ setup — sensors calibration (compass, CompassMot and ESC calibration not run) · ◐ setup — radio (calibration started and cancelled, not run to the end; Spektrum/CRSF bind not run) · ✓ setup — flight modes
- ◐ setup — power (the voltage/amps calculator never gets a reading on the mock: the core's powercalc looks for battery id 0, the mock reports 1 and 2) · ✓ setup — safety · ✓ setup — motors · ✓ setup — frame
- ✓ setup — parameters · ✓ setup — parameter editor · ✓ setup — flight behavior
- ✓ setup — remote support · ▢ setup — camera (the mocks expose no camera setup page) · ▢ setup — lights (the mocks expose no lights component) · ✓ setup — tuning

One root page (`Pages/Setup/SetupPage`, two-pane only) over `view.setup`, reusing the `Forms/` engine (which now raises `Rejected` so
the parameter editor can offer "Force save (dangerous!)" after a refused write, as Android does). Airframe pictures ship as
`Assets/Airframe/*.svg`. Driven on `winvm` with no vehicle, the mock ArduCopter and the mock PX4 (2026-10-20) through UI Automation and
the debug API: the no-vehicle state and "Set up connection" to Settings > Links; the summary with each component's line; parameters by
group, search, the "Changed" filter (PX4), edit and read back, range refusal then force save, manual entry for an option list, Tools
(Refresh, Reset all confirmation cancelled); APM accelerometer calibration run to "YOU MUST REBOOT"; PX4 gyro calibration started, the
nav pane, Back and the setup list refused with "Complete or cancel the current calibration first" until the link dropped (the PX4 mock
never answers a cancel); radio Mode 1/2 and the calibration start/cancel; flight mode, battery capacity, RTL altitude, frame class/type,
servo range, PX4 return altitude and responsiveness edits, each read back and restored; motor test gates (props-off switch, refused
while armed); PX4 Airframe Apply confirmation (cancelled); APM basic/advanced tuning with restore from clipboard, PX4 PID tuning. Not
checked: PX4 Actuators (no ActuatorComponent on the mock), ESC/compass calibration, parameter file load/save, RC-to-param, joystick with
a real gamepad, Sub/Rover-only pages. The parameter list does not live-update from vehicle pushes (it refreshes after an edit and on
Tools > Refresh). Form and tuning sliders write only on pointer release or key up, so a UI Automation `RangeValue.SetValue` moves a slider
without writing. PX4 Airframe and PID tuning reach their pages through a two-entry name table in `SetupModel` (`Airframe` → Frame,
`PID Tuning` → Tuning); the core should name the page per component in `view.setup`.

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

- ✓ settings · ✓ settings — general · ◐ settings — fly view (the on-screen RC controls editor is not built) · ✓ settings — plan view
- ◐ settings — video (the Video sources camera-list editor is not built and is hidden) · ◐ settings — maps (a new set's area is typed as centre + width; no map picker) · ✓ settings — connections
- ◐ settings — add link (only UDP, USB radio and TCP are offered and no USB port is ever listed: the Windows core sends an empty `linkTypeIds` and lists serial ports only on macOS, so Bluetooth, log replay and the simulated vehicle never show)
- ✓ settings — comm links · ◐ settings — mavlink (signing Disable not seen: the mock never completes signing) · ✓ settings — flight modes · ✓ settings — adsb server
- ◐ settings — packet radio (no Wi-Fi adapter on the VM: only "Unavailable" and the Automatic adapter seen) · ◐ settings — remote id (the GCS position block never shows: Windows has no ground-station position source) · ✓ settings — rtk gps
- ◐ settings — firmware upgrade (Analyze's FirmwareTool; flashing not checked, no USB board on the VM) · ✓ settings — 3d viewer · ◐ settings — about (help links' Open not clicked)

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
