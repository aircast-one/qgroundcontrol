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
