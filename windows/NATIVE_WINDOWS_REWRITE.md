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
- [x] Third merged build (2026-10-20, round 5: Aircast device, Aircast Cloud and deep links, video overlays, Fly controls, settings depth) smoke-tested on `winvm`: every page opens with no vehicle, the mock ArduCopter and the mock PX4; Settings > Aircast device against the aircastd fake-device harness (home, checks, link and health charts, all 10 page rows, the Telemetry page, Disconnect); Add link offers Aircast Cloud with its inline errors; all 17 APM and 10 PX4 setup components open from the core's `page` fields; the map layers flyout writes the obstacle-distance toggle; grid lines on a UDP H.264 stream; full-screen video from the rail button and with real F11/Escape keys; real mouse drags on the on-screen sticks climb the mock ArduCopter 11 → 18.6 m in 3 s and fly it 12.5 m/s north; Aircraft safety writes `RTL_ALT_M` (15 → 25 → 15) and shows PX4's rows; the light palette switches the running app on Fly, Settings and Vehicle Setup
- [x] Fourth merged build (2026-10-21, round 8: Fly polish and the off-thread Plan refresh) smoke-tested on `winvm`: Fly with no vehicle and the mock ArduCopter on the ground and flying at 1440×900 and 1024×700 (rail, thumb swap, OSD readings, map corner); Plan on a ~3,900-item hover-capture survey (grid-angle edits, insert-then-select, delete, undo/redo, drag)

## Screens (Penpot "06 Screens", 54 boards @ 1440×900)

Status: ▢ todo · ◐ in progress · ✓ done

### Fly

- ✓ fly · ◐ fly — flight mode (refused-change notice untested: the mock accepts every mode) · ◐ fly — camera (frames ✓, picture-in-picture ✓, video fit ✓; WHEP not seen in the shipped build: `build-whep.ps1` put `gstwebrtchttp.dll` in `C:\gstreamer`, but the stage's bundled runtime predates it until `build-video.ps1` is rerun) · ✓ fly — map context menu
- ✓ fly — armed, confirm return · ✓ fly — checklist · ✓ fly — no vehicle
- ✓ fly — host notices, message banner and sheet, voice alerts, disarm toast · ✓ fly — status strip and detail sheets · ✓ fly — multi-vehicle, operator position, Follow Me · ✓ fly — prompts (enforced checklist, mission popups, mission complete, close guard, connecting card, mission progress, set waypoint, resume failed, first run) · ✓ fly — field ops (KML/KMZ open, video press-and-hold and right-click menu, orbit radius, look-at reticle, route recording; round 14) · ✓ fly — payload (gimbal pill and sheet, map wedge, beam and radar, record shutter, gripper bays, camera details); details and gaps in the parity backlog below
- ✓ fly — on-screen sticks · ✓ fly — heading tape, guided presets and quick picks, mission identity card, ROI pin sheet · ◐ fly — operator control and VTOL cell (synthetic views only) · ✓ fly — video grid lines, full screen (rail, F11, Escape, double-click), aspect ratio, detection boxes · ◐ fly — obstacle arc, readout and map overlay, AR marks (constructed payloads only) · ✓ fly — Aircast device status chip
- ✓ fly — title bar fit and All status sheet · ◐ fly — overlay layout editing (widget drags driven in-process only) · ✓ fly — short-window layout · ◐ fly — synthetic view (drag aim not driven) · ◐ fly — 3D view (core reports the vehicle at 0 m facing north and no route segments)

Title bar (`Controls/StatusRow`, round 6): the status cells that fit sit next to the fixed link, picker, mode and armed pills; the rest go
behind an All status chevron tinted by the worst hidden cell, and its Status sheet lists every cell in the user's order (Android's compact
strip and all-status sheet, width-driven). Slot: 1200×800 and 1024×700 at 100/125/150 % font, mock ArduCopter with two vehicles. Merged
build (1200×800, 100 %): no title-bar button crosses the caption buttons with no vehicle (Connections only), the mock ArduCopter with two
vehicles (cells end at 2379 px, caption at 2390 px physical) or the mock PX4; the sheet lists Battery, Flight time, GPS, RC signal,
Telemetry RSSI and ESC status.

Overlay layout editing (`Controls/OverlayLayout`, Android's SharedPreferences layout as `%LOCALAPPDATA%\AircastQGC\fly-overlay-layout.json`):
Edit layout from the Status sheet or a widget's context menu; up/down and Hide/Show for the 16 title-bar indicators; hide badge and drag for
the message banner, fleet card, mission progress, obstacle readout and terrain warning; Reset layout (tap twice); Done or Escape; locked while
armed. Merged build: the sheet's Edit layout showed the move/hide rows and the Fly edit bar (Reset layout, Done editing layout), and Done
ended editing. Drags went only through an in-process hook in the slot; the right-click and press-and-hold entries were not driven. Not
ported: Android's value-size toggle, and the cameraSwitch and mapLayers widgets. Short windows: the fleet card and RC controls share a
scrolling left column beside the action rail, and the checklist, edit bar, right column and map corner stay clear of the PiP, the OSD and the
telemetry card (slot, 1024×700 at 100 and 150 %).

Synthetic view (`Controls/SyntheticView`, `Core/SyntheticNative`): the core's wgpu renderer draws on DX12 into a SwapChainPanel bound to a
DirectComposition surface handle (`qgc_synthetic_attach`/`qgc_synthetic_release`), with XAML overlays on top. It stands in for missing video in
the tile, full size and the picture-in-picture tile, aims with the tilt slider and recentre (`syntheticView.aim`) or follows `gimbal.aim` when a
gimbal steers it, and releases its surface when Fly is hidden or the window is minimized; "Synthetic View" is a camera source on Windows.
Slot: WARP adapter, mock ArduCopter, PX4 and gimbal, PiP as a second surface. Merged build: with the WHEP source unreachable, the tile and
the swapped full-size view drew 3D buildings over imagery ("Connecting to WHEP · showing synthetic view", `present=mailbox`) on the mock
ArduCopter, and the tile drew on the mock PX4. Not driven: drag aim (no pointer input then). Full screen also works over the synthetic view since round 9,
and its double-tap recentre gave way to double-click full screen, as on Android. An occluded window drops the view to about 1 fps.

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
Video sources. The Picture in picture item in the camera switch menu writes `multiViewEnabled`. The 160×90 PiP tile (`Controls/PipTile`, channel 1) reports
`video.setPipShown`, and its frames moved alongside the main channel. Pressing the tile makes that camera the main one
(`video.setActiveVideoSource`), and the core moves the old main into the PiP. Turning the toggle off clears `pipPipeline`.
`settings.videoSettings.videoFit` (Fit Width, Fit Height, Fill, No Crop) is applied to the main view as Android `videoContentSize` does,
checked with 1280×360 and 720×720 streams; the PiP always draws No Crop, like Android's second pane. Re-checked in the merged build.

On-screen RC controls (`Controls/RcControlsLayer`, on Fly while connected) from `view.rcControls`, `view.flyState` and the Fly view camera
channels: slider, button, three-way switch and momentary controls plus Tilt/Pan/Zoom sliders, Record, Light, Recenter and Give back. It
sends `vehicle.setRcChannelOverride` at most every 100 ms with a final send. Against the mock ArduCopter a UI Automation slider set gave
pwm 1800 held in `view.rcControls`, `vehicle.rcChannelOverrides` `{6:1800}` and `rcOverride` true; Give back emptied the overrides. Not
driven: holding a Momentary control and pointer slider drags. Overrides are released when you leave Fly as well as on unload
(`Controls/Presence.OnHidden` watches the ancestors' Visibility), as Android releases them when the layer leaves composition; verified
`{6:1800}` → `{}` on switching to Settings. Android's gimbal-manager controls in RcCameraControls are not ported (the gimbal pill and sheet
now cover acquire control, tilt and centre).

On-screen sticks (`Controls/VirtualJoystick`) follow `view.virtualJoystick`: two thumbpads, left-handed mode, the throttle parked at idle on
the ground, auto-centre. `StickSender` sends `vehicle.virtualTabletJoystickValue` every `periodMs` or on a move from a background loop, with
the touch age, while enabled and sending; leaving Fly lets the sticks go. In the merged build, real mouse drags on `winvm` held the left stick
up for 3 s in Loiter (mock ArduCopter 11.1 → 18.6 m, 22.8 m after release) and the right stick forward (12.5 m/s, latitude rising); the core's
sticks log read 204–209 sends per 9 s, none refused. The mock PX4 shows the pads with the throttle parked.

Heading tape (`view.attitude`) under the status row. The guided confirm card has takeoff presets and change-altitude quick picks, a mission
identity card on Start/Continue Mission and a FROM altitude tile on Land. Tapping the ROI pin opens Edit ROI position / Cancel ROI (the hit
test and card were driven in-process, not a real map tap). The title bar has a VTOL state cell and an operator-control pill and flyout, and
Fly has the control-request prompt; the mocks send no CONTROL_STATUS or VTOL state, so these were checked with synthetic views only.

Video overlays (`Controls/VideoMarks`, `ObstacleArc`, `ObstacleReadout`, `ObstacleMapLayer`, `LensSection`): grid lines; full-screen video
from the right column's Full screen button or F11 (AppWindow FullScreen presenter, title bar and nav hidden; Escape, the Exit button, the video stopping or leaving
Fly exit it), checked in the merged build with real keys; detection boxes from the aircastd SSE feed (checked against an SSE stub, with the
stale and refused states); AR marks and obstacle rings on the video; the map layers flyout with the obstacle-distance toggle and the map
obstacle overlay; the obstacle arc and readout; lens measure and forget in the camera panel. Obstacle and AR drawing were checked only from
constructed `view.obstacle`/`view.videoAr` payloads (the mock has no OBSTACLE_DISTANCE; the core's `synthview.rs overlays()` returns no home or
waypoints with coreplan). Mouse-wheel zoom calls `stepZoom`, but the mock camera's zoom never changes, so no effect was seen; touch pinch was
not driven. `SwapVideo` now docks the payload and marks layers with the video tile, so the payload layer takes input when the video is main.
`VideoDriver` also plays the core's preview channels 2..5 (`video.previews`) for device camera previews; no frames seen yet (needs a WHEP-serving
device).

### Fly layout

Round 7 (2026-10-20) gives Fly the Android/web DJI layout (`android/.../ui/FlyShell.kt`, `web/src/ui/FlyShell.tsx`) while keeping
Windows materials: Mica window, acrylic circles, chips and flyouts (`ChipBrush`), Segoe Fluent Icons, WinUI MenuFlyout and ContentDialog,
system caption buttons, keyboard focus and a tooltip on every icon-only control. Every colour is a token in `Theme/Tokens.xaml` (dark and
light). Only layout and look changed; the core calls are the same.

| Region | Where it is now | Was |
| --- | --- | --- |
| Navigation | ☰ MenuFlyout in the title bar (Fly, Plan, Analyze, Vehicle Setup, Settings with icons) | permanent nav pane |
| Status | transparent title bar over the view: vehicle, mode, armed, compact cells, All status sheet for the overflow | pill chips on Mica |
| Actions | `GuidedBar` is a vertical rail of 48 px icon buttons in `LeftRail`, centred between the status row and the PiP: Checklist (on the ground only, as on Android), then Takeoff (or Arm) on the ground, or Pause, Return, Land and Payload in flight; ⋮ holds the rest; Emergency stop sits 24 px below in red acrylic (`StopBrush`), hold 1.5 s to cut or click for the slide card | labelled toolbar top-centre |
| Confirm card | opens beside the rail | under the toolbar |
| PiP | bottom-left, 320×180 (web `DESKTOP_PIP`), 2 px frame, map clipped round when the video is main, `PipFrame` swaps back; tapping the main video (or a still tap on the gimbal-drag layer) does nothing, only the thumb swaps; the second-camera `PipTile` stacks above it | top-right tile |
| Telemetry | `InstrumentBar` as OSD text beside the PiP (small H.S/V.S row, big H/D row, white with a composition shadow, Android's sizes 12/14/22 via `FontSizeOsdValue`, no space before the unit); default readings are Android's D, H, H.S, V.S plus A.S for fixed wing, VTOL and airship until the user picks their own; compass/attitude card off by default, shown from the chooser | boxed bar and compass card |
| Camera | right edge, centred: `CameraSwitch` chip (cameras and Picture in picture), tilt readout and slider, shutter, Camera, Full screen | tile buttons and a separate shutter |
| Map controls | `MapCorner` bottom-right on the map only: zoom, centre on vehicle, 48 px layers button, scale bar under them; the scale bar hides while the map is the PiP | right column |
| No video | `NoVideoPill` in `CenterMessage`, above the No vehicle card when both show | text inside the tile |

Merged build on `winvm` (1440×900, 1200×800, 1024×700, scratch variant with the mock ArduCopter): no vehicle, on the ground and flying, map
main and video main, light palette once. Every page opened from ☰; the rail's Takeoff, Arm, Force Arm, Pause, Return, Land, Change
Altitude, Change Max Ground Speed and Emergency stop cards opened and cancelled, Payload opened the gripper panel, and hold-to-cut disarmed
the flying mock; the All status sheet, the Telemetry display editor, the camera switch menu and the map layers menu opened. Side by side
with Android: `~/.claude/jobs/6037fcd4/tmp/design/round7-compare/`.

### Fly polish (round 8)

The `HeadingTape` has no box over the main video (white ticks and numbers under the OSD shadow) and keeps Android's 35% black box
(`OsdBackdropBrush`) over the main map. The OSD, scale bar, main-video tap and checklist changes are in the table above. Merged build on
`winvm` (2026-10-21, scratch variant with the mock ArduCopter, 1440×900 and 1024×700): with no vehicle, then on the ground, then flying
at 10 m/s after a debug-API takeoff and go-to. The automation tree lists exactly Ground Speed, Climb Rate, Distance to Home and
Alt (Rel). The checklist shows on the ground and is gone in flight. Invoking the main video leaves it main, and Show the map swaps
back. Side by side with Android: `~/.claude/jobs/6037fcd4/tmp/design/round8-compare/android-vs-windows.png`.

Still different from Android: the status strip and rail are acrylic chips and circles, not bare text and icons (the Windows material
choice). There are no on-screen sticks over the video (only on touch). The PiP is web desktop's 320×180, not the phone's small square.
The camera switch chip, tilt readout and gimbal controls need a camera, and the mock has none. D.OP is missing (it needs a
ground-station position). A WHEP pill sits in the centre while Android names the source in its
camera chip. The synthetic view draws grey extruded buildings, not photoreal tiles.

### Fly 3D view (round 9)

`Controls/Viewer3DView` is Fly's 3D main view, like Android's MapLibre viewer: OSM buildings from `view.viewer3d` over the core's map
tiles, the quad frame from `view.viewer3dVehicle`, mission markers and labels from `view.viewer3dPath`, and the flown track from
`view.track` draped on the ground. A cube button under Map layers (`MapControls.ThreeD`, tooltip "Show the 3D view") appears when the
3D viewer setting is on and turns into Show the map while 3D shows. Zoom in/out and Center on vehicle drive the 3D camera while it
shows, the video thumb still swaps, and turning the setting off falls back to the 2D map. The camera starts like Android (zoom 16,
pitch 60) and follows the vehicle. Left-drag orbits and tilts, the wheel zooms, right-drag or Ctrl/Shift-drag pans and stops
following, Home or Center on vehicle resumes, and arrows and +/- work. A tap on a mission marker turns it yellow. WinUI has no 3D
control and no new packages were allowed, so `Viewer3DRaster` is a CPU rasteriser with a 1/z depth buffer into a WriteableBitmap, and
`Viewer3DScene` holds the geometry. Twelve `Viewer3D*Brush` tokens are in both palettes.

Merged build (2026-10-21, scratch variant, mock ArduCopter flying a five-item mission over the core's Zurich test OSM file, 1440×900
and 1024×700): buildings over Bing tiles, labels 2/3/R/5 and the flown track; orbit, zoom and follow; swap to video and back to 3D;
Show the map back to 2D. Frame cost at 2427×1518 px: median 2.8 ms, p95 3.4 ms on a quiet VM (88 buildings, 768 faces). The slot
measured median 4.5–4.8 ms with a 4,370-building extract.
Screenshots: `~/.claude/jobs/6037fcd4/tmp/design/round9-merge/shot-fly1-3d-flying.jpg`, `shot-fly1-1024-3d.jpg`.

Still open: real mouse, wheel and keys were not driven: the camera was moved through the same methods in-process. Switching OSM files
builds the faces on the UI thread (about 100 ms for 4,370 buildings). Round 10 closed the altitude, heading, route and saved-view gaps.

### Fly 3D view fixes (round 10)

Core `viewer3d.rs`: `view.viewer3dVehicle` reads `altitudeRelative` and `heading` from the facade's `facts` array (raw metres first).
Before, it looked for top-level fields and always drew the drone at 0 m facing north. A takeoff with no coordinate now stands on the
vehicle home (else the plan's launch point), so an uploaded mission gets the green T marker and its route starts there, as in upstream
QGC. The JSON shape is unchanged, so Android and web get both fixes with no edits. Windows keeps the 2D/3D choice across restarts in
`%LOCALAPPDATA%\AircastQGC\fly-view.json` (`Controls/FlyViewStore`), as Android keeps its fly view.

Merged build (2026-10-21, scratch variant, mock ArduCopter, the core's Zurich OSM extract, a takeoff and two waypoints sent with
`plan.sendToVehicle`): the downloaded takeoff is cmd 22, altitude-only, with no coordinate. `view.viewer3dPath` gives T (green, over
home at 50 m), 2 and 3 with orange segments T→2→3. In Auto at 50 m `view.viewer3dVehicle` read `[8.5463516, 47.3979914, 50.0]`
heading 64, matching the vehicle's facts, and the drone sits on the route ribbon. Show the 3D view stored "ThreeD"; after a restart
the 3D view came back; turning the setting off fell back to the map and stored "Map". The 3D setting, OSM path and `fly-view.json`
were restored afterwards.
Screenshots: `~/.claude/jobs/6037fcd4/tmp/design/round10-merge/shot-m10-3d-leg1.jpg`, `shot-m10-3d-restarted.jpg`.

Still open: Windows keeps only 2D or 3D, not whether video is the main view (Android keeps Video, Map or ThreeD as one value). The
debug API's `/mission/upload?file=` fails on the native host with `Unknown plan "all"`. Heading was judged from the readings and the
default camera angle, not by orbiting.

### Video gestures and aspect ratio (round 9)

Double-click or double-tap on the main video enters or leaves full screen through the same path as the rail button, F11 and Escape
(`VideoMarks.FullScreenToggled`). It toggles only when both presses landed on the main picture and not on a focusable control. A single
click still does nothing, the thumb only swaps, and the exit button and aim pill keep their own clicks. Full screen and its rail button
follow `VideoThumbnail.Picture` (decoded video or the synthetic view), Android's `videoDecoding || syntheticOnScreen`. The picture and
its overlays use Android's aspect rule: the decoded size wins, then `aspectRatio` if above 0, then fill the box. Settings > Video >
Advanced shows Aspect ratio as W:H and accepts W:H or a number.

Merged build (2026-10-21, mock ArduCopter, synthetic view as the picture): double-click full screen both ways at 1440×900 and
1024×700, also after swapping from the 3D view; a double-click on the thumb only swaps; the in-process aspect check gives 1589×894 for
16:9, 1192×894 for 4:3 and fill for 0 without a source size, and 1589×894 for every setting with a 640×360 source. The slot also ran it
over a live 640×360 WHEP stream.
Screenshots: `~/.claude/jobs/6037fcd4/tmp/design/round9-merge/shot-fly1-dbl-full.jpg`, `shot-fly1-1024-dbl-full.jpg`.

Still open: no real mouse or touch double-click was sent; the hook did WinUI's real hit test, then called the handlers. F11 and Escape
were not pressed. The WHEP stream did not connect in the merged variant (its runtime predates the WHEP plugins, see Fly), so live video
was not re-run after the merge. With a live stream the aspect setting does not change the frame, by design. Over the synthetic view,
full screen shows a black strip for 1–2 s while the renderer resizes.

### Fly UX review fixes (round 11)

A task-level review of the Fly view ("get the drone in the air, watch it, bring it back") ranked six findings. All six are fixed in
the head; finding 1 also needed a core change.

1. No camera, generated picture. A tag on the main picture (`Controls/PictureTag`, caution edge, acrylic chip) reads
   "No camera · 3D map view" once the 3D view has drawn, and "No camera" before that. Its second line gives the camera state in plain
   words ("Connecting to the camera…", "Camera: Reconnecting…", "Video off"). The tag is a button: its menu lists the cameras and the
   picture-in-picture toggle (`CameraMenu`, shared with the camera chip) and opens Settings > Video sources. The thumbnail badge says
   "No camera · 3D map view" instead of "WHEP · Connecting…". The 3D view hides its stale frame until it has painted at the new size,
   so the black area with an old corner frame at 1024 wide is gone. Core (`synthview.rs`, `synthetic/host.rs`): `Draw::awaiting` makes
   `view.syntheticView.draw` report `loading` from an attach or resize until the first frame at the new size is presented.
2. Status row. Cells are fitted by severity: battery and the core's critical and caution cells are pinned, healthy cells overflow
   first, the battery drops time and current when room is short, and the chevron names an abnormal cell it hides ("RC signal ›").
   On the ground one verdict replaces "Disarmed": "Ready to fly" or "Not ready · <reason>", taken only from `view.flyState` and
   `view.warnings`; it opens Vehicle status. Pinned cells get width before the verdict text.
3. Guided confirm. The card has a focused "Confirm <action>" button: hold 0.8 s (1.5 s for destructive asks) with a sweeping fill,
   or a fresh Enter or Space; Escape cancels; the slide stays. Destructive asks get a red button and focus starts on Cancel.
   Automatic mission popups do not take focus. Ctrl+R opens the Return card.
4. Rail labels. The rail owns Checklist, the actions, More and Stop, each with a short label (Take off, Start, Return, Land, Pause,
   Payload, More, Checklist, Stop motors). When the labelled rail does not fit, only the primary keeps its label. Return uses a U-turn
   arrow with an H instead of the house.
5. Emphasis. Take off is the filled primary on the ground and Return in flight; Stop motors is a red outline at the bottom after a gap.
6. No vehicle. The thumbnail is hidden while no vehicle is connected, the video is not the main view and no picture is live, so the
   connect card stands alone.

The rail starts below the picture tag (`MainWindow.RailTop`), so the two never overlap at short window heights.

Merged build (2026-10-21, scratch variant, mock ArduCopter, the VM's saved camera at an address with nothing behind it): no vehicle at
1440×900 and 1024×700 shows the connect card with no thumbnail. On the ground the verdict reads "Ready to fly" and the rail is
Checklist, Take off (filled), More. In flight the rail is Pause, Return (filled, Ctrl+R), Land, Payload, More and an outlined
Stop motors; the main 3D picture carries "No camera · 3D map view"; no protocol name appears in any text or button name. Right after a
resize to 1024 the view showed "Loading 3D view…" over a blank picture and the tag read "No camera", then the full picture. Return was
confirmed four ways in-process: a full hold, a fresh Enter (a repeated Enter did nothing), Enter and Escape posted as window messages
to the app's own window, and Ctrl+R posted the same way opened the card. A 0.37 hold cancelled. The Stop card opened with focus on
Cancel and Escape closed it with the motors running. Light palette checked once and restored. The tag menu opened Settings > Video
sources. Core: `cargo test --lib` 2675 passed, iOS and wasm32 checks clean, DLL built on the VM.
Screenshots: `~/.claude/jobs/6037fcd4/tmp/design/round11-ux/side-f1-video-label-1440.jpg` through `side-f6-no-vehicle.jpg`, and the
`merged-before-*` / `merged-after-*` pairs in the same folder.

Still open: the core has no "RC not required" signal, so the verdict can read "Ready to fly" next to a red "No signal RC" when the
vehicle itself reports ready (proposed: `flyState.rcRequired` from PX4 `COM_RC_IN_MODE` or ArduPilot `RC_OPTIONS` bit 0). The `draw`
state is shared by every 3D surface, so resizing the PiP's 3D view briefly marks the main one as loading. The camera title is user data
and can still be a protocol name (the VM's camera is called "WHEP"). Android shows the thumbnail whenever a source is configured;
Windows hides it with no vehicle unless the picture is live. Start mission sits on the ground rail; Android keeps it in More. No real
mouse, touch or key press was sent: holds and slides ran through the card's own handlers and keys were posted to the app's window.

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

The item editor has the waypoint camera section (`Pages/Plan/PlanCamera`, `view.itemCamera`): action, time or distance interval, camera mode,
gimbal pitch and yaw sliders and the notes, at the top for simple items and inside the editor for mission start. Surveys have the camera-calc
block: brand and model pickers, read-only sensor facts, overlaps, the altitude (distance) mode and Set by altitude or ground resolution.
Checked on the mock PX4 in the depth slot (2026-10-20). Not driven: typing into the Time/Distance/Pitch/Yaw boxes (sliders and combos were).

Added and driven on `winvm` (2026-10-20): a mid-leg + on the leg into the selected item inserts a waypoint at its midpoint (UIA Invoke;
re-checked in the merged build against the mock PX4). The landing pattern is drawn (approach path, loiter ring), with Approach/Touchdown
handles on the selected landing. A selected fence circle gets a Radius handle; fence polygons and survey/structure areas get a Move centre
handle. Each handle's drop handler was driven in-process and its write read back: `finalApproachCoordinate`/`landingCoordinate`,
`circles.N.radius`, clear+`appendVertices`. Pattern presets (save with blank and "/" refused, apply, delete with confirmation) go through
`core.plan.savePreset`/`applyPreset`/`deletePreset` and `view.patternPresets`. Centre > Follow vehicle follows the flying mock and any
re-centre turns it off. Delete, Escape, Ctrl+Z/Y/S/O were sent as window messages posted to the app's own input window, and are ignored
while a dialog is open. Not driven: map tap, real pointer drags, terrain-chart marker taps. Not ported: Centre > My location (Windows has
no ground-station position source). A double-click zooms in one level at the point, and a single tap adds its waypoint after the
system double-click time (500 ms), as on Android (round 9). Landing handles appear only while that
landing is selected (Android draws them always). The centre handle moves a shape by its vertex average, not Android's area centroid.
`view.missionItems` is still polled every 700 ms, as on Android, but off the UI thread; it skips parse and redraw when the payload is
unchanged, and the overlay re-projects only when the map view moves (`MapView.ViewChanged` now fires only on a real change). With a
~4,200-item hover-capture survey (140 KB), the worst idle UI stall fell from 51 to 28 ms and opening the plan menu from 242 to 94 ms.

### Plan refresh off the UI thread (round 8)

After an edit, `Changed()` returns at once. The fetch and parse of `view.missionItems` run on a thread-pool thread, one at a time,
and edits made meanwhile fold into one more fetch. Only `Apply` (rendering) runs on the UI thread. Insert-then-select, remove, drop,
open-plan and the item editor await the refresh before their next step. Map taps, split handles, rail buttons and keyboard shortcuts
wait for a pending refresh (`Settled`), so a quick second input still sees fresh `_items`. On a ~4,000-item survey the post-edit UI
block fell from 16–22 ms to 0 ms, and the worst stall per grid-angle edit fell from a median of 159 ms to 121 ms. Merged build
(2026-10-21, ~3,900 items, mock ArduCopter flying): three grid-angle edits applied (folded commands 3,919 → 4,063 → 4,053 → 3,919),
and two back-to-back taps after waypoint #2 landed at indexes 3 and 4 with #4 selected. A tap with an item selected deselects it,
and the next tap appends and selects the new item. Two undos removed the tapped waypoint and then both inserted ones; redo restored
both. A drag kept the item selected with the panel open, and delete left #3 selected. Round 9 moved the core write itself off the UI thread too (next section). A pin press is
not gated, because drags must stay live, so during a refresh of roughly 10–20 ms a press can hit a pin from the old scene.

### Plan writes on a background lane (round 9)

Every Plan write runs on one serial background lane (`PlanCalls.Queued`), awaited by its caller: insert, remove, undo and redo, select,
fence, rally, breach, vertex and circle edits, editor fields and route defaults, the survey camera block, presets, load, save, KML,
import, templates, download, upload, transform, map type and the undo flag. UI values (map centre, bounds, toggles) are read before a
job is queued. The post-edit refresh and the two reads that gate an action (upload and save status) go through the lane too. Drags lost
the 120 ms throttle: each handle has one write in flight and one pending position, and the drop writes the final one in lane order.
`Settled` waits until the lane, the refresh and the page's current edit are idle, so a fast second tap inserts in the right place.

Slot (4,033–4,105-item hover survey, 16 ms heartbeat): the `gridAngle` write left the UI thread (it took 103–187 ms there), and the
worst UI gap per edit fell from 108–199 ms (median about 168) to 40–126 ms (median about 65). What remains is the redraw of the survey.
40 vertex moves in 1.5 s became 12 lane writes, and the vertex ended at the drop point. Save and open through the real pickers worked.

Merged build (2026-10-21, scratch variant, mock ArduCopter connected, 4,033-item survey): six Angle edits applied with worst UI gaps
of 74–121 ms (median about 105). The rail Waypoint after selected #2 inserted at 3 and selected it. Two back-to-back taps kept their
order in 3 of 3 rounds. Undo (hook, then toolbar) and Redo applied in order. A waypoint drag through OnDragged/OnDropped ended at the
drop point with "Moved #2". Delete kept the next item selected. 40 survey-vertex moves in 1.8 s moved the corner.
Screenshots: `~/.claude/jobs/6037fcd4/tmp/design/round9-merge/shot-plan1-edits.jpg`, `shot-plan1-dragv.jpg`.

Still open: every bridge call first takes the core's plan mutex (`poll_host` → `held()`), so a UI-thread read during a lane write can
wait 13–44 ms. Not driven on the lane: the fence-circle, rally, landing and takeoff drag handles, Upload, the vehicle-change and
altitude prompts, Transform, KML/SHP import and templates. Real pointer drags were not driven.

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
core, so their sliders watch `FormControl.LivePath`, the underlying parameter. `SetupModel` reads each component's `page` and `screen` from
`view.setup` (PX4 Airframe → Frame/px4Airframe, PID Tuning → Tuning/px4Tuning); its own name table is gone. In the round-5 merged build all
17 APM and 10 PX4 mock components opened from their rows. The "Aircraft setup" link (vehicle status sheet) now lands on an overview on wide
layouts, the list plus "Aircraft setup · Choose a component on the left" naming components that need attention, instead of opening the first
component (slot, 1200×800 and 800×700); the nav pane's Vehicle Setup still opens the first component.

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
- ◐ settings — add link (Wi-Fi or network, USB telemetry radio, Bluetooth radio, Aircast Cloud, Network server (TCP), Replay a flight log and Simulated vehicle are offered and the 19 Windows baud rates listed; no COM port seen: the VM has none; Bluetooth not seen with a real adapter)
- ✓ settings — aircast device (aircastd) · ✓ settings — aircraft safety and aircraft limits (pilot rows) · ◐ settings — appearance (light palette live; font size live for XAML-built UI only)
- ✓ settings — comm links · ◐ settings — mavlink (signing Disable not seen: the mock never completes signing) · ✓ settings — flight modes · ✓ settings — adsb server
- ◐ settings — packet radio (no Wi-Fi adapter on the VM: only "Unavailable" and the Automatic adapter seen) · ✓ settings — remote id (GCS position block seen with an NMEA fix; the VM's own Windows fix is too coarse) · ✓ settings — rtk gps
- ◐ settings — firmware upgrade (Analyze's FirmwareTool; flashing not checked, no USB board on the VM) · ✓ settings — 3d viewer · ✓ settings — about
- ◐ settings — aircast card (SD-card flasher, see SD-card flasher (round 10); a virtual disk was written, no real reader or UAC prompt seen)

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

Aircast device (`Pages/Device`, Settings > Device > Aircast device): connect by address with the core's refusals, the device home (headline,
Before you fly checks, link and health charts, the page list, This app with Disconnect) and the 11 native device pages over one keyed
reconcile kit (`DeviceTree`), so live SSE pushes keep typed text and focus; the Wi-Fi and camera wizards; the Fly status chip
(`Controls/DeviceStatusChip`) when the device is not ready. Checked on `winvm` against the aircastd fake-device harness relayed onto
192.168.105.1 (device slot: every page, writes read back on the harness, QR, flight lanes and zoom, both wizards, setup steps; merged build:
home, all 10 page rows and the Telemetry page). Not seen: camera preview frames, file-picker uploads, pointer-only lane gestures, several
harness-unreachable states (wizard steps 2–3, Cloudflare and account stages, Tailscale sign-in).

Add link offers Aircast Cloud (`Pages/Settings/AircastCloud`): device-code sign-in with the browser opened by `Windows.System.Launcher`, the
drone picker, sign out and cancel, then `links.createAircastCloudLink`; checked against a stub account server, not a real account. The title
bar has the USB-radio cell (only the stopped state seen, with a forced port) and the support-forwarding cell. `aircast-qgc://` links are
registered per user under HKCU; a second launch carrying a link hands it to the running instance and exits, and the "Set up from this
device?" prompt follows (warm and cold launches checked; accepting reaches the core only, no device to fetch from).

Aircraft safety and Aircraft limits (`Pages/Settings/PilotSettings`) are Android's Safety and Control tab homes: return-home altitude, max
altitude and distance, the signal/RC/battery failsafes and max speeds built from the firmware's parameters, with offline and loading states,
the sensor checks from `view.calibration`, Calibrate sensors and More failsafe settings into Vehicle Setup, and search hits. Checked on both
mocks; the merged build wrote `RTL_ALT_M` 15 → 25 → 15 through UI Automation. Appearance: `Theme/Appearance` applies
`indoorPalette` (dark, light, follow Windows) and `appFontPointSize` live; Light values live in `Theme/Tokens.xaml`. Code-built panels pick up
a new font size on their next render; the title bar fits its cells at 125 and 150 % (round-6 slot, after a restart at each size). Fly readings open a telemetry display editor
(`Controls/InstrumentDisplay`: icon or text label, units, colour/opacity/icon ranges, stored per vehicle class in
`%LOCALAPPDATA%\AircastQGC\instrument-displays.json`); its right-click menu entry was not driven.

Bluetooth radio (`Core/BluetoothHost`, `Core/BluetoothRadio`, round 6): a WinRT RFCOMM (SPP) host installed through `qgc_bluetooth_install`
lists paired devices, discovers others for 15 s with a DeviceWatcher and connects (pairing if needed, SPP service lookup, StreamSocket). The
picker says why scanning is impossible and no longer rebuilds its list every second. Slot: the no-radio path, and against an in-slot fake
radio scan, empty scan, pick, connect with the vehicle heard, disconnect, device drop, the no-SPP refusal and changing device in Edit link.
Merged build (no adapter on the VM): `view.links` has `bluetooth {available:true}` and `bluetooth` in `linkTypeIds`; Add link offers
"Bluetooth radio"; the picker shows "No devices found. Scan to look for radios nearby." and "This PC has no Bluetooth radio."; Scan keeps
that reason; Connect with no device says "Pick a Bluetooth device.". Not seen: a real adapter, discovery, pairing (the PIN prompt for an
unpackaged app is untested), RFCOMM throughput.

Settings search lists the On-screen RC controls editor under Editors > Fly view (merged build: "rc controls" finds it).

The core fixes the head relies on (Windows serial ports and baud rates, `links` fields kept when the native host answers null, mock battery
ids 0/1, `page`/`screen` per setup component, and round 6's fleet Return, Bluetooth host and synthetic view) are on fork/main (808c4bd521).
Round 10's core changes (`windisk.rs`, the `platformdisk` fallback, the `windows-sys` dependency and the `viewer3d.rs` fixes) are not on
fork/main yet. The stage runtime's `groundstation.dll` is built from fork/main 667f20a521 plus those files, and they must land on fork/main
before the head files that use them.

### SD-card flasher (round 10)

Settings > Aircast card (`Pages/Settings/FlasherPage`, General group, USB glyph) ports Android's Card → Setup → Write flow over
`view.flasher` and the unchanged `flasher.*` commands; `FlasherModel` holds Android's pure step helpers. It opens from a row at the top of
Settings > General and from "New drone? Flash an Aircast card" on Fly's no-vehicle card. Write asks first in a ContentDialog ("Erase and
write"), then shows a ProgressBar with percent, speed and time left, Cancel, and ends in Card ready, Cancelled or Failed. A DisplayRequest
keeps the screen on and navigation is locked while busy. Without administrator rights the card fails with "Writing a card needs
administrator rights" and an InfoBar offers Restart as administrator (`Core/Elevation`: ShellExecute `runas` with `--after-exit-of <pid>`;
`App.OnLaunched` waits for the old instance so the ports are free).

Core: `groundstation/src/windisk.rs` is the Windows raw-disk backend behind `platformdisk` (`windows-sys` 0.61.2 as a `cfg(windows)`
dependency, already in Cargo.lock). It lists `\\.\PhysicalDrive0..63` on the USB, SD and MMC buses, never the disk holding Windows or the
app, as "vendor product · size". It opens the card read/write with the admin and write-protect refusals, and locks and dismounts the card's
volumes only on the first write. `QGC_FLASHER_VIRTUAL_DISKS=1` also lists file-backed virtual disks; it is a test switch the app never sets.

Merged build (2026-10-21, scratch variant with the virtual-disk switch, a fresh 6 GB VHDX holding a mounted FAT volume): Fly's link
opened the page with only "Msft Virtual Disk · 6.4 GB" listed, not the NVMe system disk. After a safety check (File Backed Virtual,
6442450944 bytes, not boot or system, the VHDX path, card id `\\.\PhysicalDrive1`) it downloaded Aircast OS v0.3.5, then wrote, verified
and customised the card in 166 s (about 35 MB/s). Read back: MBR 55AA, a FAT32 `bootfs` with user-data (hostname falcon-11, pi
password), network-config (field-net, regulatory domain US) and meta-data, and ext4 magic on partition 2. Relaunched with a Basic User
token, the card showed the admin refusal with the InfoBar, its Restart dialog opened, and Cancel kept the app usable. The VHDX and the
image cache were deleted afterwards.
Screenshots: `~/.claude/jobs/6037fcd4/tmp/design/round10-merge/shot-m10-writing.jpg`, `shot-m10-done.jpg`, `shot-m10-needs-admin.jpg`.

Still open: real elevation (UAC is off on the VM, so `runas` relaunches with the same token); no real USB or SD reader; external USB
disks are listed too (no removable-media filter); closing the window mid-write gives no warning and leaves the card half-written; the
PC's Wi-Fi name prefill and the metered note are untested (no Wi-Fi on the VM); the light theme was not screenshotted.

## Parity backlog

Audit against the Android head (2026-10-20): the Windows head covers the 54 Penpot boards but little of what Android's Fly screen adds on
top of them. Effort: S under a day, M 1–3 days, L more. Items closed since the audit: the camera switcher and PiP tile, the Video sources
editor, video fit, the RC controls layer and editor, the offline map picker, pattern presets, the leg-split + and landing/radius/centre
handles, Follow vehicle, the `view.missionItems` poll moved off the UI thread, live parameter rows, and in core the Windows serial ports and
`linkTypeIds`, the mock battery ids and `page`/`screen` per setup component.

Status marks below (✓ done · ◐ partial · unmarked open) come from rounds 4 to 6 (2026-10-20). Round 4: notices, prompts, indicators, fleet,
payload. Round 5: Aircast device, Aircast Cloud and deep links, video overlays, Fly controls, settings depth. Round 6: title bar and layout
editing, synthetic view, Bluetooth links, polish (fleet Return, guided stepper, arming-check links). Each area was verified in its own
VM slot against the mock ArduCopter and PX4 (and stubs where named), then merged into this tree and smoke-tested together on `winvm` (no
vehicle, mock ArduCopter, mock PX4, two ArduCopter mocks, a UDP H.264 test stream, the aircastd fake-device harness). Pointer-only gestures
were mostly driven through in-process hooks or UI Automation; the round-5 merge smoke used a real mouse for the sticks and real keys for
full screen, and the round-6 merge smoke real mouse slides for arm, takeoff, change altitude, fleet Arm and fleet Return.

### Field call fixes (round 13)

From a call with a field user flying real drones on the Windows app (2026-10-21):

- UDP links: the core path was correct; Windows Firewall drops inbound UDP for an exe it was never asked about. A UDP link that
  is connected but hears nothing now says the firewall is blocking port N and opens the allowed-apps page. Adding the radio's
  address under "Also send to" works without admin (Windows lets the peer's replies in).
- The Windows head started the core with `--no-autoconnect`, so USB radio and Pixhawk autoconnect, "Connect on start" links
  and reconnects never ran; the flag is gone.
- Radio latency: on Windows the serial port opened synchronously, so a write waited behind the pending read (uplink mean 420 ms,
  p95 2 s on a quiet line). The core now opens COM ports overlapped (serial2), like QSerialPort: writes arrive in under 4 ms.
- Gimbal: the gimbal worked, but Tap to point ignored drags and the mode pill hid under the title bar. A drag now moves the
  camera, the pill sits below the status row, refused gimbal commands show for 4 s, and Android's tilt rail (Tilt chip plus a
  vertical pitch slider) is in the camera column.
- Video: one Video settings page (sources first). A double-click on the thumbnail swaps once and enters full screen.
- Icons: one `IconFontFamily` ("Segoe Fluent Icons,Segoe MDL2 Assets") so icons show on Windows 10; the exe and window carry the
  Aircast icon.
- Scrolling: hidden Fly controls stop working behind other pages and replay their state on return (heartbeat gap p95 28-39 ms to
  about 20 ms, gaps over 100 ms 9 to 0); the vehicle status sheet has one scroll viewer.
- Not driven with real input on the final build (a firewall dialog blocked the VM desktop); no real radio or Windows 10 PC.

### Field ops (round 14)

The field-ops features from the Qt app, on the core's KML import, route recorder, ArduPilot orbit and video projection:

- ✓ Plan > Open takes `.kml`/`.kmz`; the core's "KML imported" summary shows as a host dialog and its refusals (areas only, broken
  XML, nothing found) in the Plan status line.
- ✓ Press and hold the main video (mouse, touch or pen) or right-click it: a ring fills over 0.8 s, `videoAr.groundAt` places the
  spot and the map click menu opens there with only Go to location, Orbit at location, Look at this point and Stop looking, plus the
  core's field-of-view warning. It leaves full screen and closes an automatic Start/Continue Mission prompt. One-time hint
  (`%LOCALAPPDATA%\AircastQGC\video-hold-hint-seen`), "Press and hold on a spot below the horizon", and the no-gimbal message. The
  gimbal's Hold aim mode keeps press-and-hold for steering; right-click still opens the menu there.
- ✓ Orbit confirm: a 10–500 m radius stepper (10 m steps, 30 m default, from `orbitRadiusMetres`) with "Orbit the location at a
  50 m radius", direction and height; the map menu and the video menu share it. The pending circle (`videoAr.outline`) and the running
  orbit (`view.videoAr.orbit`) are drawn on decoded video.
- ✓ While looking at or orbiting a point: a reticle at the picture centre and "Looking at this point" / "Orbiting this point" with
  Stop (stop looking, or pause when orbiting). The map pin already reads "Looking here".
- ✓ Route recording: Route/Stop on the left rail, the "Recording route · 230 m" pill, the recorded path on the Fly map, and a
  "Route saved" notice that stays until dismissed with the editable name (a used name reverts and says "Name already used"),
  Open in Plan and Dismiss.
- Smoke on `winvm` against the mock ArduCopter with a gimbal and the local MediaMTX WHEP test stream (decoded): hold and
  right-click menus (with Stop looking and the field-of-view warning), the hold ring, the stepper 30 → 50 m with the pending circle
  on the video, a confirmed orbit with its circle, chip and Stop, Look at then Stop, the horizon hint, the recording pill and path on
  the map, a route saved, renamed, refused on a used name and opened in Plan with the KML summary; screenshotted. Not seen: touch
  input, the picker with a `.kmz`.

### P0: bugs and real-use blockers

- ✓ Enforced checklist: `ChecklistGate` writes `vehicle.checkListState` 1/2 and the checklist pops up 1 s after connecting when enforced;
  Arm stays unoffered until it passes (merged build: state 0 → 1 after ticking, Arm offer hidden → ready, slide card opens). The core
  still accepts a raw `vehicle.armed` debug write with the list unpassed; only the offer is withheld, as on Android.
- ✓ Host notices (`Controls/NoticeLayer`): banners as 4 s toasts, critical vehicle errors as an InfoBar off Fly with
  `vehicle.resetErrorLevelMessages` on close, app dialogs (Reboot vehicle, Open Setup, Use radio, plain OK), page navigation,
  `host.acknowledgeThrough`. Dialogs queue behind any open ContentDialog.
- ✓ Vehicle message banner and messages sheet on Fly ("Why it will not arm" when pre-arm blocks); the status sheet's Messages row opens it.
  Arming-check lines have tappable links (`Controls/ParameterLinks`): a `param://` link opens the parameter editor and the sheet comes back on
  Done; a missing parameter gets the core's notice; web links open. Driven in the slot with an injected check (the mocks send no libevents
  arming checks).
- ✓ Local record button and REC clock (`Controls/StreamShutter`): merged build wrote a 3.1 MB MP4 from the UDP test stream; hidden when a
  MAVLink camera is present, as on Android.
- USB/serial links: the core now lists COM ports on Windows, but no real device has been seen. ◐ Bluetooth links: the WinRT RFCOMM host is in
  (see Settings); a real adapter and radio are still needed.

### P1: core flying parity

- Fly prompts: ◐ voice alerts (92 Android clips via `MediaPlayer`, `SpeechSynthesis` fallback, mute, return-home alert, link-loss
  `MessageBeep`, which now also respects mute; player state verified, nobody listened on the VM) · ◐ automatic mission popups (Start Mission verified; Land Abort and
  Continue Mission share the path but were never triggered) · ✓ mission-complete / mission-interrupted dialog · ✓ close guard
  (`view.closeChecks`) · ✓ connecting card · ✓ traffic banner and list.
- Indicators: ✓ battery/GPS/RC/telemetry/link detail sheets (battery packs, return-budget bar plus a Fly top bar, Return, failsafes,
  display, silence line) · ✓ flight-time, RC-signal, RC-override (click clears) and RSSI cells · ◐ vehicle status sheet from the state
  chip (summary, Messages, Arm via the slide card, sensors into Setup, inline messages, Overall status links into the parameter editor;
  force arm not driven: no mock refuses arming) · ✓ title-bar overflow: cells that do not fit go behind the All status chevron and sheet.
- ✓ Multi-vehicle: fleet card (select, per-vehicle mode, fleet Arm/Disarm/Start/Pause), title-bar "Fly which aircraft?" picker, every
  vehicle on the map, tap a vehicle to fly it, handover notice. ✓ Fleet Return: the core's `mvRtl` sends `guidedModeRTL` to the armed
  selection (merged build: two mock ArduCopters selected, Arm (MV) and Return (MV) by slide, both in RTL with `returning`, then Return
  blocked as "Every armed selection is already returning.").
  ✓ Mission progress card with Skip · ✓ tap a Fly mission pin to set the current waypoint (16 px hit radius; a tap on the map tile while the
  video is main swaps back instead).
- Video: WHEP in the shipped runtime (rerun `build-video.ps1`, then check against a SFU) · ◐ gimbal indicator and control sheet (tilt,
  centre, point home, retract, yaw lock, acquire control, settings form verified; the take-control dialog was never triggered).
- Settings and Plan: ✓ pilot settings rows built from parameters (Settings > Aircraft safety and Aircraft limits) · ✓ waypoint camera
  section from `view.itemCamera`.
- ✓ Aircast device (aircastd): device home, the 11 device pages, the Wi-Fi and camera wizards and the Fly status chip, against the fake-device
  harness. ◐ Device camera previews: `VideoDriver` starts the WHEP pipelines on channels 2..5, but no frames were seen (no WHEP-serving device).

### P2: advanced and secondary

- Platform: ✓ ground-station position (`--position-source internalGps`, `Core/GcsLocation` feeds Geolocator fixes; operator dot, map
  framing without a vehicle, Remote ID GCS block, compass GCS bearing — proven with the core's NMEA source because the VM's Windows fix is
  10 km and refused) · ✓ Follow Me readout · ◐ GCS battery cell (built; the VM has no battery) · ✓ keep-display-awake (`powercfg
  /requests` lists the app while connected) · ✓ Aircast Cloud link and sign-in (stub account server; no real account used).
- Fly overlays: ✓ ESC and Remote ID cells (self-ID, emergency label) · ◐ GPS resilience, RTK (survey-in, base Save), Aircast link and
  joystick cells (built, no mock source) · ◐ VTOL cell (synthetic views only; no VTOL mock) · ✓ gripper panel with bays, problems and fix
  button (Payload tile in the deck) · ◐ orbit readout (built; the PX4 mock never reports orbiting) · ✓ proximity radar on the video and map ·
  ◐ obstacle arc, readout and map layer (constructed `view.obstacle` only; the layers toggle is live) · ✓ gimbal wedge and camera beam ·
  ◐ gimbal on-screen aim (mode switch verified; tap, drag and hold sends not driven) · ✓ virtual joystick (real mouse drags fly the mock) ·
  ◐ operator-control prompt and allow-takeover (synthetic views; the takeover setting and request invoke reach the core) · ◐ terrain-data
  warning (built; the stalled state cannot be driven) · ✓ offline status sheet · ✓ resume-failed prompt (only on Fly, as on Android;
  hidden elsewhere without being dismissed) · ✓ disarm and landed summary toast · ✓ first-run dialog.
- Video: ✓ grid lines and full-screen video (rail button, F11, Escape) · ◐ zoom (wheel and pinch call `stepZoom`; no zoom effect on the mock,
  pinch not driven) · ✓ double-click full screen · ✓ aspect ratio · ◐ camera details (camera and stream choice, time
  lapse and interval, tracking toggle and box, reset and format verified; thermal untested, tap-to-track not driven) · ✓ video stats pill
  (merged build: "31 fps · 720p" on the full-size video) · ◐ AR overlay (draws from an injected payload; live home and waypoints blocked by the
  core's `synthview.rs overlays()`) · ✓ detection boxes from aircastd (SSE stub) · ◐ synthetic view (DX12 SwapChainPanel; drag and
  drag aim not driven; see Fly).
- Settings: ✓ the light palette · ◐ app font size (XAML-built UI rescales live, code-built panels on their next render) · ◐ SD-card flasher (Settings > Aircast card over the core's Windows raw-disk backend; a virtual disk was written, no real reader seen).
- Plan: ✓ survey camera-calc block (brand/model, overlap, distance mode, set by altitude or ground resolution) · ✓ writes on a background lane; a big-survey edit still stalls
  the UI for the redraw (74–126 ms).

### P3: polish or low value on desktop

- ✓ 3D viewer as a Fly view (altitude, heading, route and the saved view fixed in round 10) · ◐ overlay layout editing (see Fly) · ✓ telemetry display editor (right-click entry not driven).
- ✓ Heading tape · ✓ guided-value presets and quick picks · ✓ Android's −/+ stepper, tap-to-type and probe sentence (merged build on the mock
  ArduCopter at 10 m: "already at 10.0 m and will not move", + three times "climb 3.0 m to 13.0 m", − "12.0 m", typed 25 "climb 15.0 m to
  25.0 m", and the slide flew it to 25 m) · ✓ mission identity confirm ·
  ✓ ROI tap sheet (map tap driven in-process) · ◐ lens measure (forget and the refusal seen; a successful measurement not seen) · ◐ USB-radio
  indicator (only the stopped state, with a forced port) · ✓ support-forwarding cell · ✓ deep-link prompt (`aircast-qgc://`, registered per
  user, single-instance redirect) · ✓ Plan double-click zoom · ✓ sensor-check and setup rows in the Settings tabs (on the Aircraft safety and
  Aircraft limits pages).

Not applicable on Windows: the phone Wi-Fi SSID, RadioMaster AX12 built-in sticks, portrait Fly, the notification permission, the
Android foreground service and the phone camera as a video source.
