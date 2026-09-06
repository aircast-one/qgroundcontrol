# Native macOS Migration — Implementation Plan

> **The phase plan below is not yet safe to execute.** A file-level inventory
> (`tools/macos/qml-inventory.py`) contradicts the sizing it was built on. Read
> "What the inventory found" before scheduling any of it.

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

Three consequences:

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

CMake cannot mix Swift and C++ in one target, so making `main.swift` the literal entry point needs
QGC built as a library rather than an executable — the Phase 1 restructure. The trampoline delivers
the same ownership without it, so that restructure is now only a packaging decision.

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

**Universal builds are blocked.** `swiftc` accepts one `-target`, so `CMAKE_OSX_ARCHITECTURES`
holding both arm64 and x86_64 cannot produce one Swift library. `macos/CMakeLists.txt` fails loudly
on that rather than silently emitting a single-arch library, so the release CI's universal build
needs one Swift library per architecture `lipo`'d together before it can ship. Phase 1 work.

Open at Phase 1: the Swift moves to an `aircast-macos` repo once qgroundcontrol exposes QGC as a
library target, mirroring how `aircast-android` consumes `AircastQGC.aar`. The C bridge header stays
here permanently — it is the macOS `QGCBridge.java`.

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
