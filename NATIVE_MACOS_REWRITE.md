# Native macOS Migration — Implementation Plan

> **Ordering decided: shared layer first.** The inventory below showed two thirds of the
> QML trapped behind the final, safety-critical phase under a risk-ascending order. That
> tail is unmanageable, so the reusable controls are converted first and later phases
> compose rather than block. Phases are re-sized on measured numbers.

## What the inventory found

Phase sizes in this plan were estimated by eye. Measured, they are wrong:

| | plan said | actual |
|---|---|---|
| Settings | ~2k | 5,703 (`UI/AppSettings`) |
| Plan | ~12k | 4,608 (`FlightMap` + editors) |
| Fly | ~14k | 9,012 (`FlightDisplay`) |
| Vehicle Setup | ~15k | 14,993 |

Worse, `QmlControls/` is not the design system this plan calls it. It holds
`PlanView.qml` (1,467 lines), `MissionItemEditor.qml`, `GeoFenceEditor.qml` and the
mission-item editors alongside the actual reusable controls. "25k evaporates" is false.

The real constraint is the reference graph. Assigning every QML file to the **last** view
group that reaches it gives the earliest phase it can be deleted in:

| deletable in | files | lines |
|---|---|---|
| 1 Settings | 6 | 628 |
| 2 Analyze | 6 | 1,218 |
| 3 Setup | 87 | 16,275 |
| 4 Plan | 8 | 907 |
| 5 Fly | 315 | 47,459 |
| not statically reached | 44 | 5,609 |

**Two thirds of the QML can only be deleted in the final phase**, because 175 files
(27,949 lines) are shared across view groups. Phases 1-4 together retire 26%.

**The decision this forced.** Risk-ascending ordering would have put 47,459 of the 72,096
lines in the last phase, which is also the one where a mistake hurts someone. That is the
wrong place to concentrate schedule risk, so the shared control layer converts first:
`macos/Sources/Controls.swift` holds the label/value arrangements every view composes
from, and each feature since has been built on it rather than growing its own. Controls
are the least safety-critical thing to get wrong and the thing everything else waits on.

Three consequences of the graph:

- The ground rule "a view is not converted until its QML is deleted" is unachievable for
  phases 1-4. Their QML has to stay alive for the Fly view.
- Risk-ascending order concentrates two thirds of the work in the last phase, which is also
  the safety-critical one. All schedule risk lands where it is least affordable.
- The 44 unreached files are **not** dead code. Mission item editors are loaded from C++
  `editorQml` properties (`RallyPointController`, `GeoFenceController`), so a static scan
  cannot see them. They need a separate runtime inventory before they can be scheduled.

Still missing before this counts as rigorous: a dynamic-load inventory; a map of which of the
1,672 bridge properties each view actually needs and which are unreachable through the path
resolver; a test strategy for the SwiftUI side (78 suites cover C++/QML today, none cover
native); per-view parity specs rather than one-line gates; and validated throughput, of which
Phase 0 is the single data point.


Goal: replace QGC's Qt Quick UI with native SwiftUI on macOS, **one view at a time, in a shipping
app**, keeping the C++ flight core untouched.

No parallel rewrite, no big-bang cutover. Every phase ships to real users with some views native and
the rest still QML.

## Status — Phase 0 built and verified, gate PASSED

Built and run against ArduPilot SITL on 2026-09-06. The native SwiftUI window ran inside the Qt
process with the QML Fly view live behind it.

| Gate criterion | Budget | Measured |
|---|---|---|
| Bridge round trip, 10 paths per tick @ 10 Hz | < 16 ms | **1.18 ms worst**, 0.48 ms typical |
| Ticks over budget | 0 | **0 of 1606** |
| QML render loop while native window polls | no stall | fresh 1280x823 frame served |
| Reflection bridge needing per-property code | none | **none** — 6 attitude Facts and 3 scalars, zero bespoke code |

Heading read through the bridge (351 deg) matched the QML compass exactly. The `watch` path
delivered real transitions: link disconnect drove `vehicles.activeVehicleAvailable` 1 to 0 and
`vehicle.armed` / `vehicle.flightMode` to null, reconnect restored all three.

Neither kill criterion fired. The two toolkits share one `CFRunLoop` with no render control, no
input forwarding and no second event loop.

What landed:

- `src/Bridge/QGCBridgeCore.{h,cc}` — the platform-neutral bridge, lifted out of `src/Android/`.
  `Watcher` now calls a `std::function` handler instead of JNI directly.
- `src/Bridge/QGCBridgeC.{h,cc}` + `module.modulemap` — a six-function C ABI. Because the bridge
  already speaks JSON strings, **no Objective-C++ was needed**: Swift imports the C header directly.
- `src/Android/QGCBridge.cc` — 518 lines down to 120, now only the JNI head.
- `macos/Sources/*.swift` — `Bridge`, `Telemetry`, `TelemetryView`, `NativeWindow` (~300 lines).
- `macos/CMakeLists.txt` — Swift static library linked into the Qt executable. Needs
  `AUTOMOC OFF` (Qt's global autogen injects a C++ file into a Swift-only target) and
  `$<$<COMPILE_LANGUAGE:Swift>:...>` guards on the `-Xcc` module-map flags.
- `--native-window` flag in `src/main.cc`, macOS only.

Run it: `QGC_DEBUG_API_PORT=8778 build-test/Debug/AircastQGC.app/Contents/MacOS/AircastQGC --allow-multiple --native-window`

Since then, the ownership inversion landed too: `src/main.cc` splits into `qgc_start` / `qgc_run` /
`qgc_shutdown` (declared in `src/Bridge/QGCEntry.h`), and on macOS `main()` is a three-line
trampoline into Swift's `AppShell.run`, which sequences all three and owns the menu bar. Verified:
the running app's menu bar is `Apple · Aircast QGC · Window`, with the Window menu carrying the
native telemetry item — Qt builds no such menu. The unflagged path is unchanged and still launches
the normal app.

Qt keeps its `NSApplicationDelegate` deliberately: `QGCApplication::event` depends on it for the
`QFileOpenEvent` deep links. That delegate is the last thing to move, at Phase 6.

**QGC is a library on macOS (2026-09-07).** The project target `AircastQGC` builds
`libAircastQGC.dylib` — every subdirectory still adds its sources to that name — and a thin
`AircastQGCApp` executable target (one file, `src/main.cc`) links it and produces the same
`AircastQGC.app`. The dylib is copied into `Contents/Frameworks` after every link and the app's only
rpath is `@executable_path/../Frameworks`, so a cloned bundle stays self-contained and a relink of
the core never invalidates a running clone. `qgc_start` / `qgc_run` / `qgc_shutdown` moved from
`main.cc` into `src/Bridge/QGCEntry.cc`, compiled into the core on every platform; `main()` is all
that is left outside it. Build the app with `cmake --build <dir> --target AircastQGCApp` (or no
target); `--target AircastQGC` now builds only the core.

**Qt now renders inside a native window.** `qgc_embed_main_window` takes the `NSView` behind a
Swift-owned `NSWindow` and reparents the root `QQuickWindow` onto it with
`QWindow::fromWinId` + `setParent`. No QML changed. Verified: the full Fly view renders inside a
native titled window, resizing the `NSWindow` drives Qt's scene (1400x900 to 1000x700 followed
exactly), and synthetic AppKit clicks reach QML. Qt's own `NSWindow` survives as an invisible 0x0
shell, which `/native/windows` reports at index 0.

**Native observability.** The Qt debug API gained a `/native/*` surface, backed by Swift through
`QGCNativeDebugC.h`, because verifying any of the above through accessibility scripting and
screen-coordinate arithmetic was slow and wrong often enough to matter:

| Route | Gives you |
|---|---|
| `/native/windows` | titles, CGWindowIDs, capture rects (top-left origin), `hostsQt` |
| `/native/click` | click by content-view coordinate; raises the window first |
| `/native/menu` | the menu bar as a tree |
| `/native/menu/invoke` | fire an item by `"Window/Native Telemetry"` |
| `/native/bridge` | bridge call count, last, worst and mean milliseconds |

`tools/qgc-mcp/server.py` wraps these as `native_windows`, `native_screenshot`, `native_click`,
`native_menu`, `native_menu_invoke` and `bridge_stats`. `native_screenshot` captures by CGWindowID
via `screencapture -l`, so it gets the right window even when it is buried — a screen-rectangle
crop does not, and picks up whatever is in front. It captures Qt's render surface correctly.

Per-call bridge cost measured this way is **0.009 ms mean, 0.135 ms worst over 7,640 calls** — the
earlier 1.15 ms figure was ten paths per tick, not one call.

**Correction to an earlier note.** A previous commit recorded that a disconnected link
keeps reporting connected, and called it parity with QML. That was wrong. `_link` is a
`std::weak_ptr`, so it clears within about two seconds once the last owner releases. What
was actually happening: `LinkManager::createConnectedLink` had no guard against a
configuration that already owns a link, so connecting twice built two, and one disconnect
left the other alive. Fixed in `LinkManager`, where it protects QML, Compose and SwiftUI
alike, and covered by `LinkDuplicateConnectTest`.

**Driving the native UI in tests.** Synthesised input does not work when the screen is
locked: no window can become key, so a click either lands on the wrong window or is
swallowed, and both look like a pass. Screenshots by CGWindowID keep working. So the
native UI is driven by *identity*, the counterpart of QGC's objectName-addressed `/ui/*`
surface for QML:

- `/native/probe` lists the registered probes and reports `screenIsLocked`.
- `/native/probe?id=settings` reads a probe's state; `&action=select&page=Connections`
  drives it. Same for `id=links` with connect/disconnect/create/remove/add/edit.
- A store conforms to `Probeable` and registers itself; view state a test needs to reach
  belongs in the store rather than in `@State`, precisely so the probe can reach it.
- `/native/click` and `/native/type` still exist for an unlocked screen, and now refuse
  with an explanation when the screen is locked instead of failing mysteriously.

**The in-tree Swift is optional.** `QGC_NATIVE_UI` (default ON) builds `macos/Sources` into the app
and defines `QGC_NATIVE_UI` for `main.cc`, which then trampolines into `AppShell.run`; OFF gives the
plain Qt app. A universal configure (`x86_64;arm64`) turns it OFF with a status message instead of
failing, so the production release build ships again. Per-arch Swift plus `lipo` is not needed here:
the Swift's universal build belongs to the app repo's Xcode project.

**The SDK artifact exists.** `cmake --install <dir> --component sdk --prefix <out>` installs
`lib/libAircastQGC.dylib` and `include/AircastQGC/` with the bridge C headers and `module.modulemap`.
The release CI uploads `qgc-macos-sdk`: that prefix plus the deployed bundle's `Frameworks`,
`PlugIns` and `Resources` (Qt, GStreamer, wfb and the QML modules, already relinked by
`macdeployqt`), which is everything an `aircast-macos` app has to embed.

Open: the `aircast-macos` repo itself. It consumes the SDK tarball, owns the bundle, Info.plist,
entitlements, icon, menus and signing, and pins a QGC version; `macos/Sources/*.swift` moves there
when it exists. The C bridge headers stay here permanently — they are the macOS `QGCBridge.java`.
`aircast-android` should move from its relative path into `build-android/` to the same published
artifact model at the same time.

---

## Two measurements that set the architecture

**1. The core is already free of QtGui.** Files including a `QtGui`/`QtQuick`/`QtQml` header:

| `Vehicle/` | `MissionManager/` | `Comms/` | `FactSystem/` | `FirmwarePlugin/` | `AutoPilotPlugins/` | `Camera/` | `GPS/` | `Terrain/` | `Joystick/` | `Gimbal/` | `Settings/` |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0/103 | 0/73 | 0/38 | 0/14 | 0/28 | 0/90 | 0/14 | 0/26 | 0/12 | 0/8 | 0/4 | 2/48 |

151k lines of C++ needing only QtCore. 72k lines of QML that are pure UI. The seam already exists.

**2. The bridge already exists.** `src/Android/QGCBridge.cc` (518 lines) is a generic path-addressed
`get`/`set`/`invoke`/`watch` API over `QMetaProperty`/`QMetaMethod`, JSON in and out, with a JNI head
bolted on at line 411. Lines 38–405 are platform-neutral. SwiftUI needs the same core with an
Objective-C++ head.

That is the leverage: **you do not hand-write 1,672 property bridges.** SwiftUI reads the core
through the same reflection surface QML uses.

## The hosting model

**`aircast-macos` is the app. QGC is a library inside it.** Same shape as `aircast-android`
consuming `AircastQGC.aar`.

The thing that makes this cheap: *who calls `exec()` is invisible to SwiftUI.* The Phase 0 spike
proved SwiftUI windows, `NSHostingView` and AppKit all behave normally under
`QApplication::exec()`, because Qt's macOS dispatcher runs on `CFRunLoop` — it *is* the Cocoa run
loop. So Swift can own the entire application without owning the loop.

Three stages, one C API throughout:

| | Who owns the bundle, menus, windows | Who drives the run loop | QML |
|---|---|---|---|
| **Spike** (done) | Qt | `QApplication::exec()` | all views |
| **Phases 1–5** | `aircast-macos` | `QApplication::exec()`, called *from* Swift | shrinking |
| **Phase 6+** | `aircast-macos` | `NSApplicationMain`; Qt core on a worker thread | none |

During the migration, `main.swift` is a plain top-level file, not `@main`:

```swift
qgc_start(CommandLine.argc, CommandLine.unsafeArgv)   // builds QApplication, no exec
AppDelegate.install()                                  // Swift owns NSApp, menus, windows
qgc_run()                                              // enters QApplication::exec()
```

Swift owns every line of UI, the Info.plist, the entitlements, the icon, the signing and the repo.
Qt just holds the loop — and only because **QtQuick requires the main thread on macOS for as long as
any QML view is left**. That is the one real constraint, and it expires with the last QML view.

At Phase 6, `qgc_run()` changes to start a `QCoreApplication` on a worker thread and return, and
Swift calls `NSApplicationMain`. Nothing else in Swift changes. The marshalling this needs is
already shipped: `runOnQtThread` in `QGCBridgeCore.cc` uses a `BlockingQueuedConnection` whenever the
caller is off the Qt thread, and a direct call when it is not.

```
        aircast-macos  (NSApp, menus, windows, SwiftUI)
                          │
                    QGCBridgeC  (6 C functions, JSON in / JSON out)
                          │
                    QGCBridgeCore  (path → QMetaProperty)
                          │
              C++ core — QtCore only, unchanged
    Vehicle · MissionManager · FactSystem · Comms · GPS · MAVLink
```

What this costs: qgroundcontrol has to expose QGC as a **library** target rather than an executable.
Android gets that free — Qt builds a shared library for the `.aar` — but on macOS it is a real
restructure of the main target, and it is the first task of Phase 1.

The bundle carries both toolkits until Phase 6, roughly 40 MB heavier. That is the price of not
doing a big-bang cutover, and it is worth paying.

Two things it buys: upstream QGC merges keep applying, because the core is never touched — you fork
the UI, not the project. And the Android Compose frontend becomes a sibling rather than a competing
effort: one core, two bridges, two native apps.

## Order of conversion: risk ascending

Deliberately **not** most-valuable-first. Start where a bug costs nothing, end where a bug hurts
someone, so the safety-critical view is built by people already fluent in the stack.

| # | View | QML today | Flight risk | Why here |
|---|---|---|---|---|
| 1 | Settings | ~2k | none | Self-contained Fact forms. Proves the bridge. |
| 2 | Analyze | 1.3k | none | Read-only. Swift Charts, log download. |
| 3 | Vehicle Setup | ~15k | first HW gate | Biggest deletion. Generic Fact form pays off. |
| 4 | Plan | ~12k | ground only | Hardest UI. Map editing is the real work. |
| 5 | Fly + video | ~14k | **critical** | Last, most-tested, most-reviewed. |
| 6 | Shell | — | — | Delete `QApplication`. |

`QmlControls/` (25k) drains continuously — each phase deletes the controls only its view used.

## Ground rules

- Phases run **in order**. Each ends in a build that ships.
- No phase starts until the previous phase's gate passes.
- Gates marked **HW** are verified against a real vehicle, never MockLink.
- Native UI lives in a new top-level `macos/`, never inside `src/` — this keeps upstream merges clean.
- Every bridge path is added to `QGCBridgeCore`, never to a platform head, or Android forks.
- No SwiftUI reimplementation of a control AppKit already has. That mistake is what `QmlControls/`
  became.
- A view is not converted until its QML is **deleted**. No dual-maintenance window.

---

## Phase 0 — Spike · 2 weeks · go/no-go

- Split `QGCBridge.cc` into `QGCBridgeCore.cc` (lines 38–405), `QGCBridgeJNI.cc` (existing head),
  `QGCBridgeObjC.mm` (new).
- Swift package wrapping the ObjC++ bridge as an `@Observable QGCObject`.
- One `NSWindow` opened from the running Qt app showing live vehicle attitude from the bridge.

**Gate (HW):** a native window inside the Qt app shows live telemetry from a real Pixhawk. Bridge
round-trip under 16 ms at 10 Hz. No event-loop stalls in the QML views while it is open.

**Kill criteria:** if the two toolkits cannot share the run loop, or the bridge needs hand-written
per-property code, stop. That is what these two weeks buy.

## Phase 1 — Library target, app shell, Settings · 5 weeks

- **qgroundcontrol exposes QGC as a library** instead of an executable, plus `qgc_start()` /
  `qgc_run()` alongside the existing `QGCBridgeC`. This is the task that unblocks everything else.
- **`aircast-macos` repo created**, owning the bundle, Info.plist, entitlements, icon, menus and
  signing. `macos/Sources/*.swift` from the spike moves there.
- Generic `FactForm` driving off `FactMetaData` through the bridge — the component the whole
  migration leans on.
- Native window, native `NavigationSplitView`, native preferences idiom.
- Delete `src/UI/preferences/` QML.

**Gate:** `aircast-macos` launches as its own app with QGC linked in, native menu bar, and every
setting reads and writes; values agree with the Android frontend against the same `SettingsManager`.

## Phase 2 — Analyze · 3 weeks

- MAVLink inspector, log download, GeoTag, vibration → Swift Charts.
- `Viewer3D` → SceneKit.

**Gate (HW):** log download from real hardware; chart values match the Qt build's numbers.

## Phase 3 — Vehicle Setup · 6 weeks

The big evaporation: ~15k lines of `AutoPilotPlugins` QML plus most of the controls it used.

- `FactForm` from Phase 1 replaces the hand-built parameter panels.
- Sensor calibration, radio calibration, motor test, power, safety, tuning.
- The sidebar redesign carries over as `NavigationSplitView` — closer to the System Settings model it
  was imitating than the QML ever got.

**Gate (HW):** full parameter tree loads and writes on PX4 and ArduPilot; accelerometer, compass and
radio calibration all complete on real hardware.

## Phase 4 — Plan · 8 weeks

Hardest phase. `MissionManager` (20.7k C++) survives entirely; the map-editing UI does not.

- MapKit with `MKTileOverlay` fed by the existing `QGCTileCacheWorker`/`QGCMapEngine` — that 5.7k is
  a SQLite tile cache, not Qt map code, and keeps working.
- Draggable waypoint annotations, polygon vertex handles, survey and corridor rubber-banding. MapKit
  gives tiles and annotations; drag-to-edit is yours to build.
- Mission upload/download, geofence, rally points, terrain profile.
- The grouped-card inspector language ports to native `List`/`Form` — cheaper than it was in QML.

**Gate (HW):** a 200+ waypoint survey planned, uploaded, flown and downloaded byte-identical.

## Phase 5 — Fly and video · 8 weeks

Safety-critical, deliberately last.

- Vehicle marker, trail, instruments, telemetry chips on the Phase 4 map.
- Arm/disarm, mode change, takeoff/land/RTL, guided actions, confirmation slider, failsafe surfaces.
- Joystick: `Joystick/` C++ survives; HID rebinds to `GameController.framework`.
- Video: GStreamer is not Qt and survives. `qmlglsink` → `appsink` → `CVPixelBuffer` → Metal layer.
  RTSP/UDP/RTP, the WHEP/WebRTC path, recording, picture-in-picture.

**Gate (HW):** every guided action verified on PX4 and ArduPilot. Confirmation control cannot be
actuated accidentally. Sub-200 ms glass-to-glass on WHEP. A 30-minute flight with flat memory.

This is the phase where a bug hurts someone. Budget review time, not just build time.

## Phase 6 — Shell · 3 weeks

- `qgc_run()` flips from `QApplication::exec()` on the main thread to a worker-thread
  `QCoreApplication`; `aircast-macos` calls `NSApplicationMain`. No Swift outside that one call
  changes — `runOnQtThread` already marshals correctly.
- QtQuick, QtQml, QtGui, QtLocation, QtMultimedia, QtCharts, QtWidgets, QtPositioning drop from the
  macOS dependency set. QtCore and QtSerialPort stay.
- The QWindowKit integrated-titlebar work is replaced by the real thing and deleted.
- Notarized universal build through the existing release CI; `make release.*` updated.

**Gate:** two weeks of internal flying with the previous release as fallback, then delete the
fallback.

---

**Total ≈ 35 weeks.** Call it 7–9 months for one developer with AI assistance — but see Risk 1.

## Risks

1. **Flight-test throughput is the critical path, not code.** Five of seven gates need real
   airframes. AI compresses the writing, not the flying. Book hardware time before writing code.
2. **Visual mismatch mid-migration.** A native Settings window beside a QML Fly view will not match.
   The per-OS platform theme layer already built is what keeps this coherent — it earns its keep here.
3. **Map editing.** Phase 4 is the one place the Qt version does real work MapKit does not replace.
   If the schedule slips, it slips here.
4. **Android divergence.** Every path the macOS UI needs must also serve Compose. Add to
   `QGCBridgeCore`, never to a head.
5. **Scope drift into de-Qt.** Once SwiftUI works, stripping QObject from the core looks tempting. It
   ends the upstream merge stream permanently and buys a smaller bundle. Don't.

## Not covered

- **iOS.** `src/iOS` is 6 lines today; a SwiftUI codebase makes iOS newly cheap, but it is not scoped
  here.
- **Linux and Windows.** They stay on Qt indefinitely. This is a macOS frontend, not a migration of
  the project.
- **Android.** Stays Qt-cored with the Compose frontend already in flight.
- **UTMSP** (2.7k QML). Niche EU compliance. Decide keep-or-drop at Phase 4; this is the cheapest
  moment to drop it.

---

## Work split across parallel sessions (2026-09-07)

Five sessions work this repo at once. Yesterday they all edited `macos/Sources` and the bridge
together; today one commit sweeps another's staged files and the shared Debug build tree is relinked
under a running probe. This section fixes ownership so the plan above can be executed in parallel
without that. A session states its stream in its first message and stays inside it.

### Streams and ownership

Ownership is by path. A stream edits only what it owns; anything else goes through a request to the
owning stream (a note in that stream's section below, or a one-line issue in the commit message).

| Stream | Scope (plan phase) | Owns | Gate it drives |
|---|---|---|---|
| **A · Shell & shipping** | Phase 1 structure, Phase 6 prep | root `CMakeLists.txt`, `macos/CMakeLists.txt`, `src/main.cc`, `src/Bridge/QGCEntry.h`, `src/Bridge/QGCEmbed.cc`, `macos/Sources/AppShell.swift`, `macos/Sources/QtHostWindow.swift`, `macos/Sources/NativeWindow.swift`, `.github/workflows/aircast-release.yml`, `Makefile` release targets, the **Status** section of this document | QGC as a library target; per-arch Swift libraries lipo'd in the release CI; `aircast-macos` repo scaffold; `make release.*` producing a notarized universal bundle |
| **B · Plan** | Phase 4 | `macos/Sources/{PlanWindow,Mission,MissionMap,MissionItem*,MissionCommandModel,ItemFactModel,MapFraming,TilePyramid,CachedTileOverlay,FenceRally*,TerrainProfileModel}.swift`, `src/Bridge/QGCMapTileC.*`, `src/MissionManager/` | the 200+ waypoint survey planned, uploaded, flown and downloaded byte-identical (HW) |
| **C · Fly & video** | Phase 5 | `macos/Sources/{FlyWindow,Telemetry,TelemetryView,FlightModePositions}.swift`, new `macos/Sources/Video*.swift` and `Joystick*.swift`, a new `src/Bridge/QGCFlyC.*` for guided actions, `src/VideoManager/` appsink path, `src/Joystick/` | every guided action verified on PX4 and ArduPilot; sub-200 ms glass-to-glass on WHEP; 30-minute flight with flat memory (HW) |
| **D · Setup, Analyze, Settings** | Phases 1 (settings), 2, 3 | `macos/Sources/{SettingsWindow,SettingsPages,SettingsStore,AnalyzeWindow,LogDownload,LogEntryModel,Vibration*,VehicleSetupWindow,Parameters,ParameterModel,ParameterRow,Sensors,SensorHealth,SafetySections,VehicleComponent*,ConnectionsSection,LinkConfigModel,Links,Fact}.swift`, `src/Bridge/QGCLinksC.*`, `src/AutoPilotPlugins/`, `src/Settings/`, `src/Comms/` | full parameter tree loads and writes on PX4 and ArduPilot; calibrations complete on real hardware; log download from real hardware with chart values matching the Qt build (HW) |
| **E · Qt frontends & QA** | not a migration phase | everything under `src/FlightDisplay/`, `src/QmlControls/`, `src/UI/`, `src/PlanView/`, `android/`, `test/`, `tools/`, `macos/Tests/`, `macos/Sources/{NativeProbe,Probeable,NativeDebug}.swift`, `src/Bridge/QGCNativeDebugC.h` | Android, Linux and Windows keep shipping on Qt; the probe and unit harnesses stay green; the hardware gates above are run and recorded here |

Shared files, edited by more than one stream, are **append-only and committed within the hour**:
`macos/CMakeLists.txt` source list (A owns the file; B, C, D append their own `.swift` lines),
`src/Bridge/CMakeLists.txt`, `src/Bridge/QGCBridgeCore.*` and `QGCBridgeC.*` (add a new
stream-owned C file instead of growing these), `src/Bridge/module.modulemap`, `test/UnitTestList.cc`,
`test/*/CMakeLists.txt`. Never leave a shared file dirty across a build.

### Corrections to the plan above

- **`src/UI/preferences/` does not exist; the settings QML is `src/UI/AppSettings/`, and it cannot be
  deleted** while Linux, Windows and the Qt-hosted Android path still ship from this tree. Phase 1's
  "delete" becomes "macOS stops loading it". Same for every later "evaporation" of QML: the QML stays
  for the other platforms until they have their own plan.
- **Phase order was not kept.** Streams B, C and D run in parallel from here; A's library target and
  universal build are the only items that gate shipping and therefore come first inside A.
- **Hardware gates have not been run for any phase past 0.** SITL verification does not close a
  phase. E records each hardware gate run in this document with date, firmware and result.

### Session mechanics

- **Own build tree.** `build-<stream>` under the repo, never `build-test` or `build-aircast`; those
  two belong to E for probes and releases. Configure with `-DSDL_CCACHE=OFF` while the brew `ccache`
  is broken. Build the app with `--target AircastQGCApp` or no target; `--target AircastQGC` is
  only the core dylib and leaves the bundle stale. A rebuild into a tree another session is running from
  kills that instance with `Code Signature Invalid`.
- **Own ports and bundle name.** Debug API ports 79A0–79E9 by stream letter (A: 7900–7919, B:
  7920–7939, C: 7940–7959, D: 7960–7979, E: 7980–7999). Probe from a renamed clone
  (`<Stream>Run.app`, binary renamed, `CFBundleExecutable` updated). Never `pkill -f AircastQGC`.
- **Own settings.** A probe that changes a stored preference restores it before exit.
- **Private index commits.** `GIT_INDEX_FILE=$scratch/<stream>.index git read-tree HEAD`, stage only
  owned files, `git commit`, `git push fork main`. Rebase on a moved HEAD, never force-push.
- **Commit small, commit often.** A stream's uncommitted work is invisible to the others and gets
  built into their trees by accident; anything that compiles and passes its suite lands.
- **Tests run from a renamed clone with `--allow-multiple`**, one suite at a time while another
  session's instance holds mock links; a full-suite failure in `VehicleLinkManagerTest` or a
  `SysStatusSensorInfoTest` mismatch is checked alone before it is reported.

## Rust core (decided 2026-09-08, plan revised the same day after investigation)

The end state is no Qt and no C++: **one core in Rust** behind the C ABI the heads already
consume, serving SwiftUI (macOS, iOS), Compose (Android) and a thin QML shim (Windows, Linux).
The full plan, with the four surveys it rests on, lives in the handbook vault:
`Aircast/plans/2026-09-08 Unified Native Heads on a Rust Core.md`. This section is the summary
the streams work from.

Upstream is not a cost: the fork is 1,344 commits behind `mavlink/qgroundcontrol` with a merge
base of 2025-06-18. Risk 5 above is withdrawn.

### What changed after investigation

- **The core serves view-ready state, not a mirror of the QObject tree.** 80 of 103 Swift files
  and 25 of 59 Kotlin files are logic, about 3,400 and 2,000 lines respectively, computing the
  same things twice and disagreeing in seven places (one is a shipped bug: the save-readiness
  enum in `PlanSummaryModel.swift:36-38` is inverted). That layer moves into Rust **first**,
  before any protocol subsystem, so heads become thin renderers early.
- **Rust enters between the heads and the Qt bridge.** The crate is the sole client of
  `QGCBridgeCore`; heads call Rust; Rust forwards roots it does not yet own. Subsystems are then
  strangled underneath in include-graph order.
- **Four things are host-owned by design**: Android USB serial (the Java stays), classic
  Bluetooth, joystick input, iOS accessories. The core exposes push-bytes and push-input entry
  points; no Rust crate solves these portably.
- **GStreamer inside a Rust staticlib on mobile is unproven**; it gets a one-week spike, and
  video stays C++ behind `QGCVideoC.h` if the spike fails.

### Rules in force from now

1. Heads see the C ABI only; no Qt type, enum or `QVariant` shape crosses into Swift or Kotlin.
2. Paths are literals composed from a literal prefix; never discovered at runtime.
3. JSON shapes change only in `QGCBridgeCore` with both heads updated in the same commit.
4. Add to core, never to a head.
5. GStreamer stays; `gstreamer-rs` replaces `VideoManager`, not GStreamer.
6. Logic that appears in both heads, or needs no platform API, belongs in the core.

### R0 — background preparation, no Rust, starts now

- Fix `PlanSummaryModel.swift:36-38`.
- Extend the bridge guard `APPLE AND NOT IOS` (`src/Bridge/CMakeLists.txt:11`) to every platform.
- Revive the ten commented-out suites at `test/UnitTestList.cc:181-303`.
- Rewrite `QGCBridgeCoreTest` to assert convergence rather than 200 ms ticks; extract its cases
  as JSON fixtures; record a golden dump of every inventoried path against SITL and hardware.
- Swift moves from polling timers to `qgc_bridge_watch`; port the Kotlin refcounted registry.
- Android: add the missing `WAKE_LOCK`, `USB_PERMISSION` and storage permissions; stop reading
  the tile SQLite directly and use `QGCMapTileC.h`.
- Wire `cmake/SignMacBundle.cmake` into the release job.
- Write the hardware inventory for gates.

### Phases after macOS Phase 6

| Phase | Replaces | Weeks | Gate |
|---|---|---|---|
| R1 | view-state logic in both heads | 6 | both heads render from Rust; golden dump unchanged |
| R2 | MAVLink framing, Utilities, GPS drivers, Terrain | 4 | mission/terrain/geo fixtures pass; tlog counts match |
| R3 | FactSystem, Settings, ParameterManager | 6 | HW: full tree on PX4 and ArduPilot; existing INI read |
| R4 | Comms, tlog, log replay; iOS enters | 6 | HW: every link type on every platform; one iOS flight |
| R5 | Vehicle hub, ComponentInformation, calibration | 12 | HW: tlog replay parity; guided actions; calibrations |
| R6 | MissionManager, complex items, tile cache | 10 | HW: byte-identical survey; existing cache served |
| R7 | camera, gimbal, ADSB, follow-me, joystick mapping, analyze roots, video if spiked | 6 | HW: SIYI control; joystick; WHEP sub-200 ms |
| R8 | QML shim for Windows/Linux; entry points; DebugApi; QtCore leaves | 8 | Windows and Linux ship from the Rust core |

About 58 developer-weeks. Flight-test time, not code, is the schedule.

### R1 status

- 2026-09-08: `core-rs/` landed. Every `qgc_bridge_*` call passes through Rust on single-arch
  macOS; `view.messages` is the first view-state path; `qgc_bridge_watch_client(client, paths)`
  gives each store its own watch set with the union going to Qt. `QGCCoreCTest` is the C ABI
  suite. Without cargo, or on universal and non-macOS builds, the bridge routes straight to Qt.
- 2026-09-08: `view.plan` — readiness, upload pre-check, file actions, sync state and status line with
  the sentences included; watched through the plan and vehicle properties it derives from.
- 2026-09-08: `view.guidedActions` — the fourteen guided actions with offer hidden/ready/blocked, the
  blocked reason, titles and prompts; derived from vehicle, arming report, mission cursor and checklist.
- 2026-09-08: `view.guidedAltitude` and `view.guidedAltitude(target)` — range in the operator's unit with metres
  alongside, the delta to send, the 0.01 m firmware threshold, and the confirm sentence. View paths may carry
  arguments in parentheses.
- 2026-09-08: `view.guidedTakeoff(target)` and `view.guidedSpeed(target)` — the remaining guided value ranges;
  speed carries the vehicle method name to invoke.
- 2026-09-08: `view.battery` (the QML indicator rule, worst pack first) and `view.preflight` (the checklist by
  airframe with GPS, battery, sensor and sound verdicts).
- 2026-09-08: `view.warnings` — the QML fly-view warnings plus a single prioritised arming blocker sentence.
- 2026-09-08: `view.label(identifier)` — one humanised label rule for fact names without a description.
- 2026-09-08: `view.instruments(group/name, …)` — telemetry chips with label, value and display units resolved
  in the core; the selection stays with the head.
- 2026-09-08: `view.vibration` — axes with fraction and severity band, worst axis, clip counts.
- 2026-09-08: `view.sensors` — SYS_STATUS sensor health as healthy/unhealthy/disabled, failing first.
- 2026-09-08: `view.control(factPath)` — toggle/choice/text/number with options and bounds decoded once;
  view arguments split on top-level commas so nested accessors pass through.
- 2026-09-08: `view.links` and `view.linkForm(type,host,port)` — link list with type, editing mode and status line;
  add-link validation and auto naming.
- 2026-09-08: `view.mapScale(metresAcross)` — the QML scale-bar ladder and labels, feet from the unit setting.
- 2026-09-08: `view.terrainProfile` — profile points, padded altitude band, unknown ground count, texts in the operator's units.
- 2026-09-08: `view.missionKinds(kind?)` and `view.missionSeed(kind,lat,lon)` — the item catalogue and the default seed geometry.
- 2026-09-08: `view.calibration` — APM sensor calibration state, orientation sides, and the routine list with the accel-first gate.
- 2026-09-08: `view.radio` — radio calibration state, channel bars, stick mapping, summary and shortfall.
- 2026-09-08: `view.logs` (log list, sizes, times, button rules) and `view.inspector` (message table, rate choices).
- 2026-09-09: `view.flightModes` — mode picker with summaries, everyday/folded split, confirm-in-flight flag.
- 2026-09-09: `view.settings` and `view.settings(page)` — pages, sections, subsections and decoded controls.
- 2026-09-09: `view.surveyStats(itemIndex)` — shots, interval, area, distance, footprint and the trigger-interval warning.
- 2026-09-09: `view.fences` and `view.polygon(path, ring|line)` — fence and rally descriptions, vertex editing rules and midpoints.
- 2026-09-09: `view.setup` and `view.setup(page)` — setup overview, readiness, page groups and per-firmware parameter sections.

### Stream F · Core

Owns `core-rs/`, the generated `QGCBridgeC.h`, the golden dump and fixtures, the router in
`QGCBridgeCore.cc`, and this section. B, C and D consume Rust view-state paths as F publishes
them and delete their model files in the same commit. E records every hardware gate.

### Open decisions

When R1 starts (after Phase 6, or from a cut-off for new screens); the QML shim versus a fifth
native head for Windows and Linux; Bluetooth on macOS; confirming `UTMSP` and `Viewer3D` are
dropped. Recommendations are in the vault plan.
