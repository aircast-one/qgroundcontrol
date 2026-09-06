# Native Android Migration — Implementation Plan

Goal: replace QGC's Qt Quick UI with native Jetpack Compose on Android, **one view at a time, in a
shipping app**, keeping the C++ flight core untouched.

Sibling of `NATIVE_MACOS_REWRITE.md`. One core, two bridges, two native apps. Every path the Compose
UI needs is added to `QGCBridgeCore`, never to a platform head.

## Status — Phases 0 to 2 built and verified on device

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

Still QML, hosted under native tabs: Fly (map + video), Plan, Vehicle Setup, Analyze.

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
| 3 | Vehicle Setup | 15.0k | first HW gate | params done, calibration pending |
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

## Phase 4 — Plan · 7 weeks

Hardest phase. `MissionManager` (20.7k C++) survives entirely; the map-editing UI does not.

- **MapLibre Native for Android** for tiles and annotations. Open source, offline-capable, no Play
  Services dependency. The existing `QGCTileCacheWorker`/`QGCMapEngine` (2.7k) is a SQLite tile
  cache, not Qt map code, and is reusable behind a local tile source.
- Draggable waypoint annotations, polygon vertex handles, survey and corridor rubber-banding.
  MapLibre gives tiles and annotations; drag-to-edit is ours to build.
- Mission upload/download, geofence, rally points, terrain profile.

**Gate (HW):** a 200+ waypoint survey planned, uploaded, flown and downloaded byte-identical.

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
