# Native Android Migration — Implementation Plan

Goal: replace QGC's Qt Quick UI with native Jetpack Compose on Android, **one view at a time, in a
shipping app**, keeping the C++ flight core untouched.

Sibling of `NATIVE_MACOS_REWRITE.md`. One core, two bridges, two native apps. Every path the Compose
UI needs is added to `QGCBridgeCore`, never to a platform head.

## Status — Phases 0 to 3 built; Phase 5 part-built; hardware gates unmet

Built and run on a OnePlus 6 (LineageOS 22.2, Android 15) on 2026-09-06 against a MAVLink vehicle.
`aircast-android` is a Kotlin/Compose app that owns the Activity, the navigation and the chrome, with
QGC linked in as `AircastQGC.aar`.

| Gate criterion | Result |
|---|---|
| Native shell owns Activity, navigation, system bars | **pass** — 6 Compose tabs, QGC's QML toolbar deleted from the fly view |
| Settings read and write through the bridge | **pass** — 8 fact groups; `speedUnits=3` persisted to QGC's own `.ini` |
| Reflection bridge needing per-property code | **none** — every screen renders from `FactMetaData` |
| Flight actions round trip to a vehicle | **pass** — arm, mode change, RTL all acknowledged by the vehicle |
| Parameters read and write | **pass** — `PARAM_SET RTL_ALT 2500.0` received; values arrive cooked |
| Embedded QML video decodes | **pass** — MPEG-TS h.264 over UDP renders in the fly view PiP |
| Rotation and background/resume | **pass** — same PID through both, no activity recreation |

What landed, native:

- **Shell and navigation** — `MainActivity`, six tabs, Material 3. QGC's fly-view toolbar and the
  plan view's Fly/Plan switcher are gated off behind `toolbarVisible` / `hostProvidesNavigation`.
- **Settings** — eight fact groups rendered from metadata; switches, dropdowns and fields chosen by
  fact type.
- **Comm links** — list, connect, disconnect over `links.linkConfigurations`.
- **Vehicle status** — flight mode, armed, battery, satellites, HDOP, RC signal.
- **Flight actions** — arm/disarm, mode picker, takeoff, land, RTL.
- **Parameters** — search and edit over the full tree.
- **Activity duties** — wake lock, multicast lock, font scale, safe area, deep links, USB serial,
  system bar appearance, all moved out of `QGCActivity` into Kotlin.

Still QML, hosted under native tabs: Fly (map + video).

### Since that gate — Phase 3 done, Phase 5 started

Vehicle Setup and Analyze are now native too; see the Phase 3 status below for
what is built and what is deferred.

Phase 5 has been started from the parts that do not need the Phase 4 map:

- **Guided actions confirm before they fire.** Arm, Takeoff, Land and RTL each
  open a confirmation naming what the aircraft will do, over a control that only
  fires past 90% of its travel. Verified on device: a tap and a short nudge leave
  the vehicle disarmed, a full slide arms it.
- **Commands the vehicle would refuse are not offered.** The availability rules
  in `GuidedActionsController.qml` are ported natively from Vehicle properties,
  so Land and RTL grey out on a grounded disarmed vehicle.
- **The aircraft can say why it will not arm.** `prearmError` and the status
  text log surface in a banner and a readable message list.
- **Telemetry and the status strip read from fact metadata** rather than
  hardcoded names, units and thresholds.
- QML's own guided buttons are gated off behind `hostProvidesGuidedActions` so
  the two heads do not both offer Takeoff.

**The Plan tab is native.** The app imports `map-spike`, so the tab is MapLibre
with QGC's tile cache, mission editing, geofence, rally and vertex drag, under a
native file row. Video, joystick and the Phase 6 host teardown are untouched; the
AAR is still 78 MB because `QtQuickView` still hosts Fly.

**The native Fly view is blocked on video, not on the map.** `map-spike` already
has `VehicleMap` — vehicle position, heading, home, trail, link-loss and a follow
mode — and the guided actions, telemetry and prearm surfaces are already native.
What stops the tab being switched over is that the QML view it would cover is
where video renders, and the gate above records that PiP as passing. Putting a
native map on the Fly tab today trades a working video feed for a map we already
have elsewhere, which is a regression, so **video is the gating feature for
Phase 5, ahead of the Fly view itself.**

**Every hardware gate from Phase 3 onward is unmet.** Everything above was
verified against ArduCopter SITL and a OnePlus 6, never a real airframe.

**"Verified on the handset" does not cover the bridge.** `aircast-android` and
`map-spike` both build against the prebuilt `AircastQGC.aar`, so a change to
`QGCBridgeCore.cc` is invisible on the device until the Qt build regenerates the
AAR. Anything a device test exercised ran against whatever bridge that AAR was
built from. A C++ bridge change is verified by `test/Bridge` and by nothing else
until the AAR is rebuilt — read every device claim in this document with that
split in mind. `0f3bdbcd7` is the current example: the write path it fixes is
proven by the suite and has never run on a phone.

## Three measurements that set the architecture

**1. Android already has the library target.** Qt builds `AircastQGC.aar` via `androiddeployqt
--build-aar`, which the Compose app consumes as a file dependency. The macOS Phase 1 restructure —
exposing QGC as a library rather than an executable — is free here. It was a one-line fix to
`android/build.gradle`, whose `qtGradlePluginType` check used the wrong Gradle call and silently
produced an APK instead of an AAR.

**2. Qt already runs off the Android main thread.** The Qt event loop lives on a dedicated
`qtMainLoopThread`; the Compose UI runs on the Android main thread. macOS has to reach Phase 6 to get
this separation. Android has it on day one, and `runOnQtThread` in `QGCBridgeCore` already marshals
correctly across it.

**3. The core is nearly free of GUI.** Files including a `QtGui`/`QtQuick`/`QtQml` header:

| `Vehicle/` | `MissionManager/` | `Comms/` | `FactSystem/` | `Terrain/` | `Joystick/` | `QtLocationPlugin/` |
|---|---|---|---|---|---|---|
| 6/103 | 6/73 | 2/38 | 2/14 | 0/12 | 1/8 | 1/23 |

Most of that is benign: `QQmlEngine` for object ownership, and `QVector3D`/`QQuaternion`, which are
maths types in QtGui with no rendering. The genuine GUI dependencies are concentrated in two places —
`Vehicle/Actuators/GeometryImage` (`QPainter`, `QQuickImageProvider`) and `FirmwareUpgradeController`
(`QPixmap`, `QQuickItem`). Both fall inside Phase 3 and are the only core files this migration edits.

72k lines of QML that are pure UI. The seam is real.

## The hosting model

**`aircast-android` is the app. QGC is a library inside it.** Already true today.

```
        aircast-android  (Activity, Compose, navigation)
                          │
                    QGCBridge.java  (JNI head, 6 statics)
                          │
                    QGCBridgeCore  (path → QMetaProperty)
                          │
              C++ core — QtCore only, unchanged
    Vehicle · MissionManager · FactSystem · Comms · GPS · MAVLink
```

Three stages, one bridge throughout:

| | Who owns the Activity | Who renders the views | QML |
|---|---|---|---|
| **Today** | `aircast-android` | Compose + `QtQuickView` | Fly, Plan, Setup, Analyze |
| **Phases 3–5** | `aircast-android` | Compose + shrinking `QtQuickView` | shrinking |
| **Phase 6** | `aircast-android` | Compose only | none |

At Phase 6 the `QtQuickView` goes away and with it QtQuick, QtQml, QtLocation, QtMultimedia,
QtCharts, QtWidgets and QtPositioning. QtCore stays. Nothing in Kotlin changes except deleting the
`AndroidView` that hosts the QML.

## Order of conversion: risk ascending

Same rule as macOS. Start where a bug costs nothing, end where a bug hurts someone.

| # | View | QML today | Flight risk | Android status |
|---|---|---|---|---|
| 1 | Settings | 5.7k | none | **done** |
| 2 | Analyze | 1.3k | none | next |
| 3 | Vehicle Setup | 15.0k | first HW gate | native; HW gate open, motor test deferred |
| 4 | Plan | 12k in `QmlControls` | ground only | pending — the map is the work |
| 5 | Fly + video | 9.0k + 4.6k map | **critical** | status and actions done, map and video pending |
| 6 | Shell | — | — | pending |

`QmlControls/` (25.2k) drains continuously — each phase deletes only the controls its view used.

## Ground rules

Inherited from the macOS plan, unchanged:

- Phases run **in order**. Each ends in a build that ships.
- No phase starts until the previous phase's gate passes.
- Gates marked **HW** are verified against a real vehicle, never MockLink.
- Native UI lives in the `aircast-android` repo, never inside `src/` — this keeps upstream merges clean.
- Every bridge path is added to `QGCBridgeCore`, never to a platform head, or macOS forks.
- No Compose reimplementation of a control Material already has.
- A view is not converted until its QML is **deleted**. No dual-maintenance window.

One addition, learned the hard way this session:

- **The bridge needs tests.** It is the single point every screen depends on, and it currently has
  none. Path resolution, list indices, accessor-call segments and invoke conversion all need a
  fixture-based unit test in `test/Bridge/` before Phase 3 starts.

---

## Phase 2 — Analyze · 2 weeks

Smallest QML surface, read-only, zero flight risk. Proves charting and file transfer through the
bridge.

- Log download → Compose list plus the existing `LogDownloadController`.
- MAVLink inspector and console → Compose, backed by `MAVLinkInspectorController`.
- Vibration → a Compose chart.
- `Viewer3D` (1.7k) → decide keep-or-drop; it is the cheapest moment to drop it.

Needs from the bridge: a table/list model reader. `QAbstractTableModel` is not yet traversable —
`QmlObjectListModel` is. That is the one bridge addition this phase requires.

**Gate (HW):** log download from real hardware; chart values match the Qt build. `src/AnalyzeView/`
QML deleted.

## Phase 3 — Vehicle Setup · 5 weeks

The big evaporation: 15k lines of `AutoPilotPlugins` QML.

- The generic fact form from Settings replaces the hand-built parameter panels.
- Sensor calibration, radio calibration, motor test, power, safety, tuning.
- `GeometryImage` and `FirmwareUpgradeController` lose their `QPainter`/`QQuickItem` dependencies —
  the only core edits in this plan.

**Gate (HW):** full parameter tree loads and writes on PX4 and ArduPilot; accelerometer, compass and
radio calibration complete on real hardware.

### Status — every component is native or deliberately deferred; the HW gate is not met

The Setup tab is native. It reads the autopilot plugin's own component list and
renders a page per component, so it follows whatever the vehicle reports rather
than a hardcoded list.

Generic parameter forms, no new C++: Frame, Flight Modes, Power, Safety, Tuning,
Camera, Lights, PX4's Flight Behavior. Remote Support is a small custom page.

Sensor calibration is native and works: accelerometer, compass, level horizon,
gyro and pressure, with the six orientation tiles, progress and Next/Cancel.
Radio is a read-only check — mapped channel and live PWM per attitude control,
plus a monitor of every channel.

**The gate is still open.** Everything above was verified against ArduCopter
SITL and on a OnePlus 6, never against a real airframe. Accelerometer, compass
and radio calibration on real hardware remain unproven.

Three findings worth carrying:

- **More reduces to parameters than the plan assumed.** Frame was expected to
  need `APMAirframeComponentController`; on ArduCopter it is `FRAME_CLASS` and
  `FRAME_TYPE`. Check for a parameter pair before reaching for a controller — it
  also keeps `loadParameters()`, which rewrites the vehicle and reboots it, out
  of reach.
- **A controller that drives QML items cannot serve a second head, and the fix
  is mechanical.** `APMSensorsComponentController` and `RadioComponentController`
  each held `QQuickItem*` members and wrote into the view. Both now expose
  properties and signals instead; the QML binds to those and behaves as before.
  Doing this to the sensors one exposed a fork bug that made accelerometer
  calibration unusable on the desktop too — the sheet holding Next was gated on
  the Cancel button's enabled state.
- **State, never signal arguments.** The bridge watcher polls properties, so
  anything a native head must observe has to be readable. The calibration log
  and the per-channel PWM list were signal-only and had to become `statusText`
  and `rcValues` before Android could show them.

**Parameters** (`3013d68`): the list sliced its matches to the first 60, so a
parameter matching 61st could not be reached at all without narrowing the search,
and every edit re-read all 60 facts through the bridge. The `LazyColumn` now
takes the whole match list and each row loads its own fact, so only composed rows
cost a read.

**Testing parameters without SITL.** A heartbeat-only fake vehicle does not get
there: QGC asks for `AUTOPILOT_VERSION`, then tries the parameter download over
**MAVFTP** and keeps trying even when `AUTOPILOT_VERSION` advertises no
`PARAM_FTP`. The switch is not a capability at all —
`ParameterManager.cc:40` is `_tryftp(vehicle->apmFirmware())`, so FTP is
attempted for ArduPilot and only for ArduPilot. A fake vehicle whose heartbeat
reports `MAV_AUTOPILOT_PX4` takes the conventional `PARAM_REQUEST_LIST` path and
can serve parameters in a few lines. That is enough to exercise the list, the
search and a write end to end; it is not enough for anything firmware-specific.

**A refused setting said nothing.** The switch and the dropdown wrote through a
detached thread and discarded `Qgc.set`'s result, so a refused write left the
control showing the new value until the next poll put it back — which reads as
the app glitching rather than as a refusal. The text field already reported, so
two of three controls were silent and one was not. All three share one write path
now. The message says only that the change was not accepted: the reason the
bridge carries names a property and a class, which is true and no use to a pilot.

The same fire-and-forget shape is still in `VehicleUi`, `LinksScreen`,
`SensorsScreen`, `UnitsPage` and `InspectorScreen`. Arming and the guided actions
are covered by other surfaces — the prearm banner and the vehicle's own state —
but the link and calibration calls are not, and that is where to look next.

**Deliberately not built:** the motor test and CompassMot. Both spin the
propellers, `APMMotorComponent` sets `allowSetupWhileArmed`, and their gate needs
a supervised airframe. `Vehicle::motorTest` is already `Q_INVOKABLE` through the
`vehicle` root, so this is a UI and safety decision, not a bridge one. The Setup
pages say so and send the operator to desktop QGroundControl.

**Radio calibration cannot be tested in this rig at all.** Stick movement cannot
be simulated: `setRcChannelOverride` is accepted but SITL does not reflect it in
`RC_CHANNELS`, so the controller never sees a stick move. The long-disabled
`RadioConfigTest` hits the same wall — its mock input no longer drives channel
identification, so no channel ever maps. Covering radio calibration needs either
a transmitter on the bench or MockLink taught to drive `_inputStickDetect`; the
latter would revive that test too.

## Phase 4 — Plan · 7 weeks

Hardest phase. `MissionManager` (20.7k C++) survives entirely; the map-editing UI does not.

- **MapLibre Native for Android** for tiles and annotations. Open source, offline-capable, no Play
  Services dependency. The existing `QGCTileCacheWorker`/`QGCMapEngine` (2.7k) is a SQLite tile
  cache, not Qt map code, and is reusable behind a local tile source.
- Draggable waypoint annotations, polygon vertex handles, survey and corridor rubber-banding.
  MapLibre gives tiles and annotations; drag-to-edit is ours to build.
- Mission upload/download, geofence, rally points, terrain profile.

**Gate (HW):** a 200+ waypoint survey planned, uploaded, flown and downloaded byte-identical.

**Task-level UX review of the finished tab (2026-09-08).** Every command from
`PlanView.qml` is now present, and that is the problem: twelve controls of
identical weight, so the action that completes the job — Upload — is styled
exactly like Fit, which recentres the view. There is no primary action. Measured
off the handset, the map gets 34% of the screen (header 18, map 34, terrain
profile 11, Follow 5, button grid 11, nav 21), so on a screen whose job is
editing a map the chrome outweighs the canvas about two to one. The bottom grid
also mixes vehicle sync, item creation and view control with nothing separating
them. None of this is a bug; it is the difference between a tab that has every
command and one that shows a pilot which to press. The fix spans both the shell
and the map module — the file row should collapse to a single File button as QGC
has, but only once Upload is visually primary, or Save goes two taps away while
the text beside it says "unsaved changes".

**Boundary import** (`230b5d9`): a site boundary arrives as a file, and
`insertComplexMissionItemFromKMLOrSHP` had been in `MissionController` all along
with nothing calling it from a native head. Two things the Android side had to
get right: the cache copy keeps the source suffix, because `QGCMapPolygon` picks
its parser off the extension; and the pattern is read from
`complexMissionItemNames` rather than hardcoded, because the C++ compares against
`SurveyComplexItem::name`, a `tr()` string that would stop matching under
localisation — reading it back also offers only the patterns the vehicle
supports. A file with no usable area is inserted anyway by `QGCMapPolygon`, which
reports its parse failure through `showAppMessage`, so the import checks the
distance of what came back and removes the empty item rather than leaving it in
the plan.

**An imported pattern could not be saved** (`fa8ba8de3`). Finding the import
worked was not the same as finding it useful: `_insertComplexMissionItemWorker`
puts every new complex item into wizard mode so the operator finishes drawing it,
and the item leaves wizard mode when its area becomes valid. A pattern built from
a file loads its area in the constructor, so that signal fires before the worker
runs and nothing clears the flag. `wizardMode` is half of `NotReadyForSaveData`,
so an imported survey reported not-ready for the life of the plan — save refused,
upload refused, and the message told the operator to draw an area that was
already drawn. Verified on the handset after rebuilding the AAR: an imported
survey now saves, 55 KB of plan JSON where the save was previously refused.

**The Android link had been broken for hours** (`22f25f637`), which is why no
C++ change had reached a handset. `GStreamer::createNativeSink` calls
`qgc_video_attach_appsink` unconditionally while `QGCVideoC.cc` was listed only
under `if(APPLE AND NOT IOS)`, so `ld.lld` failed on `undefined symbol`. It builds
clean on macOS, so the stream that introduced it had no way to see it. Both
Android modules consume the prebuilt AAR, so this is a silent, total block on
every bridge change — **when a bridge fix seems not to take effect on the device,
check that the AAR actually rebuilt before doubting the fix.**

**Known gap — the map draws less than the plan holds.** Waypoints, fences, rally
points and surveys are drawn; corridor scans, structure scans and landing
patterns are not. They still count in the distance and duration and they still
upload to the aircraft, so a pilot can see a map that looks like their whole
plan and fly one that is not. Opening a `.plan` file is the likeliest way such
an item arrives, since it was planned on desktop.

A warning was attempted three times and withdrawn (`58295f8`). Each predicate —
is the item complex, did anything on the map come from it, does it specify a
coordinate — fired on ordinary plans, because Mission Start satisfies all three
while being drawn or being nothing. A warning that fires on every plan is worse
than the gap it names, since it trains the pilot to ignore the next one. Whoever
picks this up should start from a plan that actually contains a corridor scan
and read its real serialisation, rather than reasoning about what the fields
ought to mean. The gate above is unaffected for surveys but cannot be claimed
for a mixed plan.

### Plan files — scope the plan missed

This phase lists mission upload and download and its gate is a vehicle round
trip, so saving a plan to a file never appeared in it. `PlanView.qml` has it
today, through `QGCFileDialog`, and it is not gated off on mobile — so a user on
the current Android build can save and open `.plan` files. The native Plan tab
cannot, and Phase 6 deleting that view makes the loss permanent.

That makes it a **regression to prevent**, not a feature to consider, and it has
to land before the Phase 6 teardown rather than after.

It is app-shell work rather than map work. `PlanMasterController` already exposes
`loadFromFile`, `saveToFile`, `saveToCurrent` and `saveToKml` as `Q_INVOKABLE`,
alongside `currentPlanFile` and `dirty`, so the missing half is entirely the
Android file layer:

- **Use the system document picker.** A plan belongs to the person who made it:
  it has to survive uninstall and be shareable off the device, and app-private
  storage fails both. That also avoids inventing an in-app file browser.
- **The picker returns a `content://` URI and the controller wants a path**, so
  the shell copies in to cache before `loadFromFile`, and after `saveToFile`
  streams the result back out to the chosen document.
- **`saveToCurrent` and `dirty` carry Save versus Save As**, which is what keeps
  a single silently-overwritten slot from being the design.

**Landed** (`aircast-android` edf764c): `PlanFiles.kt` and `PlanTab.kt`. Open,
Save and Save as verified on the OnePlus 6 against a real document — the written
file is valid plan JSON, reopening it loads, and Save writes back to the opened
document (`ACTION_OPEN_DOCUMENT` does grant write; confirmed by mtime, not by the
success notice).

Two corrections to the paragraph above:

- **`saveToCurrent` is the wrong call under SAF.** `currentPlanFile` is the cache
  path the shell handed to `saveToFile`, not the user's document, so
  `saveToCurrent` would write to cache and never reach their file. Save versus
  Save As is carried by whether the shell is holding a document URI.
- **`CreateDocument` must be given a wildcard MIME.** Naming it
  `application/json` makes DocumentsUI append `.json`, and `mission.plan.json`
  does not match the `*.plan` filter desktop QGC opens with.

**Save readiness** (`e18f84f`): `PlanView.qml` blocks save and upload unless
`readyForSaveState()` is `ReadyForSave` — a survey still fetching terrain stores
wrong altitudes, an item missing a position stores an incomplete mission. The
native tab wrote the file regardless, which is a worse regression than the
missing file layer, because the result looks like a valid plan. The guard now
runs before the picker opens, and an unreadable check refuses rather than
allows.

The unsaved-changes label is verified on the handset through all four of its
states. The map in the Plan tab already edits plans — a long press adds a
waypoint — so `plan.dirty` could be exercised directly: `New plan` becomes
`Unsaved plan` on the first edit, and `mission.plan` becomes
`mission.plan \u00b7 unsaved changes` after a save and a further edit. The saved
file grew from 589 to 1168 bytes across it, which is the edit landing in the
file rather than only in the label.

**Gating** (`4e294b1`): the file scope is closed. `PlanView.qml`'s file menu
gates every action, and the native row was missing all of it — most seriously,
Open discarded unsaved edits without asking. Open now confirms when the plan is
dirty; Save and Save as need `containsItems` and no sync in progress; Export KML
needs `plan.missionController.containsItems` rather than the master's, because
`saveToKml` writes only the mission and a fence-only plan would export an empty
document. `saveToKml` returns void, so success comes from the file on disk.

The whole file scope is verified on the handset, including the negative cases:
the menu items are disabled on an empty plan and enable when a waypoint is
added, which is what proves the nested property path resolves rather than
merely reading false.

**New plan and Clear mission** (`c03ed83`) complete the file menu. The trap
worth recording: `removeAll` clears `currentPlanFile` only when the controller
is offline, and the SAF document URI the Android shell holds is a separate thing
that no C++ path would ever clear. Any command that empties the plan must tell
the shell to forget the document, or the next Save writes a blank plan over the
user's mission and reports success. This is why a clear belonging to the map
module cannot stay inside it — the dependency runs app to map, so a clear there
has to be a callback the app supplies.

`Clear mission` is verified both ways against a live vehicle: greyed with none,
enabled with one, so `plan.offline` resolves rather than reading a stuck false.
The sync gate disables Open and Save while the clear is in flight.

Its notice was wrong and is worth recording. `removeAllFromVehicle` returns
void, and a bridge invoke reporting success means the method was called, not
that the aircraft complied — firing it at a sim that never acknowledges still
produced "Mission cleared from the vehicle." The bridge polls properties and has
no signal for the vehicle having obeyed, so the notice now says the clear was
sent. **Any void `Q_INVOKABLE` that commands the vehicle has this shape**: the
native head can report that it asked, never that it was done.

**Whether a call can be checked at all is a property of the C++ signature**, and
nothing on the native side shows which case you are in. Three categories:

| Shape | What a success reply means | How to check |
|---|---|---|
| invoke returning a pointer | a null arrives as `{"kind":"null"}` in `result` | require an object in `result` |
| invoke returning `void` | the method was found and called | read the state back |
| `set` | `setProperty` returned true | re-read; a Fact may clamp to its own range |

`LinkManager` is the worked example: `createAndConnectLink` returns `bool`, while
`createConnectedLink`, `removeConfiguration` and `LinkInterface::disconnect` are
all `void`. Reporting built on `ok` therefore covered the one path that needed it
least, and the three that needed it were checking a value that cannot carry the
answer. They read the link list back now (`aircast-android`).

**Two guards we rely on were put there for other reasons.** An out-of-range
settings value never reaches the clamp, because the field calls the Fact's own
`validate` first — added to produce a message, not to foreclose clamping. The map
module's radius has the same property from a 20 m floor that exists so nobody
builds a one-metre fence. Neither protection was designed for the job it is
doing, so the next person to touch either will be thinking about the reason it
exists rather than the one it also serves.

**"It re-reads" means two different contracts.** Settings facts go through a
watch, so any value on screen comes from the poll and a change from any source
surfaces. Parameters rows are not watched — each loads on demand and re-reads
after its own write, so a clamp surfaces once and a change made by the vehicle or
another GCS never does. That is the right trade at two hundred paths, but it is
not the settings contract and reads identically in the code.

Two corollaries, both found by auditing against that rule rather than by a
failure:

- **A property that correlates with success is not evidence of it.** `dirty`
  falling to false after a sync means the controller has no unsent changes — a
  statement about the controller, not about the aircraft. It reads like an
  acknowledgement and is not one. The same bit caught the Android head from the
  other side: `saveToFile` clears it *only when offline* — QGC's own comment says
  so — because `dirty` means "not sent to the vehicle", not "not written to a
  file". A saved plan therefore kept reading "unsaved changes" with a vehicle
  attached. With no vehicle the two meanings coincide; with one, the honest claim
  is the weaker one that always holds when the bit is set: the plan is not on the
  aircraft (`7cff5b5`).
- **Do not claim to know why you failed.** `planLoad` reported both a rejected
  file and a bridge call that never answered as "That is not a plan file",
  printing our own failure as an accusation against the user's document. The two
  now get different sentences (`f36aa76`).

## Phase 5 — Fly and video · 7 weeks

Safety-critical, deliberately last.

- Vehicle marker, trail, instruments and telemetry on the Phase 4 map.
- Guided actions, confirmation slider, failsafe surfaces. Arm/disarm, mode, takeoff/land/RTL already
  work natively and move onto the native map.
- Joystick: `Joystick/` C++ survives; HID rebinds to Android's `InputDevice` API. `JoystickAndroid`
  already exists.
- Video: GStreamer is not Qt and survives. The Android build already carries `libgstapp` and
  `libgstandroidmedia`, so the path is `appsink` → `ImageReader`/`Surface` → Compose, replacing the
  QML GL sink. Hardware decode via `androidmedia` is already in use.

**Gate (HW):** every guided action verified on PX4 and ArduPilot. Confirmation control cannot be
actuated accidentally. Sub-200 ms glass-to-glass on WHEP. A 30-minute flight with flat memory.

## Phase 6 — Shell · 2 weeks

Cheaper than macOS, because Qt is already off the main thread.

- Delete the `QtQuickView` host and `AndroidHost.qml`.
- `QGCApplication` drops from `QApplication` to `QCoreApplication`; the embedded-host boot path
  becomes the only path.
- QtQuick, QtQml, QtGui, QtLocation, QtMultimedia, QtCharts, QtWidgets and QtPositioning drop from
  the Android dependency set. Expect the 82 MB AAR to roughly halve.
- Release build, signing and CI for `aircast-android`, which today only builds debug locally.

**Gate:** two weeks of internal flying with the previous release as fallback, then delete the
fallback.

---

**Total ≈ 23 weeks** on top of what is already built. Android runs shorter than macOS's 35 because
Phases 0 and 1 are done, the library target was free, and the thread separation already exists.

## Risks

1. **Flight-test throughput is the critical path, not code.** Four of five remaining gates need real
   airframes. Book hardware time before writing code.
2. **Map editing.** Phase 4 is the one place Qt does real work MapLibre does not replace. If the
   schedule slips, it slips here. Same risk as macOS, different SDK.
3. **The bridge has no tests.** Every native screen depends on it and nothing guards it. This is the
   one item that should be fixed before Phase 3, not after.
4. **Blocking calls from the Android main thread.** `runOnQtThread` uses a `BlockingQueuedConnection`
   when called off the Qt thread. A Qt thread that is itself waiting on the Android main thread would
   deadlock. Not observed, not prevented.
5. **Watcher latency.** Watched paths are polled and diffed at 200 ms, not signal-connected. Fine for
   status, too slow for an attitude indicator. Phase 5 needs the notify-signal path.
6. **macOS divergence.** Every path Compose needs must also serve SwiftUI. Add to `QGCBridgeCore`,
   never to a head.

## Not covered

- **iOS, Linux, Windows.** They stay on Qt. This is an Android frontend, not a migration of the
  project.
- **De-Qt of the core.** Stripping QObject ends the upstream merge stream permanently. Don't.
