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

### Multi-vehicle: the head identifies, it does not command (2026-09-11)

`FlyViewTopRightPanel.qml` is entirely multi-vehicle -- a vehicle list, Select All, Deselect All,
and multi-vehicle Arm, Disarm, Start and Pause. It appeared in no phase of this plan, in no view
the core serves, and in neither native head. That was an omission rather than a decision, and this
is the decision.

**The multi-vehicle commands are not ported.** Commanding several aircraft to arm at once is the
highest-consequence action in the application. Nothing in this environment can exercise one, so it
would ship having never run; the standing rule here is that a gate which fails closed may ship
unverified but a capability that fails by *doing* something may not, and this is the second kind
six times over. No requirement for it is recorded in the repository or the handbook.

**Identifying the active aircraft is not optional, and today the head does not.** With two vehicles
connected the native head reads `vehicle.*`, which is whichever one is active, and draws its
telemetry with nothing naming it and no way to change it. `vehicles.activeVehicleAvailable` is a
boolean and is the only thing either head knows about how many there are. So an operator with two
aircraft up cannot tell which one an arm or an RTL reaches. QML at least shows the list. Declining
to support multi-vehicle is defensible; supporting it silently and wrongly is not, and that is
what the head does now.

**The core moves first.** No `view.*` carries a vehicle list. What a head needs is the id, the name,
which one is active, which link each arrived on, and a way to make one active -- nothing that
commands, and no aggregate action.

**Phase 5 gate, added:** whenever more than one vehicle is connected, the head names the one it is
commanding and can switch between them. No multi-vehicle command is ported.

**Phase 6 may then delete the multi-vehicle half of `FlyViewTopRightPanel`** as a recorded
non-port rather than an oversight. `MultiVehicleManager` itself stays -- it is on the core's
critical path and is what would answer the list above.

## Phase 6 — Shell · 3 weeks

- `qgc_run()` flips from `QApplication::exec()` on the main thread to a worker-thread
  `QCoreApplication`; `aircast-macos` calls `NSApplicationMain`. No Swift outside that one call
  changes — `runOnQtThread` already marshals correctly.
- QtQuick, QtQml, QtGui, QtLocation, QtMultimedia, QtCharts, QtWidgets, QtPositioning drop from the
  macOS dependency set. QtCore and QtSerialPort stay.
- The QWindowKit integrated-titlebar work is replaced by the real thing and deleted.
- Notarized universal build through the existing release CI; `make release.*` updated.

### A guard that never opens looks exactly like a careful guard (2026-09-11)

Both sessions audited their own rules this morning, found nothing by reading, and then found
eleven and six by mutation. The method is cheap and the reason it works is worth stating, because
neither of us reached it by thinking about it.

**Replace each rule's body with a constant and see whether anything fails.** Not invert it —
invert is the weaker probe, and the core session ran it first and found nothing. An inverted rule
still fails a test that asserts only one direction, so a one-sided rule scores as covered. A
constant asks the real question: is there any test that distinguishes this rule from `true`, and
any that distinguishes it from `false`.

**Run both directions.** The macOS head ran only the `false` direction and found `offersShutter`,
which guards taking a photo: every assertion said the shutter was *not* offered, so a head whose
shutter never worked passed. The mirror found `GuidedOffer.blocked`, which gates arm, takeoff,
return and land: every assertion said an action *was* blocked, so a head where no guided command
could ever be pressed passed. The core session's mirror found `save` and `clearMission` only ever
unavailable, and a calibration panel's `busy` only ever false — a screen that exists to show a
routine running, never shown running.

**Both sides' findings were on actuators, and that is not coincidence.** The assertions people
write about a guard are the ones where it refuses; nobody writes "and this really can fire" with
the same instinct. So the safe-seeming direction is the one that goes untested, and the failure it
hides — a control that can never be used — is invisible in a build with nothing to control.

**One-sided is more dangerous than uncovered.** An uncovered rule has no tests to reassure anyone.
A one-sided one has four.

**Check the harness before publishing its numbers.** The core session's restored files with a
backup's mtime, so the build fingerprint reused the mutant and the following run measured the
previous mutation. The failure is asymmetric and worth remembering: a mutant is always written
fresh, so **a survivor is always real, but a kill may be spurious.**

Two other rules fall out of this. A rule in a file the checks cannot compile is unpinned however
carefully it was written — `swift-checks.sh` builds 68 of 112 macOS sources, and the window files
are not among them. And, narrower than "move every rule": **a sentence making a claim about the
aircraft should be somewhere a test can read it back.** Two of those were found and moved — the
Upload button explaining itself with the plan's readiness sentence, and the Fence tab telling an
operator their firmware lacks a feature.

### A head that reads a view once is the recurring defect, and how to find it (2026-09-11)

Six defects in the Plan window in one session, and four of them were one shape: the head reads a
view once, what it read is not final, and nothing reads again. The plan summary sat an edit behind
and showed the distance of the plan before the last waypoint. The terrain panel said the mission
cleared ground it flew into. A survey would not say how many photos it takes or how far it flies. A
survey was missing from the map entirely. Only **which property lands late** varied.

**The Plan window's store never polls, by design, and that is why the class lives there.** The Fly
view's copy re-reads twice a second, which hides the same mistake rather than avoiding it.

**Fence and rally were measured and are clean**, which is what makes the class precise: a fence is
computed the moment its shape exists, with no terrain server to wait for and no transects to lay
out. So the question when wiring any panel is not "does this window poll" but **"what arrives after
the first read"**. Terrain heights and their collision verdicts land separately, one after the
other. A survey's area and shot interval land with its polygon; its shot count, its flown distance
and its own entry coordinate land with its transects.

Each was fixed the same way: watch the controller properties that land late through the bridge's
push channel, and re-read the core's view, which stays the authority. Coalesce the re-read — two
signals per item all arrive in one turn, and re-reading on each measured 7.1 ms a time for a
three-item plan against a gate of a two-hundred-waypoint survey.

**The other two defects were one list answering two questions.** The map drew a single polyline
through every placed item, so a region of interest — which earns a marker and which the aircraft
never flies to — put a dogleg in the planned route; and an item after a return to launch, which is
uploaded and never reached, got a leg drawn across the map to it. `flownLeg` and `endsRoute` answer
those separately, and neither field alone would have been enough: leaving an item out of a route
and ending a route are different questions.

**`tools/macos/head-vs-core.py` is what finds these.** It builds a plan through the debug probe —
takeoff, a waypoint over real terrain, a region of interest, a survey, a landing, and a waypoint
after it — then compares every fact the window draws against the core's answer for the same fact on
a live app, and fails on any disagreement. It found the survey's missing position by itself and the
core's null altitudes on its first run. Three rules earned the hard way:

- **It compares what the operator sees**, string against string, so neither side parses the other's
  back into a number.
- **It reports a disagreement without attributing it.** The core has been the wrong half three
  times.
- **Its guard asks the core whether a run is worth reporting, never the head.** The first version
  asked the head, and when the head was broken on purpose to prove the tool caught it, the guard
  read the defect as a trivial plan and went quiet. A guard that consults the thing under test goes
  silent exactly when it is needed.

### What Phase 6 deletes, swept (2026-09-11)

Two QML extensibility points were on the open list -- `instrumentQmlFile2` and `Viewer3D` -- with a
note that they were a category rather than a pair. They are not the same kind of thing, and the
category turns out to be almost empty.

**The plugin surface is unused by this fork, so deleting it costs this product nothing.**
`QGCCorePlugin` exposes the custom-build hooks that carry QML: `analyzePages`, `toolBarIndicators`,
`customMapItems`, `brandImageIndoor`/`Outdoor`, `paletteOverride`, `createQmlApplicationEngine`,
`createRootWindow` and `factValueGridCreateDefaultSettings`. The only thing overriding any of them
is `custom-example/`, and that is not built here: `QGC_CUSTOM_BUILD` turns on only if a `custom/`
directory exists and there is none. This fork changes QGC directly rather than through a plugin.
Downstream custom builds would break, which is upstream's problem and not this one's.

**`instrumentQmlFile2` is not a plugin hook at all, and it is the awkward one.** It is a settings
fact whose *value* is a QML file path -- three of them, the integrated, horizontal and large
vertical compass-and-attitude widgets, as `qrc:/qml/...` URLs. It is the only fact in the entire
settings tree whose value is a qrc path; nothing else in `src/Settings/*.json` mentions one. So
Phase 6 deletes the three files and leaves a persisted user setting naming things that no longer
exist, in a QSettings space shared with the QML app -- the same trap as the video source fact.
This needs a migration, not a deletion.

And the native head has no equivalent of the choice. `InstrumentSelection` is the *values grid*
(altitude, ground speed, climb rate and so on); which compass-and-attitude widget to draw is a
separate control that this head does not offer at all. Porting it means offering the choice by
name, never by file.

**`Viewer3D` is a subsystem, not an extensibility point.** `src/Viewer3D/` is an OSM parser and
parser thread, city-map geometry, an earcut triangulation header, shaders, a sample map and a
manager, with its own settings group carrying `enabled`, `osmFilePath` and building dimensions.
`FlyView.qml` imports it and instantiates it. It defaults to **off**. Nothing in either native head
touches it. It needs a decision of the same shape multi-vehicle just got -- port it, or record the
non-port -- and it should not be discovered during the Phase 6 deletion.

**So the category is: one unused plugin surface, one settings migration, one undecided subsystem.**
There is no third class hiding.

### Phase 6 carries a cost the bullet above hides (2026-09-10)

"`runOnQtThread` already marshals correctly" is true and is not the whole question. `onQtThread`
in `src/Bridge/QGCBridgeC.cc:22` runs the body **inline** when the caller is already on Qt's
thread, and falls back to `Qt::BlockingQueuedConnection` when it is not. Today Qt owns the macOS
main thread, SwiftUI calls from that same thread, and every bridge read this head makes takes the
inline path. The marshalling cost is currently zero — by construction, not by care.

Phase 6 is precisely the change that ends that. The moment `QApplication::exec()` moves to a
worker thread, every `Bridge.group` call in `macos/Sources` becomes a cross-thread blocking post.

The Android session is on the far side of this and measured it. Their first three readings each
had a mechanism that fully explained a ~930 ms bridge `get`, and all three were wrong. The control
that settled it was reading **the same path twice** in one view entry:

| | |
|---|---|
| `get plan.missionController.visualItems` at tab entry | **864 ms** |
| the same path, same plan, same process, 4 s later | **5 ms** |

Back-to-back reads drain progressively — 883, 619, 18, 4 ms — which is the shape of a queue
emptying, not of work being done. Opening their Plan tab makes the Qt thread busy for about a
second, and anything posted with `BlockingQueuedConnection` in that window waits for the loop.
What occupies the thread for that second is still open on their side; it is triggered by opening
Plan and is independent of plan contents.

Marshalling itself is cheap: with a 1 ms profiling threshold, `invoke vehicle.clearRcChannelOverrides`
cost **8 ms** off the Qt thread in the same session. And `getFields(path, "*")` changed nothing
(896–1002 ms), which also settles what `getFields` is for — `objectJson` reads every property
before checking whether it was requested, so it saves neither CPU nor latency, only bytes.

**So Phase 6 does not impose a per-read tax.** The earlier framing in this document — every read
becoming a blocking post costing most of a second — was alarming and wrong. What the flip actually
does is expose this head to whatever else occupies the Qt thread: if macOS has an equivalent of
"opening a view kicks off a second of Qt work", the first reads after it queue behind that, and
only then. That is bounded and plannable rather than a blanket latency.

The three-week budget survives this. What does not survive is discovering it late — the exposure
does not exist until the flip, so the first Phase 6 build is where it appears, and the way to
measure it is to read the same path twice under different conditions rather than once.

### Link creation needs a bridge change; link editing does not (2026-09-10)

The Android session found a boundary in the bridge that bounds both heads. QML can hold a C++
pointer in a JavaScript variable between calls — `LinkSettings.qml:187` keeps the result of
`startConfigurationEditing(object)` across several user interactions and commits at line 287.
A path-based head has nowhere to put that handle: `invokePath` returns a `QObject *` as
`objectJson`, a snapshot of values with no path, and `@path` arguments only address objects that
are already reachable. So a "create detached → configure → commit" flow cannot be driven from
SwiftUI, however many of its methods are `Q_INVOKABLE`.

Their sweep found 22 pointer-returning `Q_INVOKABLE` declarations and only two that matter, both on
`LinkManager`: `createConfiguration` (registered only when `endCreateConfiguration` calls
`addConfiguration`) and `startConfigurationEditing` (a copy that `endConfigurationEditing` copies
back and destroys). Everything else — `getFact`, `getParameter`, `getVehicleById`, every
`MissionController::insert*` — returns something already addressable by path, so a head re-reads
it and never needs the handle.

**The two are not equally blocked, and this document should not record them as one thing.**
Creating a configuration genuinely needs the handle, because the object does not exist at any path
until it is registered. Editing an existing one does not: a registered configuration is addressable
at `links.linkConfigurations.N`, and the fields an operator changes are `Q_PROPERTY` with `WRITE`
setters — `name` on `LinkConfiguration`, `localPort` on `UDPConfiguration`, `baud`, `portName`,
`dataBits`, `stopBits`, `parity`, `flowControl` on `SerialConfiguration`. A head can write those by
path today. What it cannot get that way is QML's cancel semantics, because
`startConfigurationEditing` hands back a duplicate so that Cancel can discard it; a head editing in
place either commits on change or snapshots the values itself and writes them back.

**Unverified, and deliberately so.** Writing any of those properties changes the link configuration
in the settings file this rig shares with the QML app, and creating or removing a configuration is
forbidden here. The reasoning above is from the property declarations and the bridge's `@path`
handling, not from a write that was performed.

What this head has today is connect, disconnect and remove, all through `@path` to a configuration
that is already registered — which is exactly the split the constraint predicts. It also carries
`adding` and `editingIndex` state, and decodes the core's per-link `editing` discriminator, for a
form that was never built. That is pending work blocked on an API decision, not a missing feature
somebody forgot.

**Reachability needs two things, not one — but not for the reason first recorded here.** A Qt API
is reachable when every object it needs has a path; that is necessary and not sufficient, because
the parameter *types* must also survive the bridge. The mechanism recorded in this document on
2026-09-10 was wrong and the core session measured it down:

The original reading was that `QGCBridgeCore.cc:669` hands `QMetaObject::invoke` the canonical
metatype spelling while moc records the declared one, so the two strings differ and the call is
refused. **They do not differ.** For `quint16`, `parameterTypeName()` and
`parameterMetaType().name()` both return `"ushort"` — moc normalises Qt's own typedefs at
generation time, so the spelling seen in `moc_UDPLink.cpp` does not survive into the metaobject.
Both forms were invoked and both succeed. `UDPLink::addHost` and `removeHost` are **not** affected,
and the "class of defect" recorded here did not exist in the shape described.

What actually broke `MAVLinkInspectorController::setMessageInterval(int32_t)` is one line earlier.
`int32_t` is a `<cstdint>` name moc does not normalise and Qt has never registered, so
`parameterMetaType(arg)` is invalid and `values[arg].convert(...)` fails *before* `invoke` is
reached. Handing `invoke` the written name would not have helped. Changing the signature to `int`
in `53b9a84cc` was the correct fix, not a workaround for a deeper bug.

**The surviving rule is narrower and still worth having:** a `Q_INVOKABLE` whose parameter type Qt
has not registered is refused with `ok: false` and no reason, before the call is attempted. Qt's own
typedefs are registered and safe; `<cstdint>` names and other unregistered spellings are not. A test
in the bridge suite now pins the two spellings as equal for this case, so if Qt ever stops
normalising, that goes red and the class becomes real.

Two lessons kept, because the correction cost more than the finding. The proposed one-line fix was
applied by the core and was worse than a no-op: `parameterTypeName` returns a `QByteArray` by value,
so `.constData()` on the temporary dangles, and every argument became "too few arguments" across
three unrelated tests. It was only the debug print of the two spellings that revealed the change had
no purpose at all — without it, a lifetime fix for a self-inflicted bug would have landed on top of
an unnecessary change and been reported as closing a class. **A reading of real code that explains
the symptom is still a hypothesis, and the cheapest way to settle one is usually to print the two
values it claims differ.**

### The Plan view, control by control against QML (2026-09-10)

Walked `PlanView.qml`'s menus against this head's. Most of what looked missing was drawn somewhere
else, which is the trap this comparison exists to avoid: **Download from Vehicle** is a toolbar
button here (`PlanWindow.swift:998`, gated on `offersDownload`) rather than a file-menu entry, and
the map-centre items — Launch, Vehicle, My Location, Coordinates — are all present through
`CentreMenu`. Two real results:

**Save was missing and is now built** (`c54ac28f8`). QML has had it since `PlanView.qml:697`.

**Clear Mission is missing and is deliberately not built.** QML's entry calls
`removeAllFromVehicle()` behind a confirmation reading "remove all mission items and clear the
mission from the vehicle". This head's Clear calls `plan.removeAll`, which `PlanMasterController.h`
documents as *removes all from controller only*. So an operator on the native head can clear the
plan in front of them and cannot clear the mission stored on the aircraft.

It is not built because it writes to the vehicle and this rig cannot exercise it. Adding an
untested destructive control to a ground station is a worse outcome than the gap: the fence and
read-only fixes shipped unverified because they were *gates* that fail closed, and this is a
*capability* that erases state on an aircraft. It goes on the list until someone with a vehicle can
run it.

### The reflection surface the view registry cannot guard (2026-09-10)

Every `view.*` path this head reads is in the core's registered `VIEWS`, so that half is checkable
by construction. This is the other half: **50 reflection read sites across 8 roots** — `plan` (18),
`vehicle` (19), `settings` (5), `links` (3), `mavlinkInspector` (2), and one each of `geoTag`,
`units`, `positionManager`. No registry covers them; they resolve against live Qt objects by name.

They are not equally exposed, and the tiers matter more than the count.

**Tier 1, the root — detectable.** An invented root answers `{"kind":"null"}`.
`tools/macos/bridge-paths.py` checks these and exits non-zero.

**Tier 2, a static property below the root — undetectable from here.**
`plan.geoFenceController.breachReturnAltitude`, `settings.appSettings.offlineEditingCruiseSpeed`,
`vehicle.orbitMapCircle.radius` and the rest answer `{"kind":"value","value":null}` when
misspelled, which is byte-identical to a property that is genuinely null. Renaming any of these
properties in QGC breaks this head silently. The consumer cannot close this; it needs the bridge to
distinguish "no such property" from "the value is null" — a `found: false` alongside the value
rather than a new `kind`, since it is orthogonal to what the value is.

**Tier 3, an interpolated segment — not even enumerable.** Seven sites build the path from data at
runtime: `plan.missionController.visualItems.\(item.index).\(property)`,
`...\(item.index).\(list)`, `vehicle.\(group)`,
`mavlinkInspector.activeSystem.messages.\(current.index).fields`. A static sweep cannot list what
those become. **And the interpolated segment is usually the core's own string** — `\(property)` and
`\(list)` come from the item and polygon models the core serves. So for tier 3 the guard is the
core's fixture: if the core names a property the C++ does not have, this head interpolates it into a
path that answers null and every decoder takes its default. The head cannot check that; the
producer's own tests are the only thing standing there.

Worth stating plainly because it inverts the usual direction: for tier 3 a defect in the *core's*
data becomes a silent wrong value in the *head*, with no error anywhere in between.

### The 3D Viewer settings page outlives the thing it configures (2026-09-10)

Resolved the question this document left open. **This head does offer a 3D Viewer settings page** —
one of fifteen, confirmed by opening the Settings window and reading the probe rather than guessing
from a closed one. It carries four editable controls: "Enable the 3D viewer" (a toggle, currently
false), an OSM file path, a building level height and an altitude bias. All four are writable and
this head reads none of them.

That is not a dead control **today**. The settings file is shared with the QML app, so enabling the
viewer here enables it there. It becomes dead at Phase 6, and that is the point worth recording.

Checked whether the problem is general by asking, for all 22 settings groups, which have facts no
head reads. Eleven came back — and **all eleven are false alarms**: `AutoConnect`, `RTK`,
`GimbalController`, `APMMavlinkStreamRate` and the rest are consumed by C++ that survives Phase 6,
so an operator flipping them changes real behaviour whether or not a head reads the value. Consumed
another way, again.

`Viewer3D` is the only genuine case. Every consumer of its four facts is either the settings
plumbing that merely stores them (`Viewer3DSettings`, `SettingsManager`) or `src/Viewer3D/` and its
QML — and `src/Viewer3D/CMakeLists.txt` links `Qt6::Quick3D`, which goes when QtQuick does.

**So Phase 6 has to decide between two things it currently has neither of.** Either the 3D viewer is
rebuilt natively — SceneKit or RealityKit, which is a rewrite of a feature and not a port of one —
or the viewer and its settings group are deleted together. What must not happen is the viewer going
and the page staying, which would leave an operator four controls for a feature that no longer
exists, with the enable toggle sitting at the top of them.

Not urgent: `enabled` defaults to false, so nobody meets this unless they go looking.

#### Decided: the 3D viewer is deleted with its settings group, not rebuilt (2026-09-11)

Measured before deciding. `src/Viewer3D/` is 4,462 lines, and the bulk of it is QML: a scene, a
progress bar, waypoint and line models, and a DJI F450 modelled part by part — `DroneModel_BLDC_1`,
`DroneModel_propeller22_2` and siblings, one QML file each. `Viewer3D.SettingsGroup.json` defaults
`enabled` to false and defaults `osmFilePath` to the literal string "Please select an OSM file", so
the feature does nothing at all until an operator sources an OpenStreetMap extract themselves and
points the setting at it. It is also the only thing in the tree that links `Qt6::Quick3D`.

So the choice is not "port one view". Keeping it keeps a whole Qt module, and its QML content, in
the macOS dependency set that Phase 6 exists to empty — for a feature that is off by default, inert
until a manual download, and that nothing in this repository or the handbook records a requirement
for.

**What survives is the half worth keeping.** `OsmParser`, `OsmParserThread`, `CityMapGeometry` and
`earcut.hpp` are C++ and have nothing to do with Qt's scene graph. If a 3D view is ever wanted back
it returns as a native one — SceneKit against that same parser — which is a project with its own
justification rather than a line item inside a port. Deleting the QML does not burn that bridge; it
only declines to carry a QML scene across a boundary built to leave QML behind.

**The viewer and the settings group go together**, which is the failure the section above names:
an operator left with four controls and an enable toggle for a feature that is not there.

This is a product call as much as an engineering one, and it is recorded here so it can be
overturned by someone who knows a customer uses it. What it must not do is stay undecided until
the deletion discovers it.

### The capability-versus-readiness sweep, and how to run it properly (2026-09-10)

Both heads hit the same shape four times: **the head reached for the predicate whose name sounded
like the question and got the weaker one, while the core computed the stronger answer in the same
view node.** `hasModes` where `canChangeMode` belonged (fixed in `1a457051b`), `offer != "hidden"`
where `offer == "ready"` belonged (`276cf417f`), and two on the Android head — `blocked` where
`ready` belonged, and `setupComplete` where the core's `ready` belonged.

Swept for more. **Nothing further found**, and the method is worth recording because the first
attempt ran the wrong way round.

Listing every readiness-shaped key the core *emits* and asking whether this head decodes it produced
six misses and all six were false positives. `canArm`, `canTakeoff` and `canStartMission` are C++
properties on `healthAndArmingCheckReport` that `guided.rs` *reads* to compute the per-action
`offer` this head already uses. `supportsTerrainFrame` is used inside `altitudemodes.rs` to set each
mode's `enabled` and `reason`, which is the combined answer this head reads. `canBeSet` and
`gcsFixValid` live in `hub.snapshot()` and `remote_snapshot()`, which are not view paths at all.
**A key the core mentions is not a key the core offers a head.**

The direction that matters is the inverse: list the capability-shaped keys *this head's models
decode* and ask whether a `can*`/`ready` sibling exists **in the same view node**. Sibling matching
must be scoped to the node — matching across files makes `available` collide with everything and
buries the signal.

Scoped that way, `video.rs` is the only node carrying both, and it is now correct:
`canChangeMode` gates the mode write, `hasZoom` is right because the core exposes no readiness
counterpart for zoom, and `offersShutter`/`offersRecord` derive from `canPhoto`/`canRecord` — which
the core deliberately leaves true while recording, since that control is what stops it.

Two decodes take the permissive default and are safe only because something else pins them:
`pausesFirst` (absent would skip pausing a flying vehicle before an upload) and `hasCollision`
(absent would hide a terrain-collision warning). Both are in the required-keys contract assertion,
so a rename or removal reddens the suite rather than failing silently. **That assertion is what
makes a `?? false` on a safety field survivable, and it is worth knowing that is what is holding
them.**

### The stored-flag latch family is exhausted for this head (2026-09-10)

`_flying` and `_landing` are handled in the core (`flystate.rs` lets armed decide the line).
`dirty` produced two defects and is covered above. Of the rest, `completes` and `operatorIDValid`
are read only by the core, `_lastCurrentIndex` by neither, and the three PlanView insert flags are
not read here at all — which was luck rather than care, since the Android session found them
returning member defaults forever.

That leaves one member reaching this head, and it is not a latch:
`LinkManager::mavlinkSupportForwardingEnabled()` is `mavlinkForwardingSupportLink() != nullptr`,
recomputed on every read by scanning the live link list, so it cannot go stale. The name overstates
what the value knows — it reports that a forwarding link object exists, not that anything is being
forwarded — but presence in `_rgLinks` tracks creation and removal, so the gap is transient rather
than latched. **No defect. Recorded so the sweep is not run a third time.**

**Consequence for Phase 5 and 6: link creation exists only in QML today, and so does cancel-safe
link editing.** Deleting the QML on the assumption that the native head covers what it covered
would remove both. This is the second such item; the first was noted by the Android session on
their own screen.

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
- 2026-09-09: `view.video` (stream state, camera slots, summary) and `view.camera` (MAVLink camera control state and sentences).
- 2026-09-09: R1 view-state layer served end to end; `test/Bridge/fixtures/view-shapes.json` is the recorded contract
  for all 37 view paths and `QGCCoreCTest` diffs it (re-record with `QGC_RECORD_VIEW_CONTRACT=1`).
- 2026-09-09: Android builds link the Rust core via cargo-ndk when it is installed (arm64-v8a verified); CI
  still routes straight to Qt until Rust and cargo-ndk are added to the Android job.
- 2026-09-09: R2 begins — the `mavlink` crate (ArduPilot dialect) is in the core; `view.tlog(path)` decodes a
  telemetry log frame by frame, checked against an independent frame count on the sample log.
- 2026-09-09: `view.planFile(path)` reads .plan files in the core (checked against the C++ loader); watch
  lists keep argument commas whole.
- 2026-09-09: `view.waypointsFile(path)` reads QGC WPL 110/120 in the core, checked against the C++ importer.
- 2026-09-09: `view.kmlFile(path)` reads KML polygons and polylines in the core with the map-polygon test's fixtures.
- 2026-09-09: `view.shapeFile(path)` reads .shp polygons and polylines in the core with the SHPFileHelper rules
  (prj projection, UTM, five-metre vertex filters).
- 2026-09-09: `view.geoToNed/nedToGeo/geoToUtm/utmToGeo` — QGCGeo's conversions in the core, pinned to GeoTest.
- 2026-09-09: `view.terrainTile(path[,lat,lon])` — the cached terrain tile format and lookup in the core.
- 2026-09-09: MAVLink 2 signing key derivation and timestamps in the core (SigningTest parity); not exposed over the bridge.
- 2026-09-09: STATUSTEXT chunk reassembly, counting and formatting in the core (StatusTextHandlerTest parity).
- 2026-09-09: SYS_STATUS sensor decoding in the core (SysStatusSensorInfoTest parity).
- 2026-09-09: GPS fact-group decoding from MAVLink in the core; the tlog reader visits messages.
- 2026-09-09: battery fact-group decoding in the core (BATTERY_STATUS by id, HIGH_LATENCY2 percent).
- 2026-09-09: vehicle fact-group decoding in the core (attitude, quaternion, altitude, HUD, nav controller, rangefinder).
- 2026-09-09: wind, temperature, distance sensor, local position and estimator status decoding in the core.
- 2026-09-09: `.plan` writer and `.waypoints` to `.plan` conversion in the core (`view.planFromWaypoints`), loaded back by Qt in the C ABI suite.
- 2026-09-09: `view.inspector` messages carry `title` (name, or name with comp id when the name repeats across components).
- 2026-09-09: QGC WPL waypoint writer in the core with a read-back round trip.
- 2026-09-09: mission command metadata (18 MavCmdInfo files) compiled into the core and collapsed like the command tree.
- 2026-09-09: legacy `.mission` reader (v1 and v2) served as `view.missionFile(path)` with the plan rewritten; Qt agrees on OldFileFormat.mission.
- 2026-09-09: gzip, xz and zip decompression in the core (pure Rust: flate2, lzma-rs, zip with deflate only), verified on the Qt manifest fixtures.
- 2026-09-09: review pass over the night's core slices, ten findings fixed (metadata defaults and hidden params, HUD offset relatch, attitude source filter, HIGH_LATENCY arms, WPL 120 jump targets, legacy mission parity, inflate cap).
- 2026-09-09: tlog frame, heartbeat and drop counts proven equal to the C parser in the C ABI suite (R2 gate clause).
- 2026-09-09: `view.setup(page)` lists only parameters the vehicle has (the default fact is dropped); APM mock covers the Safety page in the suite.
- 2026-09-09: fact JSON skips the default-value read when none is available (no more per-read warnings on APM parameters).
- 2026-09-09: RTCM correction fragmenter in the core (GPS_RTCM_DATA flags and sequence as RTCMMavlink).
- 2026-09-09: R3 starts: fact metadata loader in the core, all 38 SettingsGroup/FactMetaData files load with typed enums, defaults and bounds.
- 2026-09-09: settings-group registry in the core (22 groups, metadata files compiled in, QSettings key layout).
- 2026-09-09: QSettings INI reader in the core (sections, escapes, lists, @Invalid/@ByteArray/@Variant).
- 2026-09-09: parameter download/write state machine in the core (pure events in, actions out, C++ retry budgets).
- 2026-09-09: PX4 parameter XML metadata loader in the core over the bundled file (2,600+ parameters).
- 2026-09-09: ArduPilot apm.pdef.xml loader in the core with the plugin's lookup rules (Copter 4.6 fixture).
- 2026-09-09: COMPONENT_INFORMATION parameters.json loader in the core with {n} indexed-name resolution.
- 2026-09-09: PX4 metadata cache selection and storage rules in the core (pure decisions; host copies files).
- 2026-09-09: R4 starts: link configuration codec in the core over the QSettings keys, with the build-dependent type table.
- 2026-09-09: tlog recording and replay pacing in the core (worker rules, time-based seek).
- 2026-09-09: USB board classification in the core over the bundled USBBoardInfo table (ids, then description and manufacturer patterns).
- 2026-09-09: autoconnect decisions in the core (composite filter, wait list, board gating, baud choice, RTK port tracking).
- 2026-09-09: guided speed offer checks the firmware parameter live (was a flag latched once at parametersReady).
- 2026-09-09: review pass over the R3/R4 core slices, sixteen findings fixed (QSettings unescaping, metadata guards, tlog resync, replay rewind, parameter machine parity).
- 2026-09-09: link registry in the core (owner core/host, per-link framing and counters), first slice of the R4 transport seam.
- 2026-09-09: `view.fences` rally points carry altitude, altitudeUnits and altitudePath.
- 2026-09-09: core-owned UDP link (std::net + socket2: shared bind, multicast join, session targets) with loopback tests.
- 2026-09-09: core-owned TCP link (connect timeout, reader thread, disconnect events) with loopback tests.
- 2026-09-09: link ABI (qgc_core_link_open/close/write, host_link_open/bytes/closed, set_link_writer) with `view.transports`; C ABI suite exchanges a heartbeat over a core UDP link.
- 2026-09-09: contract recorder adds a populated-plan state; view.fences circles, polygons and rally points carry element shapes.
- 2026-09-09: core serial link (serialport crate, non-Android); `view.transports` lists Qt links with owner "qt"; core refuses a UDP port a Qt link serves (Android found SO_REUSEPORT load-balances).
- 2026-09-09: contract recorder adds a saved TCP link; `view.links` configured carries an element shape.
- 2026-09-09: transport review fixes (no lock across I/O, reaping, state events, resync by length) and `view.guidedAltitude(target,pause)`.
- 2026-09-09: `CoreLink` lets LinkManager build UDP/TCP/serial links on the core behind `QGC_CORE_LINKS=1`; a vehicle comes up over a core-backed link in the suite.
- 2026-09-09: CoreLink sources actually land (dbb35f05c missed src/Comms); `settings.appSettings.coreLinks` switch; view.video titles from cameraName; connect state machine ported to the core.
- 2026-09-09: command-ack retry machine and request-message protocol in the core (R5).
- 2026-09-09: MAVLink FTP codec and download state machine in the core (R5).
- 2026-09-09: MAVLink FTP directory listing in the core.
- 2026-09-09: standard flight mode collection (AVAILABLE_MODES) in the core.
- 2026-09-09: remote-ID rules in the core (EU operator check, arm status, message set).
- 2026-09-09: ULog stream processor (LOGGING_DATA sequence, drops, reassembly) in the core.
- 2026-09-09: vehicle hub in the core (`view.coreVehicle`, frames from core links to the fact groups, 3.5 s expiry); recorder adds mission items so terrainProfile points record.
- 2026-09-09: R5 gate test replays mav.tlog through both models and compares; vehicle altitude/position fallbacks ported.
- 2026-09-09: flight-mode name tables in the core; gate compares mode text, armed and battery.
- 2026-09-09: core links opened by LinkManager bypass the shared-port guard (Android found the guard refusing its own link).
- 2026-09-09: R5 review pass, ten findings fixed (reap outside the lock, FTP bounds, hub heartbeat rules, retry cadence).
- 2026-09-09: guided action planners in the core (takeoff, goto, altitude, pause, RTL, land, speed) per firmware.
- 2026-09-09: survey and terrain texts format measures the way the Qt head does (one decimal below a hundred, none above, ^2 as the superscript); heads drop their own formatters.
- 2026-09-09: guided actions execute in the core: `qgc_core_guided({"vehicle","action",...})` runs the planned steps on the vehicle's core link (mode change retried three times over 1.3 s each, arming once over 1.5 s, as the Qt head does); `view.coreGuided(id)` shows state, step, label, errors.
- 2026-09-09: `view.instruments(...)` depends on and reads the selected facts (`vehicle.gps.count`, `vehicle.batteries.0.voltage`) instead of whole groups; a view's dependencies may now derive from its arguments. Level-horizon text is QGC's.
- 2026-09-09: `view.detections` replaces DetectionOverlayVideo.qml's logic: host and camera from the RTSP URL setting, the agent's SSE detections stream followed in the core, boxes normalised with `confidence`, stale after a second, pushed to the head on every frame.
- 2026-09-09: a vehicle on a core link is walked through the initial connect sequence by the core (autopilot version, protocol version, standard modes, parameters with Qt's timers); `view.coreVehicle` reports the step, firmware, modes and parameter state; `view.coreParameter(name)` serves a value. `settings.appSettings.detectionsHttpPort` (0 = agent default) picks the detections stream port for rigs.
- 2026-09-09: `view.battery` and `view.preflight` depend on and read single facts (`vehicle.batteries.count`, six facts per pack up to four packs, `vehicle.gps.lock`, `vehicle.gps.count`) instead of the battery list and GPS group objects.
- 2026-09-09: `qgc_core_parameter({"vehicle","name","value"|"refresh"})` writes or re-reads a parameter on a core link through the core's parameter machine; `view.coreParameters(id)` lists what the core knows.
- 2026-09-09: `view.video.sourceSize` {width, height} of the decoded frame (null until decoding) so heads letterbox overlays to the painted picture.
- 2026-09-09: component metadata is fetched over MAVLink FTP for core-served vehicles (general file, then parameter descriptions, one-second ack timer, slow-download abort); `view.coreParameter(id, NAME).meta` carries description, units, range and enums.
- 2026-09-09: `view.camera` depends on the camera's properties, not the camera object; battery pack dependencies follow the pack count the vehicle reported; the router re-watches upstream when a view's dependencies change after a recompute.
- 2026-09-09: missions are read and written on core links with the Qt plan manager's protocol and timers (`qgc_core_mission`, `view.coreMission(id)`); the connect sequence loads the vehicle's mission. Fence and rally remain stepped over.
- 2026-09-09: `view.logs` entries say which time branch applies (`timeState`: unreceived, unknown, known) and keep the ISO time for the head to render in its locale; `canDownload` follows QGC's busy-only rule.
- 2026-09-09: the core's MAVLink dialect carries MAVLink 2 extension fields; mission messages are stamped with and filtered by mission type, so a fence or rally transfer sharing the link no longer reaches the mission machine.
- 2026-09-09: fence and rally plans transfer on core links with the same machine as missions (`qgc_core_mission` with `plan`, `view.coreMission(id).plans`); the connect sequence loads them when the vehicle reports the capability over MAVLink 2.
- 2026-09-09: remote ID broadcasts run in the core for core-served vehicles (arm status starts the one-second SYSTEM/BASIC_ID/SELF_ID/OPERATOR_ID cycle, 2.5 s of silence stops it); `view.coreRemoteId` feeds the settings and GCS fix from the Qt side and reports the state; `qgc_core_remote_id` declares an emergency.
- 2026-09-09: `view.messages` items carry `level` (error, warning, normal) read from the Qt handler's style token, so heads bucket without depending on the translated severity word, and an `index` to key on.
- 2026-09-09: PX4 MAVLink log streaming runs in the core for core-served vehicles (`qgc_core_log` start/stop/autoStart, file named as the Qt processor names it, acks, denial handling); `view.coreVehicle.log` reports it.
- Sensor calibration moved into the Rust core (`core-rs/src/sensorcal.rs`). The PX4 `[cal]` status-text protocol and the ArduPilot compass and accelerometer flows both run there; `view.coreCalibration(id)` serves the orientations, progress, outcome and log, and `qgc_core_calibrate` takes start/cancel/next. `view.setup.groups[].pages[].completes` is gone: it encoded a head's capability, not the core's knowledge, and the two heads had already diverged on Radio.
- Review of the calibration slice caught two defects with physical consequences: a PX4 gyro calibration asked the operator to rotate the airframe through all six orientations during a bias measurement (it shows Down only, as `SensorsComponentController.cc` does), and a PX4 stop went out unretried so one dropped datagram wedged the vehicle for the life of the link. A stalled run now ends itself after ten minutes of silence instead of refusing every later calibration.
- `view.flyState` serves link loss from the core: connected, armed, flying, `contactLost`, the state line and the stale notice, with the precedence that lost contact outranks armed and flying. Both heads had hand-written that sentence and that rule; they now read one. Guided offers are deliberately not gated on it, matching upstream `GuidedActionsController`.
- `view.guidedActions` carries the two VTOL transitions, the largest functional gap in the guided set; a head performs one by writing `vehicle.vtolInFwdFlight`. They require the vehicle to be flying, because `MainStatusIndicator.qml` only opens that drawer in the air and `setVtolInFwdFlight` has no armed check of its own. Separately, `view.video.cameras[].configured` no longer compares a `tr()` string: `VideoSettings::sourceConfigured` and a new `sourceEnabled` are `Q_INVOKABLE`, so neither the core nor a head keeps a string equality against translated text.
- A control now says which thing has to restart: `vehicleRebootRequired`, `applicationRestartRequired` and a `restartNotices` array, on both the settings path and the MAVLink parameter metadata. The camera gained `canChangeMode`, which is where `PhotoVideoControl.qml`'s capture-idle terms actually belong; they gate the mode switch, not the shutter.
- A control can be a bitmask: `control` gains the kind and each control carries `bits` of `{ label, raw, set }`, so `ARMING_CHECK` reads as its named checks rather than as 82.000. A fact carrying both `Values` and `Bitmask` stays a choice, as the Qt editor resolves it.
- `view.track` gives both heads the flight trail neither had. `TrajectoryPoints` has no `Q_PROPERTY` and no cap, so the core accumulates its own from `vehicle.coordinate` with QGC's two-metre and one-and-a-half-degree decimation, capped at five hundred points, dropping the oldest with a `dropped` count, one trail per vehicle. The heading is compared as a turn rather than a subtraction, so flying due north is not read as a 359 degree turn; QGC has that bug and it fills an uncapped list.
- Force Arm is in `view.guidedActions`, gated as QGC gates it: offered only where the ordinary arm was refused, never as a standing peer of Arm, and marked destructive. Resume Mission is deliberately not shipped — `MissionController::resumeMissionIndex` returns zero unless the controller is in fly view and the bridge's plan root is not, so the action could never fire; and the `_vehicleWasFlying` latch it needs guards against a disarmed vehicle with a stale `MISSION_CURRENT`, where resuming would strip and re-upload a truncated mission.
- A bitmask bit worth zero is dropped by the core rather than by each head: a checkbox that can never change anything is the same defect as a text field that discards what is typed into it. Both metadata paths store `1 << index`, so masking the value against the entry is right, and an ArduPilot int8 parameter carrying its top bit as -128 masks correctly in two's complement.
- Swept the views for fallbacks that grant a capability rather than withhold one. A fence with no `inclusion` flag now reads as keep-out rather than keep-in, because mistaking an exclusion zone for a boundary to stay inside flies an operator into forbidden airspace. A setup component that requires setup and will not say it is complete now counts as incomplete rather than complete.
- Swept the crate for guards that exist and are never consulted. `Vehicle::apply` was passing a hardcoded chunk id and sequence to the status text handler, so reassembly was unreachable and any message over fifty bytes reached the operator in fragments; the fields are passed now and pending chunks flush after a second as `StatusTextHandler`'s timer does. And an EU operator ID is checked against the core's own Luhn mod 36 implementation rather than trusting `operatorIDValid`, a stored fact the QML does not clear when the identifier is edited afterwards.
- `view.altitudeModes` serves the altitude menu with the gates the Qt editors apply: Terrain Frame goes when the firmware cannot hold it, Mixed Modes is a mission choice only, AMSL is hidden where the build hides it, and everything but the current mode is disabled until the plan has an item. `supportsTerrainFrame` had reached neither head, so both offered a mode a vehicle could not fly.
- The survey generator's oracle is recorded before the rewrite: twelve cases from the real `SurveyComplexItem` through the bridge, three polygons, five grid angles, two spacings, turnaround and refly, every transect point to seven decimals. R6's gate is byte-identical waypoints, so the oracle has to exist before the Rust does. The recorder reads back every fact it writes, which caught two cases that were recording the unsplit output under a split name because `splitConcavePolygons` is disabled upstream.
- The core generates survey transects identically to Qt: ten of the twelve recorded cases match `SurveyComplexItem` to seven decimals, including a concave polygon, five grid angles and a turnaround. Ported rather than reinvented, since identical output leaves no room for a better idea; the two refly cases are deliberately unimplemented and the test asserts it checks exactly ten. `view.control.readOnly` also stopped failing open: a fact that will not say whether it is writable is no longer served as writable.
- The survey generator matches Qt on all seventeen recorded cases including refly, alternating transects and every entry corner. The five cases added here exist because I had ported two code paths that nothing exercised — the orphaned-mechanism trap in my own work. The refly ordering reproduces an upstream defect on purpose: `_optimizeTransectsForShortestDistance` computes four distances and reads three, and correcting it would change generated waypoints and break the gate.
- Corridor scans generate identically to Qt too: eight recorded cases covering straight and bent polylines, a corridor narrower than one pass, one wide enough for five, a turnaround and all four entry corners. Twenty-five oracle cases in total. Two of my own defects fell out of it — the perpendicular was taken from the original edge rather than from the worker line whose length can be negative, which laid the far passes on the near side; and I stopped reading the Qt source at the entry-point switch and missed the alternating walk immediately below it.
- The core computes the camera footprint the generators fly to, matching `CameraCalc` on five recorded cases across orientation, overlap, altitude and lens. Recording it needed the custom camera, which is selected through `cameraBrand`; `cameraName` has no property and the bridge's refusal is what said so.
- The core offsets a map polygon exactly as `QGCMapPolygon::offset` does, five recorded cases across a square, a triangle and a concave shape moved both ways. That is the geometry a structure scan's flight path needs. Every oracle case now states its kind, because filtering on a field two case families share had picked up the wrong cases three times.
- Transects become mission items: the waypoints, trigger distances and closing camera-off command that actually get uploaded, recorded from Qt by saving a plan and reading what the complex item wrote into it. Two of three plans match exactly; the third is recorded failing on purpose, because with a turnaround Qt emits the trigger after the survey entry and this builder is handed bare coordinates that cannot tell a turnaround from an entry. Carrying the coordinate type through the generators is the next piece.
- Coordinates carry their type through the survey generators now, the way `CoordInfo_t` does, and all three recorded plans build mission items identically to Qt including the turnaround one. The rule needs the type: a camera trigger opens at each survey entry, one more at the very first turnaround when images are wanted through the turns, and one closes at the last point of the flight; bare points cannot tell the first two apart.
- Corridor scans build mission items identically to Qt on three recorded plans, sharing the survey's builder now that corridor coordinates carry their type. And `view.warnings` no longer contradicts itself: `arming_blocker` was returning the raw pre-arm string in the case where `warnings()` suppresses it, so a head reading the blocker and a head reading the list said different things about the same vehicle.
- `view.inspector` was describing `systems.0` while the rate change it leads to acts on `activeSystem`. With one vehicle those are the same object; with two, an operator could be shown one aircraft's message rates and have a change land on the other. The view reads `activeSystem` now. Found by the macOS session, who correctly stopped rather than fixing their head, since following the view would have made selection and display disagree.
- Collapsed the two transect generators into one each. The typed generators are the real ones and the flat lists are a projection, so the recorded transect cases are now checked against the same code the mission item builder uses rather than a parallel copy that happened to agree.
- A `Q_INVOKABLE` declared with a `<cstdint>` type is invokable and uncallable at once, because `moc` records the name verbatim and Qt has not registered it. Qt's own aliases are fine — swept the tree, and the two `quint16` invokables convert, established by calling one through the meta system rather than by reasoning about the registry.
- `LinkManager::createSerialConfiguration` lets a head make a serial link, which neither could before: the Qt flow passes a configuration pointer between four calls and a path-based head has nowhere to hold one. It registers without connecting, so parity and flow control stay ordinary property writes and the existing `createConnectedLink` finishes the job from a path. Serial is compiled in on Android where a USB radio is a mainstream way to reach an aircraft.
- A `Q_INVOKABLE` with a Qt typedef parameter is fine: `moc` normalises `quint16` to `ushort`, which is what the metatype says too, and both spellings invoke. `int32_t` fails one line earlier instead, at the metatype conversion, because `<cstdint>` names are neither normalised nor registered. Pinned by a test that goes red if Qt stops normalising.
- `LinkManager::commitLinkConfigurations` gives a head a way to persist a link it edited; nothing reachable wrote the list to disk before, so an edit took effect and vanished at restart. Editing stays ordinary property writes and the commit is the transaction boundary, rather than a wide edit signature that three link types with different fields cannot share. Two core tests that drained the socket once now wait for the message they expect.
- A failed plan save cleared the dirty flag, so both heads and QML reported a plan saved that never reached the disk. Measured by the macOS session on the running app. The first test for it passed with the fix reverted, because the fence polygon it used to dirty the plan could not be un-dirtied: clearing a fence child re-announced it through the list model, whose handler ORs an incoming value into a flag it never lowers, into a controller slot that ignores the value. One defect was hiding under another. Both fixed, each proven by reverting it. `core-rs` also gains the structure scan generator, unit tested; its recorded oracle waits on a way to read back items a structure scan never writes into the saved plan.
- The tile cache is ported to `rusqlite` and keyed identically to Qt, so an existing `qgcMapCache.db` serves rather than re-downloads. The core does not reimplement Qt's string hash: Qt's `qHashBits` is dispatched on CPU features, so a port that guessed it would look right and silently miss every tile. The 37 provider names are recorded with the integers Qt hashes them to, and a provider Qt adds later fails the test instead of quietly missing. The schema oracle is read back out of a database the real map engine created, not copied from the source, so it compares the file Qt writes rather than the statements it was asked to write.
- A structure scan could be planned inside the structure. Which side the vehicle flies depended on the vertex order, because the offset walks edges to one side of their direction and `verifyClockwiseWinding` is called from QML only, on vertex drag. A polygon from a KML file or from a native head over the bridge was never wound, so both new heads could plan a scan into the tower. The flight polygon is wound before it is offset; the oracle carries three shapes in both windings. Also: the tile cache pruned tiles belonging to downloaded offline sets, read a broken database as an empty one, and misread the provider out of four of the 37 hashes. `core-rs` gains the polyline module and polygon area, containment, splitting and winding against a recorded oracle.
- `host.notices` gives a head with no QML root the operator messages and navigation requests `QGCApplication` currently delivers by invoking QML methods by name. On the Android head those invokes fail silently, so all 140 `showAppMessage` call sites are mute, and the setup-incomplete case jumps the operator to the Setup tab with no message explaining why. It is a queue the head drains by id rather than a latest-value property, because two messages in quick succession must both be seen; it is capped and counts what it dropped. The QML path still runs.
- A name the core hands a head to interpolate into a path now has to resolve. The macOS session found the class: a path built from runtime data where the interpolated segment is a core-supplied string, so a wrong name in the core becomes a silent default in the head with no error in between, because a misspelled property and a null one are byte-identical. Every mission kind is now inserted through the invokable the core names and its geometry property resolved down to the vertices. Also fixed a race in the tile cache schema test that only a full-suite run exposed: it waited for the database file to be non-empty, which happens after the first table, then queried the last one.
- The structure scan generator is now checked against what a vehicle actually received. Survey and corridor were matched against Qt from the start; structure scan could not be, because it never writes its generated items into the saved plan, so every assertion about it was hand-derived from reading the C++. A MockLink is connected, three scans are uploaded, and the items are read back off the mission manager. Reading them off the wire rather than the plan file is what makes it an oracle rather than a restatement.
- A 383-item survey now goes to a vehicle and comes back compared item by item, which is the ground half of R6's upload gate. "Byte for byte" turned out to be the wrong words: `MISSION_ITEM_INT` carries coordinates as degrees times ten million, so a coordinate returns quantised and a not-a-number in those fields returns as zero. The test asserts what the protocol can carry — sequence, command, frame and every non-coordinate parameter exactly, coordinates within one step of the wire's resolution — which still catches a reorder, a drop, a wrong frame or a real coordinate change.
- `host.notices` sends a string token rather than an enum ordinal, keeps the oldest eight notices under a flood rather than dropping the first message that explains the rest, and guards its queue. All three came from the macOS session reading the producer before building against it, hours after it landed and while only one head had adopted it. `view.missionKinds` now carries `enabled` and `disabledReason`: a head was told all seven kinds were addable always, so it would offer a takeoff to a mission that already takes off. An unanswered read falls back to what the C++ members start as, not to everything allowed.
- Recording the oracles printed a failure that running the tests never did: the record path skips out of the test after writing the fixture, so the mock link disconnect ran inside an already-skipped test where its wait for the vehicle never completes. Fixed by disconnecting before the skip. Worth fixing rather than tolerating, because recording is exactly when someone is checking whether the numbers moved, and a failure that is always there is one nobody reads.
- `mission.insert` puts the gate behind the action rather than beside it. Both heads were offering a takeoff to a mission that already takes off; gating the buttons on the controller's flags looked like the fix, and the Android session measured on a handset that it inverts the answer, because those flags are assigned only inside `setCurrentPlanViewSeqNum` and a head that never selects a plan item reads constructor defaults forever. The view now says `enabled: null` until something has selected a point, and carries the sequence its verdict is about. The action selects the insertion point, asks, and refuses with a sentence an operator can read. A head never spells a complex item name.
- Continue Mission was never offered to either head, so an operator who paused a mission in flight could not resume it. `guided.rs` compared `currentMissionIndex` against `missionItemCount`, and both answer only in the fly view; the bridge's plan controller is the plan editor's, so it read minus one and zero and minus one is not less than minus one. The bridge now keeps a fly-view controller as well, which is the arrangement QML already has. Found by the Android session in the source; measured here on a real upload, asserting both halves in one test because the plan editor answering zero beside it is what makes the fly view's answer mean anything.
- `mission.insert` now gives an inserted item its shape: an area for a survey or structure scan, a path for a corridor, a launch position for a takeoff, and it removes the item again if the shape will not take. The Android session could not adopt the action without this and was right that doing so would have been a regression, since a survey with no area draws nothing and has to be found and deleted. They also corrected a test of mine that pinned its own stub, and corrected my claim that the core's version has no side effect: selecting the insertion point is one. The advantage is atomicity. My own `launchCoordinate.isValid` assertion was the silent-null defect it was written to catch; the bridge spells it `valid`.
- `view.setup` could never say a vehicle was ready to fly. It read `vehicleComponents` under `elements`, which the bridge writes only for a list model, while `AutoPilotPlugin` declares a plain `QVariantList` that comes back under `value`. So the component list was empty for every vehicle ever connected, and the banner read "This vehicle reports no setup components" over a list of six. Measured by the Android session on an ArduPilot quad, beside their own working read. My tests passed the list to the verdict function directly, so every branch but the empty one was unreachable in the field and reachable in every test. The recorded view contract had the defect written into it: `components` was recorded as `"empty"`, and had said so since the day it was recorded.
- The view contract described a list by its first element, which is why it recorded the setup defect rather than catching it. It folds every element in now, and that immediately widened eleven fields the contract had been under-describing: a mission kind's `complexName`, `geometry` and `geometryProperty` were recorded as always null because a waypoint comes first and has none of them, a battery voltage as a number when a battery reporting none answers null, a control's value as a number when it can be a bool or a string. None was wrong when written; each was written from one sample that happened to be the narrow case.
- A bridge read whose last segment names no property now carries `found: false`, so a misspelled path is no longer indistinguishable from one holding null. Both other sessions reached this from opposite ends within an hour, and my own `launchCoordinate.isValid` assertion earlier the same night was the defect landing on me. The key appears only when the name is wrong, so nothing that works sees a new key. `host.postNotice` lets a rig make a notice arrive, since both heads have adopted the channel and neither has ever seen one — every real poster needs a vehicle, an upload or a settings write. And an unknown mission kind says it is unknown rather than refused, because a head needs to tell "do not insert this" apart from "insert it yourself".
- The view contract now records which fields read the same with no vehicle, with one connected, and with a plan on it, and fails when that list grows. Six defects this week shared one tell: a value that never varied. Coarse on purpose, since the recorder drives few states; what it catches is a field that used to vary and stopped. Separately, `host.notices` is now pinned across four read paths after the macOS session measured it coming back as a list of nulls: the right length with no content, a queue that looks full and reads empty. Three paths carry it here and indexing into the list answers `found: false`, which is correct for a leaf list property.
- A `QVariantMap` crossing the bridge came back as null in the app and as a map in the tests. QtPositioning's QML plugin registers a map-to-coordinate converter, so in a process with QML every map answers true to `canConvert<QGeoCoordinate>()`, and `variantJson` asked that first, converted the notice to an invalid coordinate and returned null. A unit test with no QML never registers it. This is the fixture-versus-artefact trap moved out one level: the real object, the real ABI, the real process, and still not the process the defect lives in. The test now registers the converter itself so it fails without the fix. Also adds `view.modeSlots`, which answers which of the six APM flight mode slots the transmitter is selecting.
- `view.setup` no longer answers whether a head has a native screen for a page. It was a hardcoded list of screens some head might have, and the Android session found what that costs: their head has no Motors screen on purpose, the flag said it did, and the row landed an operator on a page that forwards live telemetry to a support engineer. The two heads will never implement the same set, so any list the core hardcodes is wrong for one of them. `parameterSections` stays, because whether the core can lay a page out as parameters is something the core knows. Also removed a process-wide metatype converter from one of my tests: it reproduced the app's QtPositioning condition correctly and crashed an unrelated suite in a loop, because a converter is permanent and global.
- `view.missionSummary` answers what a mission will cost to fly, formatted once rather than in each head: distance, time, hover and cruise split, furthest point from launch, batteries and an altitude range, in the operator's own units with the raw metres alongside. Every property it reads was verified against a real plan first, because three of the last six defects were views built on a property answering a different question than its name asks. A row the controller has not computed is left out rather than drawn as zero, since needing no batteries and having no battery model are different facts.
- A slot flag in `view.modeSlots` is safe to trust on its own, and a test now says so across every state where there is no answer. The macOS head required two fields to agree before painting a position active, which was a good instinct; checking showed the two cannot disagree. Worth knowing which way round, because defending against a producer that can contradict itself earns its place and defending against one that cannot is a line someone deletes later wondering what it was for.
- The Rust tile cache serves a tile Qt wrote, byte for byte, from the same database. It had no caller until now, which the code review said and I left standing. Three findings on the way: the `type` column is declared INTEGER and holds the provider name as text, so reading it as a number failed on every tile Qt ever saved and my unit tests could not see it because they wrote an integer and read one back; opening a database another writer owns has to be read-only, or an intermittent disk I/O error appears about one run in several; and `QGCMapEngine::init` chooses the database once per process, so my earlier attempt to restore the app's cache path after a test could never have worked.
- `view.missionSummary` pushes an update after an edit, proven by a test that watches it, inserts, and requires the distance to rise twice. The macOS session's plan summary was permanently one edit behind and asked for a settle signal or for `mission.insert` to block until the controller recomputed. Neither is possible: the insert runs on the same thread the recompute is queued on, so waiting for it would be waiting for work queued behind itself. A head reading a derived value once after an edit should watch it instead.
- A goto no longer carries a loiter radius nobody chose. `DO_REPOSITION` param 3 says how wide a forward-flying aircraft circles its target; the core took it from the caller with a zero default, and the macOS session found no head has ever supplied one, so every goto either head sent carried zero. Right for anything that hovers, wrong for a fixed wing. `view.guidedActions` now answers `gotoLoiterRadius` — the operator's setting when the vehicle flies forward, zero otherwise — because that is a vehicle question and a settings question, neither of which belongs on a head.
- `mission.remove` gives removal the same guarantees `mission.insert` has. It refuses index zero, which holds the planned home position and which the plan view offers no way to delete, refuses an index outside the plan before touching anything, and counts before and after. `removeVisualItem` returns void, so without counting there is nothing to tell a removal from a call that did nothing — the shape this review has found six times this week.
- `view.missionItems` lists what the plan holds, so a head draws the list instead of interpolating `visualItems.<index>.<property>` per item per property. That interpolation is the macOS session's third tier of reflection risk, where a name that does not resolve reads as a plausible null down to a default, and this is the largest single reduction in it. The classification comes from six predicates on the item and an unlisted complex type still classifies as complex rather than falling through to waypoint. A test against a real plan found my first mistake: removing everything leaves the settings entry, so a count of one is an empty plan.
- `guided.orbit` takes a place, a width, a direction and a height above the launch point. `guidedModeOrbit` wants a radius whose sign is the turn direction and an altitude above sea level, and the macOS session found their head sending zero for both to a flying aircraft, then withheld the control rather than guess. A head that never learns the sign convention cannot get it wrong. It refuses when the vehicle has not said where it launched from, because an orbit height measured against an unknown launch altitude is an altitude nobody chose.
- The never-varied sweep was recording a sampling artefact as a fact. It reported the mission summary's distance and time as never varying, when the recorder was simply sampling them before the controller had recomputed and storing the pre-edit values as this state's answer. Ten fields left the list once the snapshot waits. Not harmless noise: a field already in that list is one the check has accepted as constant, so a real regression in it would never be reported.
- `QGCHostNotices.cc` and `.h` were committed and the CMakeLists line compiling them was not, so a clean checkout would fail to link against the notice channel both heads adopted tonight. Every build on this machine worked because the change sat in the shared working tree, where all three sessions saw it. A thing that works everywhere it is looked at, because everywhere it is looked at shares the state that makes it work. Found by reading `git status`, not by anything failing.
- `view.modeSlots` read `rawValue` out of a serialised fact and `vehicleType` off the vehicle, and the bridge sends neither: a fact carries `value`, and `Vehicle` has `fixedWing`/`rover`/`vtol` but no `vehicleType`. So the mode channel was never read, every vehicle fell back to channel five, and the view lit whatever was on it — the same defect the view was added to fix, one layer down. My tests passed because the fixture invented both keys, and one of them was asserting the bug as correct. Which parameters name the slots is now decided by asking the vehicle which ones it has, as the Qt controller does. The regression test reads the channel off a real ArduPilot vehicle and compares all six slot names against their parameters.
- A code review found eight defects in the mission actions and the item list, the worst of which could delete an operator's waypoint: the insert invokables return void, so a call that added nothing looked like one that worked, and the rollback then removed whatever the selection still named. The insert now requires the plan to have grown. Also: an orbit height was unbounded where the slider offers 2 to 122 metres, a vehicle that cannot orbit was asked anyway, the insert index reached the model unvalidated, the question was asked one slot late, an unreadable item classified as complex, and an altitude below sea level printed as a dash. Two more from the macOS session: a terrain wait was reported as a blocked item, and an unplaced takeoff carried a coordinate Qt calls valid at zero, zero.
- Registering an event handler no longer starts the link pump. `qgc_core_set_event_handler` did three things under one name and the third was spawning a thread that wakes every 100 ms, takes the transports and hub locks, and can never be stopped. The macOS session wired the first Swift consumer of the push channel and found `qgc-core-pump` in their process having never asked for a link. It now starts where there is something to service. Their second finding needs no code and matters more: `announce()` calls the head's handler from the pump thread, so events do **not** all arrive on Qt's thread — the Watcher's do and the pump's do not, for `view.coreGuided`, `view.detections` and `view.transports`.
- Every list the core serves runs oldest-first now, and each says so in an `order` field. `view.messages` ran newest-first because `StatusTextHandler` prepends, while host notices and the track append, so `last()` meant the newest on two of three lists and the oldest on the other. The Android session found it by taking the last error for the newest and watching it name the first one forever; their test passed because the fixture was built in the order they assumed. Normalising rather than documenting, on their argument: a head that gets it wrong gets a plausible wrong answer, and a comment in a view does not reach a head.
- `view.missionItems` names the paths a head writes an item's fields to, closing the write half of the interpolation problem the Android session identified as the worse half: a read that resolves to nothing draws a default, a write that resolves to nothing does not happen. The test writes to every path the view names, for four item kinds, and found the defect on its first run — a serialised fact's `name` is its display name and the property name is under `property`, so 22 paths had spaces in them and named nothing. Also opts the tile cache reader out of SQLite's shared cache, which the Qt worker enables: joining it produced an intermittent disk I/O error and a row that read successfully and then went missing on the next identical query.
- An item now says whether the vehicle flies a leg to it, and the editable fields can be asked for per item rather than only for the selection. Both were what the Android session needed before adopting `view.missionItems`, and both were things they declined to guess: their map's `isFlownLeg` needs `isIncomplete`, which is on complex items only and is what a survey carries until it has its area, and their head edits whichever waypoint the operator tapped rather than whichever is current. Asking for fields costs an unfiltered read per item, so it is an argument rather than the default. `blocked` does not cover `incomplete`: one is an item missing something the operator must supply, the other is a complex item without its geometry yet.
- `view.messages.order` and `view.track.order` are pinned in the contract's enumerations. Every other token the core emits was already pinned there, and each pin lets a head assert in both directions that every value the core can emit is one it decodes. The macOS session pointed out that order had none, which makes it the field that most needs one: it exists because two heads were guessing which end of a list was newest, and an unpinned field whose purpose is to end a guess invites a different one.
- `view.vehicles` names every connected aircraft — id, name, type, firmware, the link it arrived on, and which is active — and `vehicles.setActive` chooses the one being commanded. The macOS session decided not to port multi-vehicle *commanding*, which is defensible, and then found the part that was not a decision: with two vehicles up, both heads draw whichever is active, name it nowhere and offer no way to switch, so an operator cannot tell which aircraft an arm reaches. Which link a vehicle arrived on matters most, because two identical airframes are told apart by where they are talking. Choosing needs an action because `activeVehicle` is a pointer property; the bridge writes one by `@path` and the path is a list index.
- `altitude` and `altitudeMode` answered null on every item: the first matched a fact by its display name where the bridge keys paths on `property`, and the second is not a fact at all. Both were added hours earlier to unblock the macOS head, and their tests passed because the fake invented the shape. That is the third time this week a hand-written fake described a bridge shape that does not exist, so the contract now records every field that answered null in every recorded state and fails when the set grows — the signal was already there, recorded as `"null"` rather than `"null|number"`, and a re-record absorbed it. Also adds `exitCoordinate`, the last field needed to draw a route through a survey rather than back out of the corner it entered by.
- An item says whether the route stops after it, and `view.missionItems(geometry)` carries each complex item's own vertices. The Android session walked the whole mapping rather than taking fields one at a time and found the view would have made their head worse: their plan window feeds six consumers from one bulk read, the view served four, and adopting it meant two reads and two descriptions of the same plan. Getting there fixed the classification underneath — every complex item read as "complex", so a corridor and a structure scan were indistinguishable and neither could find its geometry, while the survey worked only because it has a flag of its own.
- `endsRoute` matched a translated string, so in a German build a route would be drawn straight past the return to launch. The Android session caught it before adopting, and I had written them the argument against exactly this in the message that shipped it — the rule is easy to state about someone else's field and hard to see in the line you are writing. Two vehicles are now held at once in a test: both named, both carrying their link, exactly one marked as commanded, and either choosable by id. My first attempt raced, because the second vehicle arriving moves the active one. The tile reader stopped reporting a database it could not read as a tile that is not there.
- A survey carries its flight lines and its shot count on the item, closing the first of two gaps the Android session found tracing their six plan consumers onto the views: their map draws transects inside the boundary and the geometry carried only the boundary, so adopting would have drawn an empty outline. Writing the test found the fourth member of the late-arrival family — a pattern's transects are computed after the insert returns, so the read that follows sees an empty list. The property resolved and answered empty, which is how "not yet" was told from "never" in one probe.
- The terrain profile is sampled through a pattern rather than at its corner. A survey covering a kilometre of ground drew as a flat line across whatever it was flying over, and the operator read no collision because nothing sampled the middle. Complex items carry flight path segments with terrain heights along them; the profile walks those now. The Android session asked whether this was a decision — it was not considered. Also removed a test I wrote three hours ago: it held two vehicles, which nothing else in the suite does, and segfaults about a third of the time in the *teardown*, which I localised with a marker after the last assertion. That is a QGC defect rather than a view one, recorded rather than left crashing in a suite three sessions run.
- An altitude carries its unit beside its number. The macOS head draws an editable field, and an editor needs the two apart; handing it only formatted text would have it parsing the string back, which is the shape they had just deleted from their own tool. They offered to substitute the app's units setting and argued against it themselves — a fact carries its own units and substituting assumes the two never diverge — so the item carries both. That is the last of the eighteen fields they enumerated.
- Two changes I had told both heads were done were sitting uncommitted: the notices object's thread affinity and its `order` field. So the notices were the one list of three not carrying the field a head is meant to assert on. Same cause as the missing build line earlier — the staging list is a hand-kept enumeration, and a file added to a directory already in it is not added to the list. That found it once and I fixed the instance rather than the list; it stages the directories now. Everything worked for all three sessions throughout, because the changes were in the tree we share.

### Stream F · Core

Owns `core-rs/`, the generated `QGCBridgeC.h`, the golden dump and fixtures, the router in
`QGCBridgeCore.cc`, and this section. B, C and D consume Rust view-state paths as F publishes
them and delete their model files in the same commit. E records every hardware gate.

### Open decisions

When R1 starts (after Phase 6, or from a cut-off for new screens); the QML shim versus a fifth
native head for Windows and Linux; Bluetooth on macOS; confirming `UTMSP` and `Viewer3D` are
dropped. Recommendations are in the vault plan.

### Stream E · What has never been exercised (2026-09-10)

The plan says hardware gates have not been run past Phase 0 and that E records each one here. None
has been run, so this is the inverse register: behaviour that is implemented, tested by fixtures,
and has never once executed. It is kept because a phase can read as finished while the case its
code exists for has never occurred, and only a list makes that visible.

The macOS rig has **no vehicle** and must not connect a link, arm, upload, calibrate, forward
MAVLink, or write the shared settings file. Fixtures are the instrument for every decode below; for
anything that renders or actuates there is no instrument here at all.

| Behaviour | State | Why it is unreachable here |
|---|---|---|
| Plan upload, and its refusal when the upload check cannot be read | never run | no vehicle; the refusal needs `view.plan` to carry no `upload` object, which needs the bridge or plan root to fail |
| Force Arm, both VTOL transitions | never fired | gated on a vehicle that is refusing an arm, or a flying VTOL |
| Sensor calibration of any kind | never started | forbidden on the shared SITL |
| Bitmask parameter editor | never drawn | `bitmaskStrings` comes from vehicle parameter metadata; all fifteen settings pages answer only choice/number/toggle/text |
| Terrain Frame offered in the altitude menu | never seen offered | needs firmware answering `supportsTerrainFrame` true |
| Flight trail, and its decimation | never drawn | needs `activeVehicleAvailable` and a valid `vehicle.coordinate` |
| `view.flyState` beyond `notConnected` | one branch only | `contactLost` was verified on the Android rig, not this one |
| A camera source needing no address | never displayed | all three configured slots are url-requiring types; changing one writes the shared settings file |

Two of these are worse than they look because they are unreachable on **every** rig, not just this
one. The bitmask editor is covered by fixtures on both heads and drawn on neither, because the core
has not yet emitted `bits` on `controls` — the Android session has a vehicle and still cannot reach
it. The flight trail's decimation is designed twice, implemented twice and observed zero times.

The recorded view contract stores `view.control.bits` and `view.track.points` as `["empty"]`, so it
pins those keys but not their element shapes; unit fixtures in `macos/Tests/main.swift` are the only
thing holding `{ label, raw, set }` and `{ latitude, longitude }`.

What the Android rig settled that this one could not: link loss end to end, and that
**Armed-as-distinct-from-Flying had never been produced by any rig** until `apmvehicle.py` stopped
reporting `IN_AIR` the instant it armed — so the flight-state precedence both heads argued over had
been running against a sim that could not generate the case it was written for. A screen that looks
right is not evidence if the rig can only produce the case that looks right.

### Phase 2's gate needs files this stream does not own (2026-09-10)

Every Analyze page now exists natively. The fifth one, the MAVLink Console, was built in
`c6a437299` — it appeared nowhere in this document until now, neither in Phase 2's list nor in
`## Not covered`, so the gate could never have closed while the list was believed complete.

The ground rule says a view is not converted until its QML is deleted. Deleting
`src/AnalyzeView/*.qml` touches six files, and five of them belong to other streams:

- `src/UI/MainWindow.qml` — the QML shell's Analyze entry
- `src/API/QGCCorePlugin.cc` — registers all five pages
- `src/CMakeLists.txt`, `src/AnalyzeView/CMakeLists.txt`, `test/CMakeLists.txt`

The QML app is also how the other two sessions verify their own work, so deleting it is not a
macOS-stream decision. **Phase 2's gate is blocked on a coordination decision, not on code.**

The same shape will repeat at every later gate, and it is worse there: Plan and Fly QML are far
more entangled with `src/QmlControls` (183 files) than Analyze is. Whoever schedules Phase 6
should decide whether QML deletion happens per-view as each is converted, as the ground rule
says, or in one sweep when QtQuick drops — and if the latter, the ground rule is wrong and
should be amended rather than quietly broken five times.

One cost that is already real: `native` on `view.setup.groups[].pages[]` was the core answering
whether a head had its own version of a page. That is the head's business, the core is removing
it, and this head filters its Vehicle Setup sidebar on exactly that flag. A field that encodes
what a head can do creates this coupling every time.

### Seam (a) closed: the last three Fly view controls QML has and no head does (2026-09-11)

The QML-versus-head control diff is finished. `FlyViewAdditionalActionsList`,
`FlyViewTopRightPanel` and `FlyViewInstrumentPanel` were the remainder. Three findings, none
of them buildable here, all three recorded rather than half-built.

**Change Loiter Radius.** `FlyViewAdditionalActionsList` offers it; the core has no such
guided action, so neither head can. QGC gates it on `_vehicleInFwdFlight` and a visible
`fwdFlightGotoMapCircle`: a Goto placed while a fixed wing or a VTOL is in forward flight
draws a circle, and this changes its radius. The capability is already half here — the core's
`guidedcmd::goto` takes a `loiter_radius` and puts it in `DO_REPOSITION` param 3 for
ArduPilot — but nothing offers it, so every goto this head sends carries 0. The producer has
to move first: the gate is vehicle knowledge. **And it commands a flying aircraft, so it is in
the same class as Clear Mission — not to be shipped unverified.**

**Multi-vehicle.** `FlyViewTopRightPanel` is entirely a multi-vehicle panel: Select All,
Deselect All, and Arm / Disarm / Start / Pause applied to every selected vehicle. It appears
nowhere in this document, in no phase and not in `## Not covered`, and there is nothing for it
in the core either. Four of its five controls command several aircraft at once, which is the
most destructive surface in the application. **Decide whether the macOS head carries
multi-vehicle at all before Phase 5 is called done** — the honest answer may be that it does
not, but that has to be a decision rather than an omission.

**The instrument panel is a user-selectable QML file.** `FlyViewInstrumentPanel` is a
`SelectableControl` bound to `flyViewSettings.instrumentQmlFile2`, so an operator can point it
at their own QML. That is a QtQuick extensibility point and Phase 6 deletes QtQuick, exactly
like `Viewer3D`. Same decision, same shape: rebuild the mechanism natively, or drop it and its
setting together. There are now two of these, which makes it a category rather than a one-off.

### The head has never used the bridge's push channel (2026-09-11)

`src/Bridge/QGCBridgeC.h` has offered `qgc_bridge_watch`, `qgc_bridge_watch_client` and
`qgc_bridge_set_event_handler` since the split. **No Swift calls any of them.** Every store in
`macos/Sources` polls on a `Timer` instead — around twenty of them, at 0.5s to 1s each,
whenever their window is open.

Found while chasing a measured defect: the Plan window's summary is one edit behind, because
the controller recomputes its derived totals after the insert returns and the `reload()` that
follows the edit reads the previous values. The core answered the question directly
(`a1f274d9b`): the view already depends on the controller, so a **watch** delivers the new
totals when they exist, and they proved it with a test that inserts and requires the update to
arrive twice rather than once. Polling would fight the editor's selection state for a value
that already announces itself.

So the fix for that defect is not a timer, and neither is the general shape. **This is the
single largest piece of unused mechanism in the head.** Adopting it is not a small change:

- The event handler is a C function pointer, so Swift needs `@convention(c)`, which cannot
  capture context — the callback has to route through a registry keyed by path.
- Events arrive on the Qt thread. Every `@Published` mutation has to hop to main, and this
  head already has one recorded scar from calling the bridge off Qt's thread: the segfault
  lands much later than the call that caused it.
- The stores that poll are not all equivalent. Some poll things that genuinely have no
  producer signal; those should stay.

Worth doing per-store, starting with `view.missionSummary` where there is a measured defect
and a proven signal, rather than as a sweep.

### The staleness sweep run from the other end, and the premise it nearly rested on (2026-09-11)

The four staleness defects found in the Plan window were all found the same way: something
arrived after the read that drew the panel. That question is exhausted. The productive
inversion is **what can change without an edit at all** — a fact the window would never
re-read because nothing the operator did caused it to change.

Run over every view the Plan window reads, against the `DEPS` each one declares in
`core-rs`:

| view | deps | verdict |
|---|---|---|
| `view.missionItems` | `visualItems.count`, `currentPlanViewVIIndex`, `containsItems` | plan-edit driven |
| `view.missionKinds` | `currentPlanViewSeqNum`, the four insert-validity flags | plan-edit driven |
| `view.fences` | `geoFenceController.polygons`, `circles`, `rallyPointController.points` | plan-edit driven |
| `view.surveyStats` | `missionItemCount`, `plan.dirty` | plan-edit driven |
| `view.plan` | …`vehicles.activeVehicleAvailable`, `vehicle.armed`, `vehicle.flightMode` | **vehicle state** |
| `view.altitudeModes` | `vehicles.activeVehicleAvailable`, `vehicle.supportsTerrainFrame`, … | **vehicle state** |

Two of six, both now watched (`7232847c9`, `d45d9f5e8`). The other four are not a gap to
fill later — they are provably covered by the reload an edit already triggers, and that is
the useful half of the result.

**The near-miss is the part worth keeping.** Both fixes were written on the stated premise
that this window never polls. That premise was asserted twice before it was checked, and it
survives only on a technicality: `MissionStore` is constructed twice — `PlanWindow.swift`
and `FlyWindow.swift` — and `startWatching()`, which reloads on a 0.5 s timer, is called
**only by the Fly window**. The Fly view reads a plan it does not edit, so it has to look
again; the Plan window edits the plan and reloads on the edit. So one instance of this class
polls at 2 Hz and the other never polls, and *any rule reasoned about "the store" is wrong
for one of them*. Had the call sat one line higher, both commits would have been fixing
something a 500 ms timer already fixed.

The lesson generalises past this file: **a staleness argument is about an instance, not a
class.** Ask which construction site you are reasoning about before claiming what it does
not do.

One consequence is accepted rather than fixed: the Fly window's instance now also carries the
vehicle-presence watch, which is redundant against its own 2 Hz poll. It is one extra reload
on a rare event, and scoping the watch per-instance would cost more than it saves.

### What the head costs at Phase 4's gate scale, measured (2026-09-11)

Phase 4's gate is a **200+ waypoint survey**. Nothing had ever measured the head at that
size; every reading below is from a running app with a scratch plan grown to 227 items,
timed through `/bridge/get` with a 1.6 ms HTTP floor subtracted where it matters.

| read | 1 item | 41 items | 227 items |
|---|---|---|---|
| `view.missionItems` | 2.0 ms | 21.3 ms | **146.5 ms** |
| `view.terrainProfile` | 0.9 ms | 8.9 ms | 73.9 ms |
| `view.plan`, `view.missionSummary`, `view.missionKinds`, `view.fences`, `view.surveyStats`, `view.mapScale` | ~0 | ~0 | ~0 |

Linear at **0.64 ms per item** for `view.missionItems`, about half that for the terrain
profile. Every other view this window reads is free at any size — the cost is two reads.

**Correction (same day): the duty-cycle claim above was wrong, and the table is not what it
looks like.** Every figure in it was timed through `/bridge/get`, so each one includes a TCP
round-trip, JSON serialisation and a Python-side parse. None of that happens when the head
calls `Bridge.group` in-process, and the difference is not a constant factor — it is most of
the number. What the table actually measures is the cost of *observing* these views from a
test harness, which is worth knowing and is not what it was first written up as.

The claim was tested directly rather than argued about. `readItems()` was put behind a cache
that only re-reads when a watch says the list changed, so the 0.5 s tick stops re-reading
`view.missionItems` entirely; the Fly window was then opened on a 121-item plan and CPU
sampled with `top` once settled. **Cache on: 52.3%. Cache off, re-reading every tick: 51.4%.**
Within noise. The read the table called the dominant cost is not measurable at the process
level, and the optimisation built on it was reverted rather than kept as a change that looks
principled and does nothing.

**What the ~50% is, answered.** It is not the tick, not the map and not the plan. Decomposed
by opening one thing at a time, all with an empty plan and no vehicle:

| state | CPU |
|---|---|
| no native window open at all (Qt host only) | **21.1%** |
| Plan window open | 26.9% |
| Plan and Fly open | 23.6% |
| Plan and Fly open, 121-item plan | 39.4% |

The floor is ~21% before a single SwiftUI window exists, and adding 121 items moves it by
about two points. `sample` on the idle process shows ten `GstVideoWorker` threads plus
`rtpjitterbuffer`, `queue*:src` and `source:src` threads, and `settings.videoSettings.videoSource`
reads **UDP h.264 Video Stream**, not Disabled. So the floor is a running GStreamer pipeline
receiving nothing.

Stated as inference, not proof: the decisive experiment is to disable the video source and
re-measure, and this stream is forbidden from writing a video setting. What is established is
that the cost is independent of plan size, of item count and of whether any native window is
open — which is on its own enough to rule out everything this stream owns, and is why the
`readItems()` cache could not have helped whatever else was true.

`ps -o %cpu` on macOS is a decaying one-minute average and reads high straight after a plan
build; the settled `top` samples above are the ones to trust.

**Why the poll cannot simply become a watch.** The comment on `startWatching()` gives the
real reason it exists: the Fly view does not edit this plan, and a plan can arrive from the
vehicle or from a file. `view.missionItems` declares its deps as `visualItems.count`,
`currentPlanViewVIIndex` and `containsItems` — all of which a *replacement* plan of the same
length leaves unchanged. So a watch built on today's deps would cover editing and miss
exactly the case the poll was written for. Converting it needs a dep the core does not yet
declare, something that fires when the plan is replaced rather than resized. That is a core
change, and the Fly window is another stream's file; both have been told.

Recorded rather than fixed: no head-side change here is both safe and worth 220 ms, and the
two candidate shortcuts are the staleness class this window just spent four commits removing.

### The screenshot cannot see the map, and it reports that as an empty map (2026-09-11)

`native_screenshot` captures an AppKit window by window id. It does **not** capture the
Metal-composited content of an `MKMapView`. Two captures of the Plan window over a 120-point
lawnmower mission, correctly framed, showed a flat beige rectangle: no satellite imagery
despite `mapType: Bing Satellite`, no route polyline, and in one of the two a handful of pins
and in the other none at all.

Every one of those absences is the capture, not the map. The head's own counters, read from
the same running app in the same second:

    placed 120   annotations 120   routeLegs 120   tileOverlay true   framed true
    renderers { calls: 347, kinds: [CachedTileOverlay, MKPolyline] }

and the map centre sits within 0.1 screens of the mission's centroid. Both renderers had run.
The zero additional draw calls over the next six seconds are a static map not redrawing, which
is correct, not a stall.

**This nearly became a severe phantom defect.** The first capture, at a wider frame, showed
three pins for a 216-item mission, and the obvious reading was that MapKit was decluttering
`displayPriority = .defaultHigh` annotations down to a handful — a plausible defect at exactly
the gate scale, with a named line of code to blame. It survived one screenshot and died on the
second, where a tighter frame produced *fewer* pins rather than more. An instrument that cannot
see its subject does not return nothing; it returns something that reads as a finding.

So the pin-density question is **not answered, in either direction** — nothing here says
MapKit does or does not cull at 200 waypoints, only that screenshots cannot be used to ask.
Answering it needs an instrument inside the process: a count of annotation views MapKit
actually vends, which is what `viewFor annotation` could report and does not.

The terrain panel in the same captures rendered perfectly, because it is drawn in SwiftUI.
That contrast is the tell, and it is what makes the empty map look like a real defect rather
than a blind spot.

### The pin-density question, closed as a decision rather than a measurement (2026-09-11)

Left open by `d05f7a084`: at 200+ waypoints, does MapKit declutter the mission pins down to a
handful, and is that a defect? The instrument that would answer it — a count of annotation
views MapKit actually vends, or better, how many of those end up `isHidden` — was the next
piece of work. It is not being built, and the reason is worth more than the count.

**The answer would change nothing, because the current behaviour is already the right policy.**
`MissionMap.swift:693` sets `displayPriority = item.isCurrent ? .required : .defaultHigh`.
`.defaultHigh` is exactly the setting that lets MapKit hide a pin when it collides with
another, and the selected item is pinned to `.required` so it is never the one hidden. At the
gate's own scale that is the behaviour to want, and this window already learned why the hard
way: 405 overlapping collision circles on the terrain profile were drawn faithfully and read
as one solid smear, fixed in `b44045f1b` by drawing runs instead of points. Two hundred
overlapping numbered markers is the same defect with a different shape. Decluttering is not
MapKit losing the operator's data; it is MapKit declining to reproduce that smear.

**The mission's shape does not depend on the pins.** The route is an `MKPolyline` overlay, and
overlays are not subject to annotation decluttering — they always render. `renderers` confirms
it runs (`[CachedTileOverlay, MKPolyline]`). So the operator sees the full path at any density,
with markers thinning out as they collide and the selected one always drawn. That is the
correct reading of the panel, not a degraded one.

What stays genuinely unknown is the exact count MapKit shows at a given zoom, and nothing here
needs it. Recorded so the next pass does not rebuild the instrument to answer a question whose
answer has no consequence — and so that if the policy is ever changed to `.required` for every
item, this is the note explaining why it was not.

### Phase 4's real completion test, and where this window actually stands (2026-09-11)

The working rule has been "a view is not converted until its QML is deleted". For this tree
that rule was already corrected above: the QML stays while Linux, Windows and the Qt-hosted
Android path ship from it, so the deletion becomes **"macOS stops loading it"**. That is a
checkable fact and it had never been checked for the Plan view.

**It has not stopped loading.** Reading the live QML tree of the running macOS app, with every
native window open:

    20 of 133 QML nodes are plan chrome, 0 visible

    planDock, planDockAdd, planDockAddWaypoint, planDockCenter, planDockFile, planDockMapType,
    planInspector, planTerrainSheet, planUploadButton, planNameCapsule, planStatsCapsule,
    planLayerSelector (+3 options), planViewSwitch (+2 options), planPlacementHint,
    planDetailBack

Every one of those is something `PlanWindow.swift` now draws natively. They are constructed
and invisible, not absent. So the native Plan window is complete enough to use and the QML
Plan view is still being built alongside it on every launch.

What this does **not** establish: whether those nodes cost anything meaningful once invisible,
whether any are shared with the Fly view's toolbar rather than being Plan-only, or how hard
they are to stop constructing. The idle-CPU floor measured in `a203d62f4` was attributed to a
running GStreamer pipeline and that attribution is unchanged — this is not a competing
explanation for it, only a second thing that is also happening.

**Whose call it is.** What the Qt host loads is decided in `src/main.cc`, `src/Bridge/QGCEmbed.cc`
and `AppShell.swift` — stream A's files, not this one. Recorded here and raised with the peers
rather than changed.

**Two of those three open questions are now answered, and the count has moved (2026-09-11).**
Re-measured on a running native build rather than carried forward: **23 of 142 live QML nodes
are plan chrome, still 0 visible.** It was 20 of 133. The QML plan UI is growing while this
window replaces it, which is the actual argument for acting rather than the node count itself.

**How hard they are to stop constructing: one gate.** All 23 descend from a single object,
`PlanView { id: planView }` at `src/UI/MainWindow.qml:274`. **They are Plan-only** — the second
open question — because `PlanViewToolBar` is instantiated *inside* `PlanView`, at
`PlanView.qml:382`, so the toolbar capsules are not shared with the Fly view's toolbar despite
appearing as siblings of it in a tree dump. They read as direct children of `MainWindow` only
because `planView` carries an `id` and no `objectName`, so `namedAncestors` skips over it. Worth
stating plainly, since the earlier reading implied otherwise.

A correction to what the list above implies: these are **not** the upstream plan view. They are
the Aircast redesign's overlay chrome, declared across `PlanView.qml` and
`PlanToolBarIndicators.qml`. The distinction matters because it is the redesign the other two
sessions compare against, not dead upstream code nobody reads.

What the third question still lacks is a measurement: whether 23 invisible nodes cost anything.
They are not drawn, and the idle-CPU floor remains attributed to the GStreamer pipeline.

**Proposed, and put to both peers rather than landed.** A `Q_PROPERTY` on `QGCCorePlugin` saying
the host provides its own plan UI, true only under `QGC_NATIVE_UI`, with the `PlanView` becoming
a `Loader` gated on it. No QML deleted and no tree changed for any build that is not native
macOS, so both peers' probes keep working unchanged. Held until they answer whether either
verifies against the QML plan chrome *on a macOS build specifically* — on any other build the
gate costs them nothing.

A third instrument lesson, cheaply learned this time. The first attempt asked
`strings -a libAircastQGC.dylib` for `PlanView.qml` and got 0, which reads as "not in the
build". The same command returns 0 for `FlyView.qml`, which is certainly in the build —
qmlcachegen leaves no plain filename to find. The control case was in the same command as the
question, so the blindness surfaced immediately instead of becoming a finding.

### The alarming fallback had no siblings (2026-09-11)

`db76aa585` fixed a rule whose missing-value fallback returned an *assertion* — "Mission is
below terrain" — rather than an admission that nothing was known. Worth asking immediately
whether any other model does the same, since the shape is easy to write by accident.

Swept every `guard … else { return <named constant> }` in `macos/Sources/*Model*.swift`.
Nineteen hits, no siblings. Every other one marks the value unknown (`—`, `Not set`,
`.unknown`) or degrades to a lesser fact it does hold (a detection without a confidence draws
its label; a GPS fix without a satellite count draws the fix). None of them claims a state the
data does not support.

So this was a one-off, not a habit, and the sweep does not need running again.

### The route that does not stop at the landing, for the third time (2026-09-11)

The core has begun porting `MissionController`'s arithmetic, and serves its own flown distance
beside the controller's (`distanceComputedMetres` next to `distanceMetres`) so the two can be
compared before one replaces the other. Walking that on this stream's own plan shapes, through
the debug probe on a running app:

| plan | controller | computed |
|---|---|---|
| takeoff + 1 waypoint | 0.00 | 0.00 |
| takeoff + 2 waypoints | 492.80 | 492.80 |
| with a survey in the middle | 7981.43 | 7981.43 |
| ending in a return to launch | 7981.43 | 7981.43 |
| **a waypoint after the landing** | **7981.43** | **8674.60** |

Stable over four reads. The first four are exactly the shapes the core's unit tests build; the
fifth is the one they do not, and it is the only disagreement.

**It is the same defect this head already had.** `a14f1a2f8` stopped the route polyline at
`endsRoute` and skipped items whose `flownLeg` is false, because one list was answering two
questions: the items to upload, and the legs actually flown. Those two fields exist on every
item for that reason, they are served by the very view the new arithmetic reads, and the tail
of a plan like this shows them already correct:

    seq=145  Return To Launch  endsRoute=True   flownLeg=False
    seq=146  Waypoint          endsRoute=False  flownLeg=True

A rule phrased as "a landing ends the route" covers a landing that is *last*, where dropping
the following leg and there being no following leg cannot be told apart. Reported; the
arithmetic is the core's to fix.

**The method is the durable part.** A unit test builds the shapes its author thought of. This
stream already owns a tool that builds a plan with a takeoff, a waypoint over real terrain, an
ROI, a survey, a landing and a waypoint after the landing — assembled for a different purpose
entirely, and the ROI and the after-landing waypoint are in it precisely because both once
broke this head. So whenever the core ports an arithmetic and serves both answers, walking it
on those shapes is close to free and starts from a set of cases chosen by past defects rather
than by imagination. Worth repeating for each port rather than waiting to be asked.

No permanent check was added for `distanceComputedMetres`: it is scaffolding due for deletion
once the port is trusted, and pinning a temporary field would break its removal.

### The probe does not build states the window forbids (2026-09-11)

The core session fixed the flown-distance walk and recorded the shape that found it — a
waypoint after a landing — as one that "cannot be built through the editor at all", pinned by a
unit test over a hand-built item list because QGC refuses to append after a landing. That is
true of a LAND command and not of the plan this stream actually built, and the difference
matters: it decides whether the defect was reachable by an operator or only by a fixture.

Checked on the running app rather than argued. `arm(land)` on this vehicle yields a **Return To
Launch**, not a land:

    seq=2  kind=command   Return To Launch   endsRoute=True   flownLeg=False
    seq=3  kind=waypoint  Waypoint           endsRoute=False  flownLeg=True

and `view.missionKinds` immediately after that RTL reports `waypoint` as `enabled: true` with an
empty `disabledReason` — the same answer it gives before the RTL. So the Plan window offers the
action, the insert succeeds, and an operator reaches this shape by clicking Land and then the
map. **The defect was operator-reachable, not fixture-only.**

**This was worth checking for a second reason: whether this stream's probe can build states the
window would refuse.** If it could, any finding from a probe-built plan would be suspect — the
instrument would be manufacturing its own evidence, which is the failure mode that produced two
phantoms here today. It cannot: the gate is the core's `enabled`/`disabledReason` on
`view.missionKinds`, the probe inserts through the same path the window's own picker uses, and
the core reports the kind as offered. The probe reaches what the window reaches.

The asymmetry underneath is real and correct on both sides: `endsRoute` is true on an RTL for
the purpose of measuring the flown route, while the same item leaves `waypoint` insertable,
because an item after a return to launch is still uploaded to the vehicle — it is simply never
reached. One list, two questions, again.

Verified after their fix, on the shape that found it: controller 7981.43, computed 7981.43,
delta 0.00, and still 0.00 with two waypoints after the landing. The scaffolding is opt-in now
(`view.missionSummary(verify)`), so the watched plain path is back to ~1.0 ms at 121 items from
202 ms, with the key present and null rather than absent — the shape stays constant, which is
why no contract re-record was needed on this side.

### An intermittent failure can be a constant failure with intermittent visibility (2026-09-11)

The tile-cache test has failed in full suite runs all day, passed alone every time, and resisted
reproduction in the core session's tree. With the core's diagnostic finally on the function that
was answering `-3`, the first run here produced this — **in a run that passed 703/0/89**:

    qgc_core_tile_size: the tile database could not be read: attempt to write a readonly database

The database errors on the runs that succeed as well. The test polls in a loop that tolerates a
failed read and asserts on whatever the call after the loop returns, so what varies between a red
run and a green one is not whether the read fails but **which call catches it**. Every green run
was evidence of nothing, and so was every count of them.

The mechanism follows from the error and one pragma: `journal_mode` is `delete`, a rollback
journal rather than WAL, and the reader opens `mode=ro`. A read-only connection that meets a
journal the writer has open mid-transaction must roll it back before it can read, and rolling
back is a write. Reported; the database and its reader are the core's.

**Two attributions abandoned on the way, both mine and both plausible.** First that a leftover app
was writing the same file — the database is `QDir::temp()/qgc-core-tilecache-<applicationPid>.db`,
created fresh per test process, so nothing outside the suite can touch it. Second, and more
tempting, that this stream's habit of `kill -9` on the app left hot journals behind: same
refutation, and it would have been a satisfying story about a runbook instruction causing a bug.
The writer is the test's own map engine, on its own thread, in its own process.

**The general form is worth keeping.** "Intermittent" describes the observation, not the fault. Ask
whether the fault is constant and only its visibility varies, because a green run of a test that
swallows its own failures proves less than it appears to, and counting green runs multiplies that
by nothing.

### FlightMapTest's flick assertion, for whoever owns FlightDisplay (2026-09-11)

Left here rather than in a chat message, because the owner is not this stream and a finding in a
message is lost. **The proposed mechanism below is refuted; the suggested change is not, and it
stands on different grounds.**

`FlightMapTest::_mouseDragFlicksWithInertia` failed once in a full 89-suite run on this line:

    QVERIFY(map->property("flicking").toBool());

The drag itself landed — the position assertion immediately above it passed in the same run. The
two lines that close the test already assert the behaviour it is named for: that the map keeps
moving after release and ends more than 50 px further on. So `flicking` is a transient
intermediate, asserted between a cause and an effect that are both already checked.

**What was proposed and what happened to it.** `flickMouse` is two `QTest::mouseMove` calls with a
20 ms delay, and that delay is a minimum — `QTest::qWait` returns when the event loop gets round
to it. Qt's Flickable derives flick velocity from the wall-clock gap between events, so a slow
enough machine should produce a drag that never qualifies as a flick. That predicts the exact
observed failure, and it is wrong, or at least not sufficient: the test was run alone on an idle
machine (12/0) and alone at a sustained load average of **62 on 14 cores** (12/0). Saturating the
host does not reproduce it. Whatever the full suite does to this test, it is not simply
contention.

**What is still worth doing, independent of the trigger.** The assertion checks a flag that exists
only during the inertia, between two assertions that already bracket it. Removing it, or replacing
it with a `QTRY` on the outcome, would leave the test asserting inertia without asserting a
transient that depends on how the gesture was synthesised. That argument is about what the test
measures, not about why it failed, so it survives the refutation above.

**Recorded as an unfinished diagnosis on purpose.** The failure is real, seen once, in a full run;
the mechanism offered for it did not survive two attempts to reproduce it. Saying so is cheaper
than leaving a confident story for someone else to act on — this stream published a
0.64 ms-per-item figure and a 44% duty cycle earlier today on exactly that kind of reasoning, and
had to retract both after measuring.

### Which locales the survey-outline defect actually reached (2026-09-11)

`eddf72d1f` says the survey outline was missing "in any build that was not English". That is
wrong, and the correction is worth more than the fix because it is about how the defect hid.

The lookup matched a `tr()` name against a fixed English literal, so it only broke where the
name is *actually translated*. Reading `translations/qgc_source_*.ts` for the three complex item
names:

| locale | Survey | Corridor Scan |
|---|---|---|
| az_AZ | Müşahidə | Dəhliz Scan |
| ja_JP | 調査 | 回廊スキャン |
| ko_KR | 서베이 | 복도 스캔 |
| pt_PT | Varredura | Varredura Corredor |
| zh_CN | 勘测 | 走廊扫描 |

**Five locales, not "not English".** Fifteen shipped translations leave these names in English,
`tr_TR` translates "Survey" to "Survey", and **German is one of the untranslated ones** — which is
the part worth keeping. German is the locale anyone reaches for first when testing this class,
and it would have shown the polygon drawing perfectly.

The test fixture was wrong in the same direction and for the same reason. It used "Vermessung"
and "Korridor-Scan", words this software never produces, chosen because they are the obvious
German for the English. Now it uses 調査 and 回廊スキャン, read out of `qgc_source_ja_JP.ts`. The
rule under test is unchanged and the assertions still fail when the lookup is reverted; what
changed is that the fixture now contains a string a running build can actually emit.

Also established while checking: this build does compile and embed all 48 translations, as Qt
resources under `:/i18n` rather than as files in the bundle. `find`ing the app for `*.qm` returns
nothing and `strings` on the dylib returns nothing, and both of those are the instrument being
blind rather than the translations being absent — the third false negative from that family
today. The launcher now forwards trailing arguments so a single instance can run in another
locale via `-AppleLanguages`, which sets NSArgumentDomain and persists nothing.

### Only one of the three locale defects is real, and a static initialiser is why (2026-09-11)

Verified on a running Japanese instance (`build-run.sh -AppleLanguages '(ja)'`), which is the
first time any of this was exercised rather than reasoned about. Three results, and two of them
contradict what both sessions believed:

    plan.missionController.complexMissionItemNames -> ['Survey', 'Corridor Scan', 'Structure Scan']
    the inserted survey item's name                -> 調査
    surveys [4], surveyOverlays 1, SurveyPolygon rendering

The list is English while the item is Japanese, in the same process, at the same moment. The
cause is in two lines of QGC:

    const QString SurveyComplexItem::name(SurveyComplexItem::tr("Survey"));   // static
    QString commandName() const final { return tr("Survey"); }                // per call

The static is initialised before `main()`, and therefore before `QGCApplication` installs any
translator, so it holds the source string for the life of the process. `commandName()` is
evaluated on each call, after the translators are in, so it is translated. **Everything keyed on
`::name` is permanently English; only what is keyed on `commandName()` moves with the locale.**

So of the three defects attributed to one cause:

- **Inserting a complex item works.** `complexMissionItemNames` is built from the statics, the
  core passes its English literal, and `insertComplexMissionItem` compares against the same
  statics. A survey was inserted in the Japanese run above. The core's note in `actions.rs` says
  this fails outside English; it does not.
- **The pattern menu keeps its glyphs.** It is driven by `complexMissionItemNames`, which is
  English, so `byComplexName` matches and the symbol resolves. This stream reported that as
  broken; it is not.
- **The geometry lookup really was broken**, because it alone used the item's `name`, which is
  `commandName()` and therefore translated. That is `eddf72d1f`, and the Japanese run is the
  evidence: the polygon draws.

Two sessions each reasoned from "the name is `tr()`, therefore the locale breaks it" and neither
checked which of two names a given path reads. The reasoning was sound and applied to the wrong
half of a pair — the same failure as the duty-cycle figure and the flick mechanism, in a
different costume. A single run in a locale that actually translates these strings settled all
three in under a minute.

### A second instance of the same class, in the setup window, needing a core field (2026-09-11)

Applying the question that settled the survey outline — *which of two names does this path read,
and is it frozen or live?* — to the rest of the head found one more:

    VehicleSetupWindow.swift:738  component.name == "Sensors" && !sensors.failing.isEmpty   <- the defect
    VehicleSetupWindow.swift:856  page.name      == "Sensors" && !sensors.failing.isEmpty   <- safe

**Only the first of those two is a defect, and grouping them was the same error again.** They
look identical and read different sources. A *page* name comes from `PAGES` in `core-rs/setup.rs`,
a core-owned static of English literals, so line 856 compares a core literal against a head
literal and is correct in every locale. A *component* name comes from QGC's `VehicleComponent`.
Two lines one apart, matching the same string, and only one of them crosses a translation
boundary — which is the pair problem a third time, inside the finding written to explain it.

The name comes from `view.setup`'s components, which the core reads from QGC's
`VehicleComponent`. `SensorsComponent.cc:16` initialises it as `_name(tr("Sensors"))` — a
**constructor member initialiser, not a file-scope static**. The component is constructed when a
vehicle connects, long after `QGCApplication` installs the translators, so unlike
`SurveyComplexItem::name` this one really does move with the locale. The distinction is the whole
lesson from `1434acb9c`, and it cuts the other way here.

"Sensors" is translated in the same five locales as the mission item names — az_AZ センサ-style,
ja_JP, ko_KR, pt_PT, zh_CN — so in those builds the comparison is false and **the sensor-fault
badge silently never appears**. German is untranslated again, so the obvious test would pass.

**This one cannot be fixed in the head.** The component object carries `name`, `needsAttention`,
`openable` and `blockedReason`, and no invariant identity. `needsAttention` is
`requiresSetup && !setupComplete` — whether the component still needs configuring — which is a
different question from whether its sensors are currently failing, so it is not a substitute.
The fix is the same one the core has already made once: `view.missionKinds` gained `className`
because it is the half that survives translation, and `view.setup`'s components need the same.
Raised with the core rather than worked around.

**Not verified live, and it cannot be from here**: with no vehicle connected `view.setup` reports
no components at all, so the comparison never runs. The defect is established from the
initialisation site and the translation files, not from a running instance — which is exactly the
kind of claim that has been wrong three times today, so it is recorded as reasoning and labelled
as such rather than asserted.

### The fixture-shape checker was built, measured and thrown away (2026-09-11)

Three fixtures in one day held something the producer does not send — a flat `latitude` where
`MissionItem` reads `json["coordinate"]`, fixtures missing `movable` once the core served it,
and a coordinate in the core's own tests lacking `valid`. That is the threshold this stream set
for building the obvious tool, so it was built: compare every key a test fixture sets against
every key the model actually reads, statically, no running app.

**It does not work, and the reason is more useful than the tool.** Its premise — a fixture
setting a key the model never reads is a defect — is wrong. It flagged nine, and every one is a
fixture faithfully carrying a key the producer really emits and the model has no need for:
`inserted` and `refused` on an insert answer, `removed` and `remaining` on a remove,
`compId` on a MAVLink message, `mode`, `storageStatus`, `shots` and `batteryRemaining` on a
camera. All nine verified against `core-rs`. Mirroring the producer is exactly what a fixture
should do, so the check punishes the right behaviour.

**And it caught the motivating bug by accident.** Reintroduced, the flat-`latitude` fixture is
flagged — but only because the model reads it as `coordinate?["latitude"]`, and the `?` breaks
the `\w+\[` the key-extractor matches on. Written without the optional chain, the key would have
been collected and the bug would have passed. Detection resting on a regex quirk is not
detection.

The real defect shape is narrower than the tool's premise: **a fixture setting a key at the
wrong nesting level** — a name that exists in the model's vocabulary, at a different depth. That
is what makes it invisible to review, because the key looks right.

Two smaller things worth keeping. The first version used a bare `"([^"]+)"` to find keys and
paired the closing quote of one literal with the opening quote of the next, reporting the code
*between* keys as a key and declaring 247 fixtures broken — a tool crying wolf at maximum volume
on its first run. And checking the four camera keys against `core-rs/src/camera.rs` alone
returned zero for all of them; the file is `cameracalc.rs` and the keys are elsewhere in the
crate. Concluding from one file would have been a four-finding phantom inside the investigation
of a tool built to prevent phantoms.

The guard that does work is the cheap one already in place: assert the fixture satisfies the
precondition the test depends on, next to the assertion that depends on it.

### Walking the core's ports has paid three times, always on the same pair (2026-09-11)

`98a5ad94f` ports the mission's altitude band and the telemetry reach, serving each beside the
controller's answer behind `view.missionSummary(verify)`. Walked on this stream's plan shapes:

| shape | controller | computed | delta |
|---|---|---|---|
| takeoff + 2 waypoints | 562.90 | 562.90 | 0.00 |
| with a survey in the middle | 925.05 | 1129.45 | +204.40 |
| ending in a return to launch | 925.05 | 1129.45 | +204.40 |
| **a waypoint after the landing** | **925.05** | **1616.59** | **+691.54** |

The altitude band agrees on every shape. The reach does not, in two unrelated ways, and only one
of them is arithmetic.

**The post-landing disagreement is the `endsRoute` half, missing from a walk written beside the
one where it had just been added.** `flown_distance` truncates the list at the item that ends the
route; `max_telemetry_distance` filters on `flownLeg` and never truncates, so it measures to an
item the vehicle turns for home before reaching. Exact: that waypoint is 1616.59 m from home and
the computed answer is 1616.59.

**The survey disagreement is a question, not a slip, and was reported without attributing it.**
The survey's entry is 925.05 m from home and the controller answers exactly that, while the
computed walks the transects to 1129.45. A survey's far corner genuinely is further away and a
telemetry link genuinely has to reach it, so the computed may be the better answer and the
controller the one with the bug — but that decides what the row means, and it is the core's call.
Worth knowing that the row's value will move when the port lands.

**Three hits from this job now, all the same family: a rule about WHICH ITEMS COUNT, applied in
one walk and not in the next one written.** The route stops at a landing; a region of interest is
never flown to; an item after the end is uploaded and never reached. Each is a filter, each has
been got right once and then omitted from a sibling function within the hour. So the thing to
check on any new walk over items is not whether it is correct in general but whether it carries
**both** guards — `flownLeg` and `endsRoute` — because carrying one and not the other is what
every instance has looked like.

**Swept this head's own walks against that rule, and it is clean — but the rule has a limit worth
stating before someone applies it too widely.** `MissionItem.route` carries both guards and
`unreached` shares `routeEnd` with it. The other walks over items — `frameProbe` and
`centreState`'s `missionPoints` — filter on `hasPosition` alone, and that is **correct**: they
compute the map region that has to contain everything drawn, and a region of interest and an item
after the landing are both drawn. Adding the route guards there would quietly drop items out of
the frame the operator is looking at.

So the rule is not "every walk over items needs both guards". It is: **a walk that answers a
question about the flight needs both; a walk that answers a question about the drawing needs
neither.** Every instance found so far has been the first kind, which is why the shorter version
is tempting and wrong.

**Why the guard goes missing: one hypothesis offered, refuted by its own author's timestamps,
replaced by a better one.** The first guess was recency — that the rule just learned stays live
and the older one is dropped. The core session tested it against its own commit times rather
than agreeing: `endsRoute` was learned four hours earlier the same day, from this stream, in the
function immediately next door; `flownLeg` was months old. **The recent one vanished and the old
one made the copy — backwards from the prediction.**

What replaced it is structural. `flownLeg` is a filter *inside* the iteration; `endsRoute` is a
truncation *around* the collection, before iteration begins. Reproducing a function's shape —
iterate, filter, fold — carries the guards that live inside that shape and leaves behind the ones
that sit outside it, whenever they were learned. Three for three on the core's side.

This stream cannot test it independently and should not claim to. `route` and `unreached` both
carry `routeEnd`, but only because it was extracted deliberately after the first defect, so they
are the fix and not evidence. What is visible here is that the two positions the hypothesis names
are the two positions these functions actually use: `flownLeg` and `hasPosition` inside
`.filter { }`, `routeEnd` wrapped around the collection in a `.prefix` and a `.dropFirst`.

**The remedy was reached twice from different diagnoses**, which is worth more than either
diagnosis: "the guard outside the shape gets dropped, so make there be one walk" and "two places
that must agree will eventually disagree, so make there be one rule" produce the same fix. That
says the fix is well-founded and says nothing about which explanation is right.

### The raw-property fields are clean, and deliberately have no checker (2026-09-11)

`view-fields.py` covers the keys this head reads out of a `view.*` payload. It does not cover
the fields read from raw Qt property groups — `Bridge.group("plan")`, `"plan.controllerVehicle"`,
`"plan.missionController"`, `"vehicle"` — and the selection regression was exactly a silently
renamed field, so the gap was worth measuring.

Measured against the live bridge rather than reasoned about. Every field is present:

    plan                     5 fields read, 18 live keys, none missing
    plan.controllerVehicle   6 fields read, 141 live keys, none missing
    plan.missionController   3 fields read, 43 live keys, none missing
    vehicle                  2 fields read — unverifiable, no aircraft connected

**No checker was built for this, and the reason is an asymmetry rather than laziness.** A
`view.*` field is invented by the core, renamed by the core, and consumed only by the two heads —
so when one is renamed there is nobody else to notice, which is precisely how `current` went
missing here for hours. A Qt property is upstream, and the QML application reads the same
properties: a rename there breaks QGC's own UI loudly and in the same commit. The failure this
head is exposed to is a quiet rename of a field whose only consumers are heads, and that is what
`view-fields.py` covers.

Two blind spots recorded rather than papered over. `vehicle.*` cannot be checked without an
aircraft, which is the same limit as everywhere else in this stream. And a path assembled by
interpolation — `visualItems.\(index).\(property)` — cannot be read literally; those segments are
pinned separately by `interpolated-names.py` against the `Q_PROPERTY` that answers them.

### The Plan window's action parity, compared and with the method's limit stated (2026-09-11)

Phase 4's gate is this window replacing the QML one, so the open question is whether an operator
can still do what they could. Compared the user-visible actions `PlanView.qml` offers against
what this window exposes.

Every one has an equivalent, with two exceptions that are not gaps:

- **Clear Mission** is absent deliberately. It clears the mission *from the vehicle*, which this
  stream is forbidden from doing, so there is no probe hook capable of it and no button.
- **Coordinates…** looked like a gap and is not. It reads as numeric position entry for an item;
  it is inside the *centre* menu and calls `centerToSpecifiedLocation`, so it centres the map on
  a typed position. This window has it, in the same menu, beside Mission, Launch and Vehicle.

**The item inspector offers one editable fact for a waypoint — Hold — and that matches**, because
the core serves the editable facts and the head draws what it is given rather than deciding.
Position is not among them: an item is moved by dragging, gated on `movable`, which is the same
gate the map uses to decide whether to draw a draggable annotation.

**The limit of this comparison, stated because it would otherwise read as stronger than it is.**
It enumerates `qsTr(...)` strings out of one QML file. That finds menu items and buttons and
will not find an affordance whose label is composed, whose control carries no text, or that
lives in a file `PlanView.qml` merely instantiates. So this is evidence that the actions with
visible labels are covered, and it is not an exhaustive inventory of the QML view's behaviour.

### What the core's four new Qt properties mean here: one adoption declined, one sweep clean (2026-09-11)

`ef98b8fee` reaches four things the reflection bridge could not read before. Checked each
against this head rather than waiting to be told.

**`MissionManager::currentIndex` — reachable now, deliberately not adopted.** It is the item the
vehicle is *executing*, as distinct from the plan editor's selection, and `27cf4ed97` recorded
that no head could read it at all. Taking it would build a capability rather than replace a
derivation, and with no aircraft connected there is no way to see it move, so it would ship
unrendered and unverified. That is the same call already made for `openable`/`blockedReason` on
`view.setup`. Worth having when a vehicle exists; not before.

**`Vehicle::flightModeIds` — the right fix for a class this head turns out not to have.** Mode
names are `tr()` strings, so anything keying on one breaks where it is translated; the ids are
the stable identity. This head compares mode names in exactly two places, `FlyOverlays.keepsGoto`
and its caller in `MapClick`, and both are safe for the reason the rule predicts: the two sides
come from the same table. `Vehicle::flightMode()` calls `_firmwarePlugin->flightMode(base,
custom)`, and `PX4FirmwarePlugin::gotoFlightMode()` returns
`_modeEnumToString.value(AUTO_LOITER)` from the same `_modeEnumToString` — APM's routes through
`guidedFlightMode()` to the same place. They translate together or not at all, which is the
both-sides rule confirming a comparison rather than condemning one.

Writing a mode by name (`vehicle.flightMode`) is the same shape: the name written came from
`view.flightModes`, which came from that table, so it matches what QGC compares it against.

**`additionalTimeDelay` is the core's own arithmetic** and reaches this head only as a figure in
the summary, which the comparison tool already checks.

Checked two levels into QGC rather than stopping at the Q_PROPERTY, because the pair problem is
exactly two same-looking sources that turn out to differ. Noted in passing: `gotoFlightMode` is
declared `CONSTANT`, so it has no notify signal — harmless here because this head reads it per
reload and never watches it, but it belongs on the list of properties a watch cannot bind to.

### The first real parity gap: the per-item leg figures (2026-09-11)

Widening the action comparison from one QML file to all ten `Plan*.qml` — 85 distinct labelled
strings rather than the couple of dozen in `PlanView.qml` alone — found what the narrower pass
could not. The QML item row shows, for each waypoint, the leg that reaches it: **Azimuth,
Gradient, Alt diff, Prev WP, Distance.** This window shows altitude and Hold.

The data is already served. A waypoint carries `azimuth` 60.27, `distance` 14108.57,
`distanceFromStart`, and `altitudeChange` 75.0 — and this head reads none of the four. That is a
real affordance for a planner, who sanity-checks a mission by the heading and length of each leg,
and it is the first genuine feature gap this stream has found rather than explained away.

**It is an ask rather than a local build, and the reason is unit formatting.** Those four arrive
as raw numbers with no text beside them, unlike `altitudeText`/`altitudeUnits` on the same item
and unlike every row of the summary strip. Spelling a distance here means reimplementing
`distance_text(value, imperial)` — the metres-to-kilometres threshold and the operator's unit
preference — in a second place, and drawing the result immediately below a summary strip that
used the core's version. Two implementations of one format, side by side on the same panel, is
the failure this stream has spent the day removing.

`azimuth` is unit-free and could be drawn today; `altitudeChange` could borrow the item's own
`altitudeUnits`. Half the row is not worth shipping ahead of the other half, so the whole row
waits on the core spelling the magnitudes, as it already does for the clearance and for every
summary figure.

**Also worth noting what the field checker cannot see.** `view-fields.py` catches a key this head
reads that the core no longer emits. This is the opposite direction — a key the core emits that
the head never reads — and that direction cannot be checked mechanically, because most served
fields are legitimately unused by any given head. It took reading the QML to notice.

### The fourth walk, and the first one with its margin measured (2026-09-11)

Standing job (A) on the current plan — mission start, takeoff, waypoint, ROI, survey, return to
launch, **and a waypoint after the return to launch**, which is the shape that broke the first
three walks. All three of the core's computed fields now agree with the controller exactly:

| field | controller | computed |
|---|---|---|
| `distanceMetres` | 25383.780544817 | 25383.780544817 |
| `maxTelemetryMetres` | 18327.27166878032 | 18327.27166878032 |
| `altitudeBandMetres` | [585.0, 660.0] | [585.0, 660.0] |

They are null on a plain `view.missionSummary` read and only computed under
`view.missionSummary(verify)`, which is worth knowing before reading a null as a regression.

**What is new is not the agreement but the margin.** Every previous walk asserted that the plan
contained the discriminating case and left it there. Measured this time: the only positioned item
after the route end sits 6424.96 m from the survey's exit, so a walk missing the `endsRoute` guard
would report **31808.74 m against 25383.78 m** — a 25% error, not a rounding difference. The
agreement is therefore evidence rather than a coincidence of a plan that could not have told the
two apart.

**The non-vacuity check was itself vacuous on its first run**, which is the lesson worth keeping.
Measuring the tail from the item *at* the route end took the coordinate of the Return To Launch,
which has none — the haversine returned nothing, the sum filtered it out, and the answer was
**0.00 m**. That reads exactly like "this plan has no discriminating case" and would have retired
a genuine check as uninformative. The fix is to measure from the last item that *has* a position
at or before the route end, the survey's exit. An instrument built to catch a comparison where
both sides answer 0 produced that very answer itself, from an absent coordinate rather than an
absent defect.

Also recorded so the next read does not chase it: the mission probe's `items` are a compact
display form carrying `seq`, `command`, `position`, `altitude` and `selected` — not `sequence`,
`kind`, `endsRoute` or `flownLeg`. Reading it with the view's field names returns `None` for every
one of them and looks like a stripped payload. Those fields are on `view.missionItems`.

### The terrain profile cannot see a vertical leg (2026-09-11)

The core session ported mission time and found the term they were missing: `MissionController`
special-cases a rotor taking off straight up, climbing at the *ascent* speed rather than the
hover speed, and nothing in a horizontal walk accounts for it. Their first version was 16.667 s
short on every plan.

**The same blind spot exists in this window's terrain profile, for the same structural reason.**
Every point in `view.terrainProfile` is placed by a horizontal `x` fraction, and
`TerrainProfileModel.x(_:width:)` uses that fraction directly. A vertical climb has no horizontal
extent, so it occupies zero width: the takeoff draws as a single point at x=0 rather than as the
climb it actually is. The profile shows where the mission goes, not everywhere the aircraft does.

Not fixed, and the honest reason is that it is the least dangerous leg to omit — the climb is
where the aircraft is furthest above terrain, so the one segment the profile cannot draw is also
the one least likely to collide. That is a reason to rank it low, not a reason to call it correct.
Drawing it needs the core to serve the climb as a segment the way it now serves a survey's
transects; raised with them rather than worked around here.

**What is worth taking is their method, not their fix.** They found the missing term by serving
the inputs — the speeds, the vehicle class and the distance — beside the answer under `verify`,
after guessing twice from the difference between two totals and being wrong both times. That is
the structural form of the rule this stream reached from five retractions: reasoning that predicts
the observation is not evidence. Comparing two totals leaves you inferring the gap; serving the
inputs lets you read it. The walks on this side compare totals and quantify the discriminating
margin by hand afterwards, which is the weaker arrangement.

Also carried across: a VTOL answers null for mission time, because `vtolMode` flips between
multirotor and fixed wing as the walk passes a transition item, so a single-speed answer is right
before it and wrong after. This window draws mission time from the core's text, so a null has to
read as *not known for this aircraft* rather than as a blank.

### The macOS build has stopped loading the QML Plan view (2026-09-11)

Phase 4's last unmet criterion. Both peers cleared it first, and both checked rather than
remembered: the Android head never loads `MainWindow.qml` at all — its host is a nine-line
`Item { id: mainWindow }` — and the core session's instruments reach Qt only through
`qgc_bridge_get`/`invoke`, none of whose roots is a QML item.

**Why it is safe, which was the question worth answering before writing any of it.** The bridge's
`plan` root constructs its *own* `PlanMasterController`, with the comment "Native frontends have
no QML view to own a plan controller". The QML `PlanView` holds a second, entirely separate
controller that nothing native has ever read. Had it gone the other way, the gate would have
removed this window's whole data source while leaving every panel drawing stale values.

`QGCCorePlugin` gains a `hostProvidesPlanUI` property, set once by the host before `qgc_start`
from the same argument that installs the native windows, and the `PlanView` in `MainWindow.qml`
becomes a `Loader` gated on it. No QML file is deleted and no other build's tree changes.

**Both paths measured in the same binary**, because a gate that silently does nothing produces a
tree indistinguishable from a healthy one:

| `hostProvidesPlanUI` | live QML nodes | plan chrome |
|---|---|---|
| `true` — native macOS | 107 | **0** |
| `false` — every other build | 132 | **23** |

The false row is the one that matters. Verifying only that the nodes disappear would have left
Linux, Windows and the Qt Android path untested, and a `Loader` that fails to construct its
component looks exactly like a `Loader` that was told not to. The native Plan window is unaffected:
50 of 50 facts still agree with the core, 28 of 30 watched deps still bound, 0 binding to nothing.

**The trap that nearly landed instead.** The first version was `#ifdef QGC_NATIVE_UI` inside
`QGCCorePlugin.cc`. That define is `PRIVATE` to the app target and `QGCCorePlugin` lives in a
library, so it would have compiled to `false`, the `Loader` would have stayed active, and the gate
would have done nothing at all — caught only by checking which target carried the definition, not
by anything the compiler said. The property is `CONSTANT`, so anything trying to set it after
startup is ignored with no warning; it is a host declaration, not a runtime toggle.

**Two suite anomalies, both traced to a running app rather than to this change.** A first full run
gave 705/1/89 on `AircastDeviceSetupTest::_reapplyReplacesExistingLink`; a second died in
`FactSystemTest*`'s `qmlUpdate_test` with a SIGSEGV whose handler re-entered 21,898 times and
filled a 200 MB log. All three suites pass in isolation, and `FactSystemTest` loads
`qrc:unittest/FactSystemTest.qml` — never `MainWindow.qml` or the core plugin — so this change
cannot reach it. With no app running the suite is **706/0/89**, the exact baseline.

That is two dirty runs with an app up against one clean run without, which is suggestive and not
established — the existing note that these flakes occur with or without a running app still
stands, and one clean run does not overturn it. Recorded so the next dirty run starts by asking
what else was running.

**Handed over by the Android session, and worth taking.** `hostProvidesNavigation` and
`hostProvidesGuidedActions` sit two lines above the new property as `readonly property bool … :
false`. `readonly` bound to a literal is a compile-time constant, so no host and no C++ can ever
make them true: their three readers in `GuidedActionRTL.qml`, `FlyViewWidgetLayer.qml` and
`PlanToolBarIndicators.qml` have been evaluating `!false` since they were written. Binding them to
core-plugin properties the way `hostProvidesPlanUI` is bound would take considerably more QML out
of this build than the plan gate did. **Not done here**, because it first needs evidence that the
SwiftUI windows cover navigation and guided actions as completely as they cover planning, and this
window has no vehicle to establish that with.

### The leg row, and two things found by drawing it (2026-09-12)

The parity gap recorded yesterday is closed. The core spells all three figures
(`783187382`), so nothing is formatted here: distance is horizontal, the altitude change is
vertical, an operator can set those units differently, and either formatted in the head would be
a second copy of that choice drawn directly under a strip that used the core's.

**Which items get a row is the part worth reading.** The core serves 0° over 0 m for every item
not reached by a leg, and that cannot be the signal: **due north is a real bearing of exactly 0**,
so reading the zero as absence would hide a leg flown due north. `legs()` is the route's own rule
instead — flown to, placed, before the mission ends — with its first point dropped, because
nothing flies a leg to where the route begins. On a takeoff → waypoint → survey → RTL plan with a
waypoint past the end, that rule independently selects exactly the items the core gave non-zero
figures. Two derivations agreeing, rather than one trusted.

**An unknown mission time was drawn as nothing at all.** The core declines for a VTOL, and the
strip's `if let` made the clock chip disappear. With every surrounding figure still drawn, a
missing chip reads as "I did not look" rather than "not known" — so it now draws an em dash and
says why in its help text, which is the reading the nineteen-fallback sweep found everywhere else
in this head. It needs a VTOL to exercise and there is no vehicle, so the fixture is the whole of
the evidence.

**Standing job (B) on the core's horizontal-unit bug: no twin, because the mechanism was dead.**
`AppUnits` built a `Measure` keyed on `HorizontalDistance` / `VerticalDistance` / `Area` / `Speed`,
and passing the wrong kind for an altitude is precisely the mistake the core just fixed in itself.
Nothing had called it since the core began serving every measure as text beside its unit. Deleted.
Checked with a control rather than trusting a grep that returned nothing: the same search finds
`Measure.reading` in three files, so the search works.

**Reported to the core rather than filtered here: "Cruise 0 m".** On a multirotor the strip shows
it, and it lands in the cell the layout truncates first, so the operator sees `Crui…` — a cell
spending width to say nothing. The vehicle reports `cruiseSpeed: ""`; a multirotor has no cruise
regime, so the row is structurally zero rather than measured. **Not filtered here deliberately**,
because doing so means matching the label `"Cruise"`, and a row label is the core's string and
could be translated. Whether an aircraft has a cruise regime is what the vehicle *is*.

**`commit.sh` has a third case.** It refused the `AppUnits` deletion because `git rm` had staged
it — a deliberate staging of mine read as the shared index being armed against someone. Its own
message covers it, but the way through is to unstage and delete from the working tree so the
script stages the removal itself.

### Reading the QML for served-but-unread fields, enumerated (2026-09-12)

The direction `view-fields.py` cannot check. It catches a key this head reads that the core
dropped; a key the core *serves* that no head reads is invisible to it, and has to stay invisible,
because most served fields are legitimately unused by any given head. Only reading the other
implementation separates "unused because nothing needs it" from "unused because we never built it".

Enumerated rather than sampled this time. The core serves **41 fields per mission item; 15 are
never named anywhere in `macos/Sources`** (control: the same search finds the other 26, so it
works). Of those 15, grepping all QML found which QGC actually draws:

| field | QML references | what QGC does with it |
|---|---|---|
| `distanceFromStart` | 4 | **positions a tick per item along the terrain profile** |
| `abbreviation` | 7 | labels those ticks, and the fly view's mission status bar |
| `specifiesCoordinate` | 14 | gates item-editor actions |
| `exitCoordinate` | 6 | structure-scan map handles |
| `azimuth` | 11 | the toolbar readout this window now draws from `azimuthText` |
| `incomplete`, `edited` | 1 each | comments only, not drawn |
| the remaining 8 | 0 | raw numbers whose `*Text` variants this head already draws |

**The payload was the first two.** `TerrainStatus.qml` draws a numbered, clickable tick per item
along the profile. This panel drew ground, route and collision runs with nothing to relate them
to — so it said "below terrain by up to 797 m" and left the operator to find *where* by eye, which
is the question the panel exists to answer. The earlier UX review asked exactly this and could not
resolve it. Now drawn, and the collision bar visibly begins at the survey.

**It needed no new field.** The core already stamps every profile sample with its `sequence`, so
the marks come from the profile itself. QGC positions the same ticks from `distanceFromStart +
complexDistance` — a second measurement of the same thing, which is the arrangement where two
numbers drift apart. Worth noting as a case where the ported version is better placed than the
original, rather than merely equivalent.

**The eight raw numbers are correctly unread and get no action.** `azimuth`, `altitudeChange`,
`distanceFromStart` as a number, `altitudeAmsl` and friends are the unformatted twins of text this
head already draws, and reading them would mean formatting in the head — the thing the leg row was
built to avoid. Being unread is the correct state for them, which is precisely why this direction
cannot be mechanised.

### The served-but-unread sweep across every view, and it is clean (2026-09-12)

Ran the `view.missionItems` method over all eighteen served views: enumerate what the core emits,
subtract every name appearing anywhere in `macos/Sources`, then grep all QML for what remains.
Control in the same run — 1021 names found in the head, where a broken read gives 0.

**The plan views are clean.** Nothing survived that this window should draw:

| view | served | unread | verdict |
|---|---|---|---|
| `missionSummary` | 22 | 15 | the `*Computed*` / `*Inputs*` verify scaffolding, plus raw metres whose text this head draws |
| `terrainProfile` | 20 | 1 | `totalDistanceMeters`, the raw twin of `distanceText` |
| `surveyStats` | 18 | 8 | every one a raw twin (`areaSquareMetres`/`areaText`) or an input beside an answer |
| `fences`, `camera`, `inspector`, `sensors`, `setup`, `vibration`, `video` | — | **0** | nothing unread at all |
| `missionKinds` | 14 | 1 | `atSequence` — where an insert would land |
| `altitudeModes` | 9 | 2 | `context`, `supportsTerrainFrame` |

**The one that looked like a defect and was not.** QGC removes the Terrain Frame altitude mode
from the picker when the vehicle cannot hold one — `MissionSettingsEditor.qml:105` and
`SimpleItemEditor.qml:134` both do it — and this head does no such filtering. But the core already
does: `altitudemodes.rs:37` hides that mode and `:51` supplies the reason, so the served list
arrives filtered and `supportsTerrainFrame` rides along as evidence. Confirmed by reading the crate
rather than inferring from the payload, because a mode absent from one plan's list and a mode the
producer removes look identical from outside.

**The QML-reference count is a weak signal and was treated as one.** `highest`, `total`,
`context`, `dynamic` all matched QML lines about entirely different objects. Every flagged field
was read at its hit before being believed; the count only decided what to look at.

**A sweep finding nothing is a result here, because the same method found the terrain markers an
hour earlier** on the first view it was run against. What remains unread is unread for two good
reasons — a raw number whose formatted twin is already drawn, and an input served beside its answer
so a disagreement can be read instead of inferred. Both are states this stream deliberately wants.

The remaining unread fields sit in views belonging to other windows and mostly need a vehicle:
`enoughChannels` / `liveChannels` / `minimumChannels` on `radio`, `waitingForCancel` and the two
`*Needed` flags on `calibration`, `everyday` / `folded` / `currentSummary` on `flightModes`,
`dynamic` on `links`, `anyDownloaded` on `logs`. Recorded, not chased.

### The null-verdict finding, refuted by looking (2026-09-12)

`missionkinds.rs` serves `enabled: null` and `disabledReason: null` when the plan view has
selected nothing, deliberately — *"what can be inserted depends on where, so an answer with no
where is not an answer"* — and its test warns that a head reading those would take the
controller's constructor values as a verdict. `MissionItemKind.swift` turns a missing `enabled`
into `false`, and `PlanWindow.swift` draws `.disabled(!kind.enabled)` with
`.help(kind.disabledReason ?? "")`. Read together that says: the Add menu greys out every kind
with an empty tooltip.

**It does not happen, because the state is not reachable.** On a fresh plan the core answers
`selected: 0`, `atSequence: 0`, and every kind carries a real boolean beside a real sentence —
*"This mission starts from the ground, so a takeoff has to come before anything else."* The plan
always holds its settings entry and that entry is always selected, so the core always has a
*where*. Dropped.

Worth keeping for the method rather than the result. The reasoning was sound at every step and
predicted a defect that is not there; only reading the live view settled it. **The one piece of
evidence that looked supporting was not evidence at all** — an earlier capture of the mission
probe showed a `kinds` array carrying `reason`, not `disabledReason`, because the probe serves its
own compact projection with renamed and dropped fields. A projection is not the view, and treating
it as one would have produced a confident finding about a payload that does not exist.

### Three unit literals the head still spelled (2026-09-12)

The core swept itself for unit literals and found two (`26a01d10a`), with the tell recorded:
*"reading the screens never found them because each was right in the units under test; grepping
for unit literals did."* Ran the same sweep here — 13 hits, control confirming the search works.

Ten are `?? "m"` fallbacks beside a unit the core serves, which is the established pattern.
**Three were hardcoded**: the default item altitude and both offline speeds, drawn `units: "m"`
and `units: "m/s"` regardless of what the operator set. All three come from Qt settings facts, and
a Fact carries `units` — `FenceRally`, `LaunchPosition` and `ItemSpeed` all already read it. These
three were simply never wired.

**The control matters more than the change here**, because on a metric build reading the fact and
hardcoding `"m"` produce identical output — the null result and the healthy one are the same
pixels. Replacing both fallbacks with `"ZZ"` and rebuilding: the panel still read **75.0 m** and
**5.00 m/s**, so the unit came from the fact. A rename of a hardcode would have shown `ZZ`.

The imperial path itself cannot be exercised: proving it renders feet needs a settings fact
written, which is forbidden here. What is proven is the source of the string, not its behaviour
under a setting this session may not change.

### Absence has six causes, and four of them were drawn as values in one night (2026-09-12)

The organising idea of the night, reached from three sessions independently. A reader given a
value cannot tell which of these produced it unless the producer distinguishes them:

1. **A measured value.**
2. **A figure the producer could not work out** — mission time on a VTOL, where `vtolMode` flips
   at a transition item so a single-speed answer is right before it and wrong after.
3. **A regime that does not apply** — Cruise on a multirotor, an altitude mode on a mission start
   entry. Absent, not zero.
4. **A verdict the producer declined to give** — `enabled: null` on a kind with no insertion
   point. Reading it as `false` would refuse everything.
5. **A member the producer filtered out** — Terrain Frame on a vehicle that cannot hold one. The
   absence carries no evidence at all unless the producer publishes why.
6. **A value that resolved to something meaningless** — and this one has *no tell*. The others can
   at least be detected.

The sixth is the core session's phrasing and the sharpest thing said here: *"my fix was checked
against command metadata and a unit test, both of which said it was correct, and it was correct
about the thing they tested. What neither could see was that the value it admitted was not the
quantity it claimed to be."* Four instances in one night — `amslEntryAlt` returning a relative
height on an AMSL axis, a `DO_` command's unused `param7` read as an altitude, the leg figures
served as zeros before the walk ran, and a refused mode dropped without its reason. **A leg of 0 m
and an altitude of 0.0 m are both real possible values.**

**A seventh shape, hit by two sessions in the same hour: a flag absent from a type reads exactly
like a flag set false on it.** The core nearly blanked the launch row and every survey by gating on
bare `specifiesAltitude`, which `SimpleMissionItem` declares and the other item types do not. This
window nearly blanked `Mission Start` the same way — it reports `specifiesAltitude: false` and its
585 m is a real planned home altitude. Same shape as `groundKnown` meaning *every* sample rather
than *any*.

**The audit that followed.** Every list this head filters, twelve of them. One was wrong:
`AltitudeMode.choosable` dropped a refused mode and kept its reason for an attempted write.
Eleven were right, and what makes them right is uniform — the producer's reason travels with the
member. The Add menu and the pattern menu draw a refused kind disabled with the core's sentence as
its help; the import list intersects two lists from the same producer; `extraRows` drops only
figures already on the strip; the geometry filters require a polygon to have three points.

**What this changes about method.** Every one of these was found by rendering the panel and
reading it, or by a peer saying "if your head does X, check it" — none by a test either side owns.
A test asserts that a value equals what the code produces; it cannot ask whether the quantity means
anything. That question is only answerable by looking, or by the producer publishing what it knew.

### The setup page list is firmware-independent and the pages are not (2026-09-12)

Turning the plan window's methods on the Setup window. Its sidebar drops a page the head cannot
draw — `SetupPage.draws` is `bespoke.contains(name) || page.parameterSections` — and on this
ArduPilot vehicle exactly one of the fourteen served pages is dropped: **Flight Behavior**.

**That is correct, and it is correct by accident.** `PX4FlightBehavior` lives entirely under
`src/AutoPilotPlugins/PX4/` and is constructed only by `PX4AutoPilotPlugin`; QGC has no ArduPilot
equivalent, and `sections_for` in the core already knows this — `("Flight Behavior", true)` is the
only arm that matches it. But `PAGES`, the list the core publishes, is firmware-independent. So on
ArduPilot the core offers a page QGC itself does not have, and every head must silently drop it.
Two decisions cancelling rather than one decision made.

**The mirror is the part that matters, and it is this head's.** The same check the other way:

| page | PX4 files in QGC | APM files in QGC |
|---|---|---|
| Flight Behavior | 3 | **0** |
| Frame | **0** | 5 |
| Lights | **0** | 5 |

`SetupPage.bespoke` lists Frame and Lights unconditionally, so on a PX4 vehicle this window would
draw two pages QGC offers only on ArduPilot — and unlike Flight Behavior nothing would drop them,
because a bespoke name passes `draws` without asking the core anything. The accident that protects
the first case does not protect this one.

**Unverifiable here, and that is the whole reason it is recorded rather than fixed.** The SITL is
ArduPilot, there is no PX4 vehicle, and guessing which of the two bespoke views degrades gracefully
is exactly the reasoning that has been refuted twice tonight. The core's `PAGES` being
firmware-dependent — as `sections_for` already is — fixes both directions at the source and needs
no head to know which pages belong to which firmware. Raised with them.

Worth noting what this method found and what it could not. Counting served pages against drawn ones
is cheap and gave a precise answer for the firmware in front of me; it says nothing at all about the
firmware that is not.

### Android's "null" trap has no Swift twin, checked with a control (2026-09-12)

Android found their head drawing `Change speed · null` the moment the core began withholding a
figure, because `org.json`'s `optString` returns the string `"null"` for a JSON null (`a0b7caa`).
Standing job (B): the same question here.

**It does not happen.** Every decoder in this head reads through `as? String`, which rejects
`NSNull` and falls to its own default. Checked twice rather than by an empty grep:

- Every interpolation of a raw payload value — fifteen of them — is a probe error string with
  `?? ""`, and none reaches a drawn surface. Control: 245 interpolations found in total, so the
  search works.
- Zero compiler warnings of the class `string interpolation produces a debug description for an
  optional value`. **Control: adding one deliberately produced the warning at the expected line,
  and removing it returned the count to zero** — without which "no warnings" would have been
  indistinguishable from warnings never being reported, which is what the first run actually
  looked like, since the only warnings in that build came from `install_name_tool`.

Their note that this is uncatchable in their unit tests — the JVM `org.json` returns `""` for the
same call, so a test asserting the device behaviour fails on the JVM — has no analogue here, but
the general shape does: **a decoder's behaviour on a value the producer has only just started
sending is not something either side's tests were written against.**

### Android's index-versus-sequence defect has no twin here, and the reason is structural (2026-09-12)

Android found one screen showing three different numbers for one waypoint — `#6` in the summary
chip, `Delete #6` on the button, `Adding after #72` by the toolbar — because `index` counts a
complex item as one and `sequence` counts every item a survey expands to. They agree exactly until
a complex item is in the plan. The dangerous half was the Delete button naming a real marker it
would not have deleted.

**Checked here and clean, but the interesting part is why.** Every one of the 23 `.index`
interpolations in this head is a **bridge path** — `plan.missionController.visualItems.\(index)…`
— which is the correct use, because `visualItems` is addressed by list position. Every
operator-facing label reads `.sequence`: the blocked banner, the map annotation title, and the list
badge. Three call sites, one field. Android's three call sites used two fields.

So the split is not a convention anyone remembered to follow; it falls out of the two names meaning
two different things in the only two places they are used. Nothing here would have caught the
mistake if it had been made — the protection is that `index` never leaves a path string.

**Measured on the discriminating case**, because index and sequence agree on every plan without a
complex item and a check on one of those proves nothing:

| index | sequence | name |
|---|---|---|
| 0 | 0 | Mission Start |
| 1 | 1 | Takeoff |
| 2 | 2 | Survey |
| **3** | **144** | Waypoint |

The badge on that row draws **144**, and the terrain profile's marks read 0–1, 2, 144. A difference
of 141 between the two candidate answers, so the observation separates them.

### The remaining list filters, and what makes one right (2026-09-12)

Finishing the audit begun on the plan side.

- **`GuidedOffer.shown`** is `offer != "hidden"`, and `blocked` is `shown && !ready`. The producer
  decides what is absent; a refused action stays visible and keeps its reason. Correct shape.
  Unobservable here — `connected: false`, so the offer list is empty without a vehicle.
- **`FlightModeChoice.everyday` / `.folded`** is a partition, not a filter: every mode appears in
  one or the other, and the folded ones sit behind "More modes". Nothing is dropped.
- **`SetupPage.draws`** is examined above — it drops what this head cannot draw, and the one page
  it drops on ArduPilot is one QGC does not have for that firmware.

Which leaves the rule intact across every list either window draws: **the producer decides what is
absent, and a member it refuses keeps its reason.** `AltitudeMode.choosable` was the only place
that took a refusal and threw the reason away.

### Correcting the entry above: Frame is on both firmwares, and I counted files instead of reading the list

The Android session checked the setup finding and the naming half of it was wrong. **PX4 has a
frame page; it is called Airframe.** `PX4AutoPilotPlugin.cc:66` constructs `AirframeComponent`,
`APMAutoPilotPlugin.cc:61` constructs `APMAirframeComponent`. Gating Frame would have removed a
page PX4 operators need.

**The method was the error, not the arithmetic.** I counted files matching a case-sensitive glob
for the page name. `Airframe` does not contain `Frame`, so the PX4 half was invisible, and the one
"PX4 Camera file" the count did find is `Images/CameraTrigger.svg` — an icon. A filename is a weak
signal in exactly the way a QML name match is, and the rule that covers it was already written
down: *read the hit*. The list that decides is the set of components each plugin constructs, and
reading that takes one grep:

| | |
|---|---|
| **APM only** | Camera, Follow, Heli, **Lights**, **RemoteSupport**, SubFrame |
| **PX4 only** | Actuator, Syslink |
| **both** | **Airframe**, ESP8266, FlightModes, Motor, Power, Radio, Safety, Sensors, **Tuning** |

**So the finding stands and its scope changes.** `SetupPage.bespoke` lists thirteen pages, and
three of them — **Camera, Lights and Remote Support** — exist only on ArduPilot. Not Frame, and not
Tuning. One of the three I named was right, one was wrong, and two I had not looked for.

Android adds a consequence this window shares: their setup screen falls through to *"<page> is set
up on the desktop"* when the core reports no parameter sections, which is **true** for a page the
desktop has and **false** for Camera and Lights, where it points an operator at something QGC does
not offer for their firmware at all.

Still not fixed here, for the reason that has not changed: the asymmetry is that `PAGES` in
`setup.rs` is a flat constant while `sections_for` directly beneath it already takes a `px4` flag,
and no head can test the other firmware. The core's comment refusing to answer *"does a head have a
screen for this page"* remains right — but *"does this firmware offer this page"* is vehicle state,
which is theirs by the same rule.

### The APM regression the core asked for, and why `omitted` is deliberately not drawn (2026-09-12)

The core's `b5019228b` filters `view.setup`'s pages by firmware. Nobody has a PX4 vehicle, so the
only check either session can make is that **no ArduPilot page disappeared**. Run here:

    served now (13): Summary, Sensors, Radio, Frame, Flight Modes, Safety, Power, Motors,
                     Tuning, Camera, Lights, Remote Support, Parameters
    disappeared:     Flight Behavior only
    newly present:   none

Exactly the fourteen recorded before, minus the one PX4-only page. **The check is meaningful
despite there being no vehicle**: `px4` is read from `vehicle.px4Firmware`, which is false when
nothing is connected, so the filter took the ArduPilot branch. What it cannot speak to is the PX4
branch, which remains unwatched by anyone.

**A correction to my own first reading of it.** I reported `omitted` as null. It is not — it is
served *per group*, beside each group's `pages`, and I looked for it at the top level. The same
mistake as reading a probe projection for a view: I checked the shape I expected rather than the
shape that exists. Read properly it carries a real sentence: *"This is a PX4 setup screen.
ArduPilot firmware has no equivalent."*

**And it is deliberately not drawn, which needs saying because it looks like the rule being
broken.** The rule is that a member the producer refuses keeps its reason — applied twice tonight,
to the altitude modes and the camera list. This is the other case. The distinction the camera work
established is between **Camera 2**, which the operator switched on and which cannot work, and
**Camera 4**, which was never asked for and is correctly absent. A PX4 setup page on an ArduPilot
vehicle is Camera 4: nobody asked for it, nothing is missing from the operator's point of view, and
a permanent row reading *"Flight Behavior — PX4 only"* is noise on every ArduPilot flight forever.
The reason exists for a head that wants it; this one does not.

**The core's change also closes an item that was open against this head.** `SetupPage.bespoke`
lists Camera, Lights and Remote Support unconditionally, and on a PX4 vehicle that would have drawn
three ArduPilot-only pages. It cannot now: `draws` is only ever applied to pages `view.setup`
serves, and those three are filtered out upstream before the head sees them. The fix at the
producer removed the need for the head to know anything — which was the argument for putting it
there.

### The two reachability predicates, and what each one cost (2026-09-12)

Two questions that no check in this stream had been asking, both of them about whether a value can
be experienced rather than whether it is right.

**Does any path draw this value in the state that produces it?** Predicate: a computed `String`
property whose body branches on a state flag. Nine hits, eight explained, one real — and the real
one was in the instrument, not on the screen. `MotorTest.countWarning` answered *"The vehicle has
not said how many motors it has"* whenever the count was unknown, which includes having no vehicle;
`MotorsView` wraps itself in `SetupPageBody(connected:)`, so the page draws *"Connect a vehicle to
set this up."* and the sentence was unreachable. `Motors.swift` put it in the probe regardless.
`d9e3b6b6f` gates it, so the probe and the screen now agree. **A projection can be faithful to the
code and still describe something nobody can experience**, and neither a mutation nor the
`delivered`-versus-`bindings` distinction reaches that.

**Does a reachable state draw nothing where it should speak?** The first attempt — every draw site
whose content can be `""` — returned 84 hits and was unreadable, which is the trawl rather than the
search. One refinement made it triageable: **only the VALUE half of a label/value pair**, because
an empty `Text` collapses invisibly while a label with nothing beside it is a hole with no
explanation. 84 to 4.

All four explain. `link.portText` is emptyable on purpose (`8edb642a7`) and sits in an editable
field, where empty is what an operator types into and a zero would be a rate someone chose.
`link.name` and `link.host` are the same shape, and measured: of 140 configured links **0 have an
empty name**, and the 3 with an empty host draw *"No host set"* in the summary the core owns, so
the blank field is the affordance and not the message. `mode.summary` needs a vehicle.

**The predicates are the artifact; the results are not.** Eight of the nine explanations in the
first sweep and all four in the second are statements about today's code, and they rot. The queries
do not. Recorded here for that reason, and because the second one took two attempts — the version
that returns 84 hits looks like a finished search and is one refinement short of one.

### instrumentQmlFile2 measured rather than predicted, and it is an enum (2026-09-12)

The entry above calls this "the awkward one" and says Phase 6 would leave a persisted setting
naming files that no longer exist. Both halves are now measured rather than reasoned, and the
measurement narrows the work.

**A non-default value is persisted in the shared QSettings space right now.** `rawValue` is
`VerticalCompassAttitude.qml` and `defaultValueString` is `IntegratedCompassAttitude.qml`, so the
trap is armed on this machine today rather than hypothetically on someone's later. Anything that
deletes the three widgets inherits a live dangling path, not a defaulted one.

**But the fact is an enum and carries its own names.** `enumStrings` is
`["Integrated Compass & Attitude", "Horizontal Compass & Attitude", "Large Vertical"]`,
`enumValues` holds the three qrc paths, and `enumIndex` is `2`. The earlier entry's prescription --
*"porting it means offering the choice by name, never by file"* -- needs nothing built to become
possible: the names are already in the payload beside the index.

**That changes the shape of the migration rather than removing it.** A head that selects by
`enumIndex` never touches a path, so deleting the widgets leaves `rawValue` naming something gone
while `enumIndex` stays correct. The open question is no longer "rewrite a persisted string" but
the narrower "does anything break when the value is dead and the index is live", which is a
question for whoever deletes the files and can be answered then.

**Nothing to do here today, and that is the point of writing it down.** No Swift in
`macos/Sources` reads `instrumentQmlFile2` -- zero matches, against a control showing the head
does read two other `flyViewSettings` facts -- so this head draws no such control and has no
defect. The item stays open for Phase 6 with a measurement attached instead of a prediction.

### The two Phase 6 decision items, measured, and they answer oppositely (2026-09-12)

`instrumentQmlFile2` and `Viewer3D` have sat on the open list as the two "port it or record the
non-port" decisions. Both had an analysis and neither had a reading. Measured now, and the
readings point in opposite directions, which changes the order they should be taken in.

**`instrumentQmlFile2` is armed.** `rawValue` is `VerticalCompassAttitude.qml` against a default of
`IntegratedCompassAttitude.qml` -- a non-default value is persisted in the shared QSettings space
on this machine today, so deleting the three widgets strands live user state. Recorded in
`f2aa25654` along with the finding that the fact is an enum and already carries the names a head
would key on.

**`Viewer3D` is dormant, and every fact says so.** `enabled` is `false` against a default of
`false`; `osmFilePath` still reads `"Please select an OSM file"`, which is its default placeholder,
so no file has ever been chosen; `buildingLevelHeight` is 3 against a default of 3. Nothing in the
group has been touched. The head reads four settings groups -- `appSettings`, `flyViewSettings`,
`mavlinkSettings`, `videoSettings` -- and `viewer3DSettings` is not among them, nor is it a served
view, so `view-fields.py` would never have reported it.

**What that changes.** The two items are the same shape on paper and not in practice: one deletion
strands a persisted value and needs a migration, the other strands nothing because no user state
exists to strand. **Enabling `Viewer3D` also is not something an operator drifts into** -- the
default is off and it additionally requires deliberately choosing an OSM file, so the placeholder
being untouched is a stronger signal than the boolean alone.

**The limit of this, stated.** These are one machine's settings. A user elsewhere may have enabled
`Viewer3D` or left `instrumentQmlFile2` at its default, and nothing here measures that. The claim
is about what is persisted *here*, which is a data point the decision did not previously have --
not a claim about the population.

### Idle memory is flat and idle CPU is not the figure in this file (2026-09-12)

Phase 5's gate is *"a 30-minute flight with flat memory"* and nobody had measured the flat part.
Measured now with four windows open, no vehicle connected, the app otherwise untouched.

**Memory is flat within noise, and a short series said otherwise.** The first six samples over 100
seconds ran 217.8 to 221.2 MB and looked like ~2 MB a minute, which extrapolates to 60 MB over the
gate's half hour. Fourteen samples over 390 seconds say it is noise: the range is 197.6 to 226.3
MB with no monotonic trend, the first-half mean is 219.5 against a second-half mean of 221.1 --
a 1.6 MB difference inside a 28.7 MB spread -- and **the final sample is the lowest of the
fourteen**, which no growth story survives. **A trend needs a series longer than the thing it is
trending against**, and the 100-second version would have been reported as a leak.

**Idle CPU is 38.8% mean over those 390 seconds, range 25.5 to 47.2.** The entry in this file
attributing ~21% to a GStreamer pipeline is either stale or was measured under different
conditions; this is not a regression claim, because the earlier reading's conditions are not
recorded and cannot now be reconstructed.

**The attribution is unfinished and here is the test that would settle it.** The head runs around
twenty pollers at 0.5 to 1 second, but only while a window is open; the video pipeline retries
independently, and `view.video` currently reports `anyConnecting: true` with cameras 1 and 3
dialling a stream that is not there. **Closing every window and re-measuring separates them** -- if
the figure collapses the pollers dominate, if it holds the pipeline does. That was not run here
because the macOS window menu offers Minimize and Zoom and no Close, and nothing in the probe set
closes a window.

**What this establishes and what it does not.** It establishes that idle memory is flat on this
machine over six and a half minutes, which is the first half of the gate and was previously
unmeasured. It does not establish anything about memory under a connected vehicle, over thirty
minutes, or on any other machine.

### A core null that reaches the screen as a zero, 2026-09-12

`view-fields.py` catches a key the core stopped emitting. Its opposite is quieter:
the key is still there, its **value** is null, and the head's `?? 0` turns an
absence into a figure. `a9b4e2a53` made a link's port and baud nullable because
`port` is a Q_PROPERTY on `TCPLink` and `baud` on `SerialLink` alone; this head
went on drawing a baud rate of 0 for a UDP link until `8edb642a7`. Nothing would
have found the next one.

`tools/macos/null-fallbacks.py` reads the nineteen mapped views **live** and asks
what each model does with the keys that came back null. Reading the payload rather
than the core's source settles the question the source cannot answer — whether an
`Option` is actually `None` in a state this machine reaches — and costs the other:
**a field null only in an unreachable state is invisible. It under-reports and
never invents.**

**Control: the pre-fix `LinkConfigModel.swift` from `8edb642a7^` is reported.**
`baud` fires; `port` does not, because the live links are UDP and it reads
`localPort` — one of the pair is enough, and saying which is the point.

**Today: four hits, all the launch row, all benign.** `command`, `category`,
`specifiesAltitude` and `altitudeBandText` are null on the settings item alone.
Every consumer is gated to exclude it: `canChangeCommand` is
`isSimpleItem && sequence > 0 && !isLaunch`, and `altitudeReading` names
`settingsKind` outright rather than leaning on the flag. Rendered, the selected
launch row draws `491 m` as plain text with no chevron and no trash — the gates
are what the screen shows, not only what the source says.

The four are an `ACCEPTED` table with the reason each is safe, so the steady state
is **zero**, not four hits nobody reads. **An accept-list is the failure mode of an
accept-list**, so a stale entry is reported: three mutations fire — dropping an
acceptance surfaces the hit, naming a model the file does not check is caught, and
an acceptance the head no longer falls back on is caught.

**One mutation was vacuous and nearly passed.** Run from the scratchpad, the copied
script's `parents[2]` resolved outside the repo, it found no models at all, and
reported nothing — which reads exactly like a table that is not load-bearing. Redone
in place it fires. **A mutant that cannot reach the code proves the same thing as a
mutant that is not needed: nothing.**

`MODELS` now lives in `tools/macos/head_models.py` and both checkers import it. It
was written out twice, which is the drift these two exist to catch.

**Refuted, not a defect.** `writeFailure` looked stale — the probe showed a refusal
message while the plan behind it had grown by the item the message refused. It
drives a modal alert that clears on dismissal; only the probe, which bypasses the
UI, can reach a stale one. **An instrument faithful to the model, reporting
something no operator can experience.**

### An item that commands its own speed says so in the list, 2026-09-12

Found by asking the question `view-fields.py` does not: not "does the head read a
key the core dropped" but **"does the core serve a key the head reads nowhere"**.
Sixty-nine such fields across the nineteen mapped views. Most are the envelope's
`class` tag, the raw metric behind a `*Text` the head does draw, or a term
`head-vs-core.py` diffs. **One was a screen this head never built.**

`speedChangeText` — `specifiedFlightSpeed` through the operator's speed setting, so
`23.3 kn` where that is the setting, withheld entirely when the commanded speed is
zero because `"0.0 m/s"` reads as an instruction to stop. The core's own test says
it exists so that **a head spelling it itself would not rebuild the defect the
obstacle label was**. `grep speedChange macos/Sources` returned nothing at all.

The figure was not missing from the app — it was in the **detail panel for the
selected item only**. So reviewing a thirty-item plan for speed changes meant
selecting all thirty. **A value being right is not the same as a value being
reachable**, and this is that rule about a whole screen rather than a field.

The subtitle is one line and already a precedence chain, so the third term joins it
by urgency: **a block is a task, never being flown to is a defect, a speed change is
information.** That ordering now lives in `MissionItem.subtitle(unreached:)` rather
than at the call site — `PlanWindow.swift` is not compiled by `swift-checks.sh`, so
a rule written there is unpinnable however carefully. Five assertions, including
both losing branches; proven to run by making one deliberately false.

Rendered: row 2 reads **Waypoint / Flies at 12.0 m/s**, row 4 commands none and says
nothing, the Takeoff keeps **Set its location**. Suite 707/0/89 with no app running,
load average 8.34 during the run and neither known flake appeared.

**It also explains an artefact that was on screen with no explanation.** The rows
number 0, 1, 2, **4** — a per-item speed emits its own `DO_CHANGE_SPEED`, which
takes sequence 3. Before this the operator saw a gap in the numbering with nothing
on screen accounting for it.

**The predicate has a control and is NOT shipped as an instrument.** Pointed at
`07d9e8df9^`, where this head did not yet read `altitudeBandText`, it reports that
field — non-vacuous. But it reports it as **one line among twenty**, and a checker
whose signal is indistinguishable from its noise is one nobody reads. Shipping it
needs an accepted table of about twenty reasons, and **I have verified five**
(`bandText`, `abbreviation`, `positionText`, `incomplete`/`edited`, `altitudeOnly`).
Writing the other fifteen from belief is the failure this whole file is about, so
the table is the next cycle's work, not this one's.

### An item says everything it does, not the one thing that ranked highest, 2026-09-12

The second field the unread sweep found. `extraSeconds` is `additionalTimeDelay`
— how long a waypoint waits on arrival — and `grep extraSeconds macos/Sources`
returned nothing. Same shape as `speedChangeText`: the value was **editable** in
the detail panel as a fact named Hold with units `secs`, and **invisible while
scanning**. Two fields, one pattern: the Plan window could edit things its list
would not mention.

The core serves this one **raw**, and says why: seconds have no unit preference,
so there is nothing to convert and the spelling is the head's. A commanded speed
goes through `Unit::speed` because QGC has a Speed setting; a delay does not.

**Both are true at once, so neither ranks.** A block and a speed compete for one
line — a block is a task and wins. A hold and a speed do not compete; they are two
facts about the same item, and choosing between them drops one. The subtitle now
reads `blockedReason ?? unreached ?? doings ?? ""`, where `doings` joins whatever
the item actually does.

**The render is what found the real defect.** With both, the row read
**"Flies at 8.0 m/s · H…"** — truncated, and truncated *only when selected*,
because the chevron and trash take the width. **The item doing the most was the
one losing information, in exactly the state an operator is in while editing it.**
`GroupRow` already had `descriptionLines`, and `fixedSize` keeps a one-line row one
line tall, so a single argument fixes it: row 3 wraps to two lines, row 2 does not
grow. Nothing a fixture could have caught — only the screen.

**A robustness fix the tests pin:** `String(Int(seconds))` traps past `Int.max`, so
a payload nobody expects would take down the plan list rather than draw a silly row.
`String(format: "%.0f")` cannot trap, and the assertion spells the 31-digit value
of 1e30 exactly — **a wrong guess would have failed, so its passing is proof it
ran**, which is stronger than the mutation I ran on the neighbouring assertion.

Suite 707/0/89, no app running. **`cmake --build | grep -c 'error:'` returning 0 is
NOT the build succeeding** — rule 27, and the core lost a cycle to exactly this:
their build exited 1, the app copy silently kept the previous binary, and the suite
came back green. This build's exit status was checked directly and is 0.

**Android had already run this sweep on the item view before me** — 43 keys emitted,
19 never read, "two of them things an item does that its row never said". The two
were these two. `speedChangeText` exists at all because they asked for it. **The
same sweep, run independently against two heads, found the same pair.**

**Checked, and my head was already right:** Android stopped hardcoding
`splitPolygonSegment`/`splitSegment` and takes the name from the core's
`splitInvokable`. Mine already does — `Bridge.invoke("\(path).\(splitInvokable)")`;
the local `splitSegment` is a Swift function name, not a bridge string.

### A route leaves a pattern where the pattern ends, 2026-09-12

The third field the unread sweep found, and the first that was wrong on the **map**
rather than missing from a list. `exitCoordinate` was served and named nowhere.

A survey is entered at one corner and left at another. On a real one built here the
two are **411 m apart**. The head drew the route polyline from each item's entry
coordinate only, so the leg from the survey to the next waypoint started at the
corner the vehicle went *in* by — **the drawn route doubles back across the ground
the survey has just covered.** That is the core's own phrasing, in the test that
accompanies the field: they built it for exactly this defect and no head was using it.

The core filters the field rather than making each head dedupe: `exitCoordinate` is
null when it equals the entry, because "drawing a second point there is a point on
top of a point". The head appends what arrives and trusts that filter.

`MissionItem.routePoints` now yields entry, then exit where one exists.
`MissionMap` maps the pair; the rule lives in a `*Model*.swift` so `swift-checks.sh`
compiles it. Four assertions including the ORDER — a reversed pair draws the leg
backwards through the pattern — and the mutation that reverts to entry-only fails
exactly the two that are about the exit.

**`native_screenshot` cannot see MapKit (control 4), so the line itself was not
looked at.** Instead the probe now projects `routePoints`, and it was measured in
both states in the same binary: three items with no pattern give **2** points;
adding a survey gives **5**, so the survey contributes two. That is the coordinate
list the map consumes, one step short of the drawn line, and the limit is stated
rather than papered over.

**The sweep taught me something about itself.** Its first run saw 52 fields on the
item view; with a survey in the plan it sees 60. `exitCoordinate`, `patternDistance`
and `foldedCommands` are only present on a complex item, so **a plan of plain
waypoints cannot show the fields only a pattern carries.** Rule 38 applied to my own
instrument: the corpus depends on the plan I happened to build.

**Reasons measured this cycle, for the table:** `azimuth` → `azimuthText` drawn;
`altitudeChange` → `altitudeChangeText` drawn; `altitudeAmslLowest`/`Highest` → feed
`altitudeBandText`, drawn — all raw numbers behind a text the head already shows.
`patternDistance` is `complexDistance`, which the survey card draws through
`view.surveyStats`. `altitudeMode` IS used, but read from the controller path rather
than from the view. `distanceFromStart` has no text sibling at all: it is an input to
the core's own terrain walk, not a figure built for a screen.

### An item says how many commands it folds, 2026-09-12

The rows numbered 0, 1, 2, **144**, and nothing on screen accounted for the 141 in
between. `foldedCommands` is `lastSequenceNumber - sequenceNumber`, and the core
served it (`4457e5ef5`) after watching both heads report the symptom to each other.
Measured here: a survey folds **141**, a corridor **11**, and a waypoint that
commands a speed folds **1** — the `DO_CHANGE_SPEED` itself. So one field explains
every hole, including the commonest one.

Null and zero stay distinct: the core sends null when an item never reported a last
sequence, rather than claiming it folds none, and the head is silent for both but
for different reasons.

**The render found the same defect one term further along.** Three doings —
`Flies at 9.0 m/s · Holds for 20 s · 1 m…` — truncated at two lines, in the selected
state again. Android hit the identical thing on their own row and put it best: **a
six-character label failing is the row telling you it is over-full, not the label
telling you it is over-long.** `descriptionLines: 3`; `fixedSize` keeps every shorter
row its old height. Rendered again after the fix, not just before it.

**Android drew the same field differently, and theirs may be the better answer.**
They put the span on the marker — the list reads 0, 1, 2–3, 4, and a survey reads
4–216 — so there is no hole left to explain rather than a note explaining it. Mine is
a count in the subtitle because this head's seal is a 22 pt circle that cannot hold
`2–143`. Recording the divergence deliberately: **two heads need not render a field
identically, but the reason should be the layout and not an accident.**

**We each asked the other to ask the core for a field that already existed.** The
sweep found it; reading the core's log did not. That is the third time this cycle
that running the instrument beat following the conversation.

**Also measured, and NOT changed.** `PolygonEditModel` reads `minimumVertices` with a
`?? 3` fallback feeding an operator sentence — "A line needs at least 3 corners"
would be wrong for a corridor, which needs 2. The core serves it correctly for both
(ring 3, line 2), so the fallback is unreachable and I am not inventing a fix for a
state I cannot produce. Worth knowing that `EditablePolygon` is decoded inline in a
store, which is exactly the blind spot `null-fallbacks.py` documents.

### A fallback test that never crossed its own boundary, 2026-09-12

Android caught this in their tree and it was true in mine. Every "absent" case in the
three new checks was written as `view["key"] = optional`, and **assigning nil to a
Swift dictionary removes the key.** So all of them tested the KEY-ABSENT branch and
none tested the one that actually travels: `60141bb0c` established that the bridge
carries three states, and what the core really sends for "no value" is a **key held
at null**. The fallback existed for a boundary no assertion crossed.

Both paths do reach nil through `as? NSNumber`, so the code was right — but it was
right unverified, which is the same species as a fixture sharing an author with its
implementation. An explicit `NSNull()` payload is now asserted, and a mutation that
substitutes a value for the null fails it.

**The two sweeps are complements, not rivals — Android's correction to my corpus
point.** Mine reads LIVE payloads: it sees only what the plan I built produced, so a
simple plan hid `exitCoordinate`, `patternDistance` and `foldedCommands`. It
**under-reports**. Theirs reads the core's SOURCE: every key the core can ever emit,
including ones no reachable state produces. It **over-reports** — their 333 is
inflated by exactly what mine filters for free. Source gives the universe, live gives
reachability. Run theirs for candidates and mine to sort them; a key in one and not
the other is either unreachable or wants a richer plan, and which of those it is, is
the question worth asking.

### A pattern says how high above the ground it flies, 2026-09-12

The fifth field the sweep found — and **the sweep did not find it**. `97f05e940`
added `surfaceDistanceText` to `view.surveyStats`, and reading the core's commit
under rule (i) is what surfaced it. **The sweep had been silently skipping
`view.surveyStats` for its entire life**, because that view needs an index argument
and the scratch predicate swallowed the exception and continued. Twenty-two fields
invisible, and a clean-looking total the whole time. Rule 7 in my own tooling: **a
checker that cannot read something must say so.** Fixed; the view now reports its
refusal, and the sweep immediately shows both new keys.

A `TransectStyleComplexItem` has no altitude fact at all, so `altitudeText` is null
for a survey and always was — **this head's altitude column has been drawing an em
dash for patterns, which is correct.** The Android head drew
`cameraCalc.distanceToSurface` there instead and the row read `50.0 m` as though it
were an altitude. **Those are different quantities: one measured from the terrain,
one from the launch point, and for a terrain-following survey they differ BY the
terrain.** So the new figure goes in the survey card as **"Above the ground"**, not
in the altitude column — and an assertion pins that, with a mutation putting it in
the column failing two assertions that already existed.

**Observed and NOT fixed, because it is not mine to fix alone.** The altitude column
already mixes reference frames, and each choice is the core's: the launch row draws
`altitudeText` = **491 m AMSL** (`f8e16903b`, deliberately), a waypoint draws
**75.0 m relative to launch**, a survey draws its band = **541 m AMSL**. Scanning the
column gives 491, 75, 75, 541 with nothing saying which is measured from where. The
head draws the core's text in every case and is not re-deriving anything, so
labelling the frame is a contract question for both heads and the core rather than a
unilateral head change. Raised, not invented.

### A pattern draws the path it actually flies, 2026-09-12

**Found through the blind spot I had just written down.** `ebd931201` added
`view.missionItems(geometry)` to the recorded contract, describing it as "the map
geometry both heads draw". It is an **argument mode of the view this head reads most**
— and nothing checks argument modes. `view-fields.py` reports 0 of 62 views unread and
is right, because `(geometry)` is not a separate view; the sweep asks for bare paths.
Two cycles running, the newest thing has been behind an argument.

Measured on a real plan: a survey carries **112 transect points**, a corridor **8**,
a structure scan **none** — it circles a shape rather than mowing it, the third member
of the class behaving differently again. `grep transect macos/Sources` found two
comments and no code (control: `vertices` matches in three files). **The map drew a
survey's boundary and a straight line from entry to exit, and never the serpentine the
vehicle actually flies.** The terrain profile has been walking those legs all along —
the information was in the app and not on the map.

`PatternGeometry` parses the core's `shape`/`vertices`/`transects`; a geometry with no
shape is refused, and a single point is not a line. Five assertions, two mutations in
place. **`native_screenshot` cannot see MapKit**, so the probe projects the counts and
they were measured in both states in the same binary: plain waypoints give
`transects []`, and survey + corridor + structure gives **`[112, 8]`** — the structure
contributing nothing, exactly as asserted.

**`swift-checks.sh` caught my new model file before the compiler did**, refusing to
stay silent about a `*Model*.swift` it was not compiling. That is rule 7 built into the
tool rather than remembered, and it is the check I would most want to have written.

**The suite is 706/1/89 and the one failure is not mine.** The core added
`altitudeFrame` to `view.missionItems` — **the field I asked for when I reported the
mixed-frame column** — and the recorded contract has not caught up; both
`core-rs/src/missionitems.rs` and `test/Bridge/fixtures/view-shapes.json` are modified
in their working tree right now. My change touches no view shape. Their values are
`"launch"` and `"amsl"`: **a key rather than a phrase**, which is what Android asked
for so neither head matches on a translated string. Reading it is the next cycle's work.
