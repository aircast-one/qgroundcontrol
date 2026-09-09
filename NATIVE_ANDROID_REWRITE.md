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

**Native video decodes on Android** (`4d0c3dd16`, `d6abe4cd9`). Two faults, both
silent. The static build never registered `app` or `videoconvertscale`, so
`createNativeSink`'s `videoconvert ! appsink` could not be constructed — a
dynamic GStreamer finds those on disk, so only the Android build was affected.
And `GstVideoReceiver::startDecoding` refused outright when `_widget` was null,
before looking at the sink it had been handed; only a sink rendering into a
QQuickItem needs a widget.

**Measured on a OnePlus 6 against a 1920x1080 h.264 MPEG-TS stream**, with the
sender verified at 93 fps encoder throughput so it is not the limit:

| | |
|---|---|
| throughput | ~20 fps of a 30 fps source |
| frame copy out of the buffer | 1.3–1.7 ms for 8.29 MB, about 5 GB/s |

So the copy the JNI does is **not** the problem — at 30 fps it would cost around
5% of a core. The loss is upstream of it, and it has now been isolated.

Sampling the frame counter every 100 ms shows the arrivals are **evenly spaced**
— 2 per sample with a 3 every fifth — not the clusters and gaps a sink dropping
on full would produce. So it is a steady throughput ceiling, not backpressure.
And it scales with pixels rather than frames:

| source | delivered |
|---|---|
| 1280x720 30 fps | 30 fps, even |
| 1920x1080 30 fps | ~22 fps, even |

That is about 45 Mpx/s either way — a per-pixel CPU cost upstream of the copy.
I attributed it to the NV12-to-BGRA `videoconvert` because that is the per-pixel
stage I had put there. **That attribution is wrong, and so is the number.**

Turning on GStreamer's own logging (`gstDebugLevel=4` under `[General]`; the
`[LoggingFilters]` group only takes *registered* QGC categories, and `VideoAllLog`
is not one) shows the decoder actually selected was **`avdec_h264` — software**.
The cause is the test stream, not the app: `x264enc` defaults to **High 4:4:4
Predictive**, which no Android hardware decoder accepts, so `decodebin` fell back
to software. Software H.264 decode is itself per-pixel and scales with resolution
exactly like `videoconvert` does, so the measurement cannot separate the two, and
it was taken on a profile no drone sends.

**What this leaves established, and what it does not.** The copy costing ~5% of a
core is still measured and still true. That 720p is smooth and 1080p is not, on
*this* stream, is still true. What is **not** established is the 45 Mpx/s figure
as a property of the app, the `videoconvert` attribution, or the conclusion that
1080p needs a Surface. Re-measure against a **Main-profile 4:2:0** stream first:

    gst-launch-1.0 -q videotestsrc pattern=ball is-live=true \
      ! video/x-raw,width=1920,height=1080,framerate=30/1 \
      ! x264enc tune=zerolatency bitrate=8000 key-int-max=30 speed-preset=veryfast \
      ! video/x-h264,profile=main ! mpegtsmux ! udpsink host=<phone> port=5600

### The re-measurement produced a better answer than a frame rate

On a Main-profile stream the decoder selected is
`amcviddec-c2androidavcdecoder` — hardware, as expected. But **no frames arrive at
all**: 0 frames, 0x0, and the pipeline errors out with
`streaming stopped, reason not-negotiated (-4)`, preceded by
`<amcvideodec-c2androidavcdecoder21> Subclass refused caps` and
`invalid matrix 3 for RGB format, using RGB`.

The caps the decoder offers say why:

    video/x-raw(memory:GLMemory), format=(string)RGBA, width=1920, height=1080, ...

**The hardware decoder delivers a GL texture, not system memory.** `createNativeSink`
is `videoconvert ! appsink`, and `videoconvert` cannot consume `memory:GLMemory`, so
negotiation fails before a single frame is produced.

So the appsink path has **never worked with hardware decode**. It appeared to work
only because the 4:4:4 test stream forced the software decoder, which does output
system memory. There is no throughput ceiling to fix: there is a path that does not
connect.

**This is the gating problem for Phase 5 video, and it does point at a Surface** —
but for a sounder reason than the retracted measurement gave. Reading decoded frames
on the CPU means pulling a GL texture back across the bus every frame (`gldownload`,
with the GStreamer GL elements added to the static build), which is precisely the
work the hardware decoder exists to avoid. Rendering the texture where it already
lives is the shape that fits.

**That question is now settled, and the answer is no.** At `gstDebugLevel=5` the
decoder states it directly:

    info:     <amcvideodec-c2androidavcdecoder0> GL output: disabled
    critical: <amcvideodec-c2androidavcdecoder0> Codec only supports GL output but
              downstream does not
    warning:  <amcvideodec-c2androidavcdecoder0> Subclass refused caps

**This is now solved — Android video renders.** `createNativeSink` builds
`glupload ! glcolorconvert ! glimagesink` on Android and a new `qgc_video_set_window`
hands the sink an `ANativeWindow` through `GstVideoOverlay`; a Compose `SurfaceView`
supplies the Surface. Verified on the OnePlus 6 against 1080p30 Main profile: the
decoder selected is `amcviddec-c2androidavcdecoder`, **zero** not-negotiated and
**zero** refused caps in a log with zero dropped chunks, and the ball moves between
screenshots — (19,131), (280,98), (372,171) — which is what separates a live pipeline
from a stuck frame. `glupload` passes GLMemory through untouched and uploads system
memory when software decode is in play, so one bin serves both decoders.

**macOS keeps the appsink, and that fork is correct rather than cautious.**
`VideoSurface.swift` reads frames through `qgc_video_copy_frame`, and `vtdec`'s src pad
template offers a system-memory output where the Android codec offers none. I nearly
shipped the GL bin to both and broke macOS silently — it compiles either way, and their
video is parked waiting for a stream, so nothing would have complained for a long time.
Checking who called `qgc_video_copy_frame` before deleting it is what caught it.

**Video and telemetry together: no interference found, and the instrument cannot see one.**
Everything until now had been tested one at a time — video with no vehicle, the vehicle with
no video — so the Fly tab's actual condition was untested. Running both: the vehicle connects
and reports, the ball renders, and **zero** bridge calls exceeded the 250 ms warning threshold
across the run.

Sampling screenshots suggested the ball froze for several seconds at a time, which looked
like telemetry starving video. **The control run says otherwise** — with the vehicle stopped
and only video running, the ball sits at the same centroid for four consecutive samples too.
So `adb exec-out screencap` is not a frame-rate instrument for a `SurfaceView`; it shows the
video is live and says nothing about its rate. Measuring that needs a counter in the pipeline,
not pixels. Recorded because the wrong conclusion was one unrun control away.

**The inset expands.** A 200x112 corner box is not something to fly from, so tapping it
fills the view and a close button puts it back. The two directions are deliberately not
symmetric: enlarging a small picture is a guessable convention, while an operator flying by
map who taps the inset by accident loses the map, and that needs a button rather than a
convention to undo.

It resizes the same `SurfaceView` rather than creating one, which is forced by the
`glimagesink` one-window limit above — a design that recreated the surface on expand would
go black exactly as the tab switch did. Verified through the full cycle on the handset with
the ball moving at every step.

**The inset is not the product.** It sits in the Fly tab corner and says "No video"
until a stream starts. While the QML view still owns that tab, the operator can see
video twice — QML's own inset and this one — which is the price of proving the pipeline
before the view moves. The duplication ends when the Fly view migrates, and that is the
same moment the limitation below has to be faced, so they should be done together.

**A limitation the Fly view migration inherits.** `glimagesink` creates its GL context
against the first window it is given and does not take a second one. While the surface
lived inside the tab switch, leaving Fly destroyed it and coming back gave the sink a
new window it ignored — the view was permanently black afterwards. The surface now sits
outside the switch and other tabs draw over it, so it is never destroyed. That is
sufficient today and will not be when video becomes the Fly background rather than an
inset: something will have to restart the receiver on a genuinely new surface.

**Four defects in the first version of this, all found by reading rather than running.**
The stored sink pointer was borrowed, not referenced, so tearing the bin down left a
handle into freed memory. `ANativeWindow_fromSurface` acquires a reference that was only
released on failure, leaking one per surface cycle. `qgc_video_detach_overlay` was dead
code nothing called. And the fix for the leak had its own: `qgc_video_set_window(nullptr)`
returned false when no sink was attached, so the JNI read a legitimate *clear* as a
failure and skipped the release — the leak survived on precisely the path the fix was
written for. One bool was carrying "nothing to do" and "could not do it".

That last one is worth generalising with the notice-slot bug: a return value that
conflates two outcomes hides the failure on the path that needs it most, and the happy
path passes either way. Lifetime and ownership live in the type and the call, so reading
finds them; no amount of device time would have.

The finding that forced all this:
`c2androidavcdecoder` has **no system-memory output mode**. There is no arrangement of
caps that gets decoded frames into system memory on this device, so `videoconvert !
appsink` was never going to work and no amount of tuning it will. That leaves exactly
two options, and they are a real choice rather than one being obviously right:

- **Consume the texture where it lives** — a `Surface`/`SurfaceTexture` the decoder
  renders into. Keeps hardware decode, no per-frame copy across the bus, and is the
  reason the Phase 5 bullet now points here.
- **Software decode** (`avdec_h264`). Works today and is what every measurement in this
  section was actually taken through. Costs per-pixel CPU, which is what the retracted
  45 Mpx/s figure was really measuring.

**How this was read matters, because the first attempt at it lied.** At level 5 the log
overruns logcat's default buffer: that run reported *zero* decoders created, zero
`Streaming started`, zero refusals — all artifacts of 230 dropped chunks. Re-running with
`adb logcat -G 64M` gave 85,521 lines, **zero drops**, and the opposite picture: two
decoders, two `Decoding started`, ten refusals. Absence in a lossy log is not absence.
Check the drop count before believing a zero.

The general lesson, which was already in this plan when I broke it: a measurement
whose input you have not characterised measures the input.

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

### The bridge read got a projection, and the measurement redirected it

`plan.missionController.visualItems` with 199 items costs **247–294 ms** on the handset,
which is 95% of what the native map does per 700 ms poll. The `map-spike` session measured
the curve at 50, 100 and 199 items — about **1.3 ms an item, no knee** — which is the fact
the fix depended on: with a straight line, halving what gets serialised halves the time,
where a cliff would have meant something else dominated and a narrower read bought nothing.

Parsing the whole thing in Kotlin is 3.2 ms, so it was never the traversal. `objectJson`
walked every property *and every fact* of every item into one string, and a Fact carries
`min`, `max`, `enumStrings`, `enumValues` and both bound strings.

`getFields(path, csv)` now serialises only the named properties and emits facts as
`name`/`value`/`valueString`. Measured over 21 items:

| read | bytes | time |
|---|---|---|
| whole | 80,873 | 4,322 µs |
| `"*"` — every property, compact facts | 37,026 | 1,647 µs |
| explicit field list | 13,088 | 1,307 µs |

**The field list is the smaller half.** Compacting facts carries ~88% of the saving; naming
sixteen fields buys the last 340 µs and costs a list that has to be kept in step with the
model. The first version only compacted facts *when* a caller named every field, which
locked the cheap win behind the expensive one — `"*"` exists because measuring the halves
apart showed that was backwards.

**Bytes and time move independently, and not even in a consistent direction.** Three shapes,
all measured:

| shape | bytes | time |
|---|---|---|
| 21 simple waypoints (macOS Debug) | 16% | 30% |
| 200 simple waypoints (handset) | 46% | 56% |
| 12 surveys (handset) | 55% | **34%** |

On waypoints the byte figure *overstates* the win; on surveys it *understates* it, because a
survey drags `cameraCalc` behind it and that is facts all the way down — proportionally more
metadata to drop. I wrote "the byte count overstates the win roughly twofold" as if it were a
rule after seeing one shape; it was one shape. What survives is weaker and true: **the two
quantities are not proxies for each other, and neither predicts the other without measuring
the plan in front of you.**

**At 199 items the ratio holds**: 40.2 ms whole, 16.2 ms with `"*"`, 12.4 ms with the field
list (macOS Debug — the absolute figures are not the handset's, but it is the same code
doing the same work, so the proportions transfer). Applied to the handset's measured
250 ms that is roughly **100 ms**, which takes the map's poll from 36% of its 700 ms
interval to about 14%, and pushes the saturation point from ~500 items to well past a
thousand.

**Notify connections are *not* the answer, and the note above `Watcher` said they were.**
That note told the next person to move to generic `QMetaMethod` notify connections when
the poll cost bit. The poll cost bit, someone read the note, and reached exactly that
conclusion. It is wrong:

- `Vehicle` declares `NOTIFY` on **96 of 164** properties.
- `TransectStyleComplexItem` on **5 of 15**. `SurveyComplexItem` on **none of its 4**.
- `QmlObjectListModel` emits only `countChanged` and `dirtyChanged`, so **an element's own
  property changing is invisible to the model that holds it** — which is precisely the read
  that raised the question.

A notify-only `Watcher` would go silently stale on the majority of the properties the map
reads. On a flight display that is the worst failure available: no error, a number that
stops moving. The real upgrade is a hybrid — connect where a `NOTIFY` exists, keep polling
the rest — and the code comment now carries these counts so it is not rediscovered.

Polling was the right choice and remains it. It re-resolves each path every tick, which is
why it survives the active vehicle changing and a list being rebuilt underneath it.

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
- ~~`Viewer3D` (1.7k) → decide keep-or-drop; it is the cheapest moment to drop it.~~
  **Decided: drop for Android — but not here, and not now.** Two corrections to that line.

  *It is not an Analyze feature.* `Viewer3D` is reached from the Fly view tool strip
  (`FlyViewToolStripActionList.qml`), and `FlyView.qml` imports and instantiates it
  directly. It sits in Phase 2 only because someone filed it with the other read-only
  visualisations.

  *The case for dropping it on Android is strong.* It costs **4.0 MB** of QtQuick3D in
  every AAR (`libQt6Quick3DRuntimeRender` 2.1 MB, `libQt6Quick3D` 1.6 MB, plus plugins),
  its `enabled` fact defaults to **false**, and `osmFilePath` defaults to the placeholder
  *"Please select an OSM file"* — so on a phone it ships four megabytes to render nothing
  unless the operator has put an OSM extract on the device and pointed at it.

  *But this is not the cheapest moment, because it is not a flag flip.* `QGC_VIEWER3D=OFF`
  compiles out the C++ and skips `Viewer3DManager::registerQmlTypes()`, which is behind
  `#ifdef QGC_VIEWER3D` — while `Viewer3D.qml` and its `qmldir` stay in
  `qgroundcontrol.qrc` **unconditionally** and `FlyView.qml` still does `import Viewer3D`
  and `Viewer3D { }`. Turning the flag off therefore leaves QML instantiating C++ types
  that no longer exist, and takes the whole Fly view down with it. A clean drop has to
  remove the `.qrc` entries and the `FlyView.qml` usage in the same change.

  *So it costs nothing at the Phase 6 teardown and something now,* since Phase 6 deletes
  `FlyView.qml` anyway. Dropped there, not here.

~~Needs from the bridge: a table/list model reader.~~ **That requirement does not exist.**
Nothing under `src/AnalyzeView/` uses `QAbstractTableModel` or `QAbstractListModel`: Log
Download, the inspector and the console all expose `QmlObjectListModel`, which the bridge
already traverses, plus `QStringList`. The only `QAbstractListModel` subclasses in the tree
are `FactValueSliderListModel`, `ParameterEditorController` and `QmlObjectListModel` itself,
and none of them is on an Analyze path. Left as a struck-through line rather than deleted
because a reader who remembers the original would otherwise assume it was overlooked.

**The native head shows three pages QGC hides on Android, on purpose.**
`QGCCorePlugin::analyzePages()` wraps GeoTag, MAVLink Console, MAVLink Inspector and
Vibration in `#ifndef Q_OS_ANDROID`, so the QML build offers **only Log Download** on a
phone. The native picker offers four.

That is not an oversight to correct back. The controllers —
`MAVLinkConsoleController`, `MAVLinkInspectorController`, and the vibration facts — carry
no platform guard and are compiled on Android already; what was excluded was the
desktop-shaped QML page, not the capability. Building phone-shaped versions of exactly
those pages is the migration doing its job. GeoTag stays out: it is file-system work on
images, and its exclusion is about the platform rather than the layout.

**That makes this phase's gate unsatisfiable as written.** "Chart values match the Qt
build" cannot be checked on Android, because the Android Qt build does not draw those
charts at all. The comparison has to be against the **desktop** Qt build, on the same
vehicle, or the gate can never be met.

**The Inspector is verified on hardware**, which matters because QGC has never run it on
Android, so nothing had ever exercised it there. Against the ArduPilot sim on the
OnePlus 6: seven message types with live rates (`COMMAND_ACK` correctly at 0.0 Hz, being
event-driven), and ATTITUDE's fields updating live.

Reading it first found a defect. The open message was a snapshot carrying its index into
the live model, and the field list re-read `messages.<index>.fields` on every update —
correct while the model only grows, wrong the moment it is rebuilt, since a vehicle reboot
replaces `activeSystem` and the old index then points at a different message. The header
would have kept the old name above the new message's fields. It now holds the name and
resolves the index from the current list.

**The Console had invented a restriction QGC does not have.** It returned early for any
vehicle not reporting PX4 firmware, stating the console "is a PX4 feature" and the vehicle
"has no shell to connect to". Neither `MAVLinkConsoleController` nor `MAVLinkConsolePage`
checks firmware anywhere — QGC offers it to every vehicle. The belief behind it is roughly
right, since the shell answers on PX4 and most ArduPilot builds ignore `SERIAL_CONTROL`,
but roughly right is not a reason to refuse, and the wording claimed a certainty the code
did not have. It is now a note above a usable console.

That is the same defect as the withdrawn plan warnings and the flight-control predicate,
in its third costume: **a message asserting something the program does not know.** Here it
also blocked the feature, which is the most expensive form of it.

### Status — all four pages are built

Log Download, Vibration, MAVLink Console and MAVLink Inspector all exist natively
(`LogDownloadScreen`, `VibrationScreen`, `ConsoleScreen`, `InspectorScreen`), reached from
a picker. `Viewer3D` is still an open keep-or-drop decision, and the QML deletion in the
gate below has not happened.

**Gate (HW):** log download from real hardware; chart values match the Qt build. `src/AnalyzeView/`
QML deleted.

**Half that gate is already discharged, by construction rather than comparison.**
`VibrationPage.qml` declares `_barMinimum 0`, `_barMaximum 90`, `_barBadValue 60`,
`_barMidValue 30` and draws each bar at `min(max, value) / (max - min)`. The native screen
uses the same four numbers and an equivalent formula, and both read the same facts —
`vehicle.vibration.xAxis/yAxis/zAxis` and `clipCount1..3`. Same inputs, same scale, same
transform, so the charts cannot disagree. Unit tests pin the constants against QGC's, and
setting the maximum to 100 fails two of them.

The remaining half — log download from real hardware — still needs the vehicle. **The
software half of it is now proven, which it was not before.** The screen had only ever been
seen against a vehicle with no logs, so nothing distinguished "works and the list is empty"
from "never worked". Teaching the sim the log-download protocol showed the whole path: the
list arrives with sizes and timestamps, selecting one and downloading shows a progress bar
and a transfer rate, the row turns from Available to Downloaded, and a file lands in the log
directory. The bytes were checked rather than the file's existence - 4096 of them, matching
the pattern the sim generated exactly.

That leaves a genuinely smaller gap: whether a real autopilot's log sizes, timing and
failure modes behave the same. It does not close the gate, because a sim that answers every
request promptly proves nothing about a vehicle that stalls mid-transfer.

One protocol detail the sim got wrong first and is worth keeping: ArduPilot log ids are
1-based. Sending ids 0 and 1 with `last_log_num` 1 made QGC re-request `2..2` forever,
because it counts the last id rather than the number of entries.

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

The criterion is not "is the call `void`" — most of them are. It is **whether the
operator can tell a refusal from a call still in flight.**

Reporting was added where they cannot: the settings controls showed the new value
optimistically and then reverted, which reads as the app glitching; a link
operation is slow enough that an unchanged row is ambiguous; a calibration closes
its dialog and takes seconds to show anything.

It was deliberately **not** added to `UnitsPage`, `VehicleMessages` and
`InspectorScreen`. All three render entirely from watched paths rather than from
local state, and their operations are immediate, so a failure shows at once as
nothing changing. A notice there would be noise on a surface that is already
honest. Arming and the guided actions are left for a different reason: the prearm
banner and the vehicle's own state already report them, and a second voice would
compete with the more authoritative one.

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
than the gap it names, since it trains the pilot to ignore the next one.

`objectJson` now carries the runtime class name, so a client can name the item
exactly — `CorridorScanComplexItem`, `StructureScanComplexItem`,
`FixedWingLandingComplexItem`, `VTOLLandingComplexItem`.

**Correcting the reason I first gave for this.** I claimed no predicate could
separate a corridor scan from Mission Start. That was wrong, and reading a real
corridor scan is what showed it — the same mistake the withdrawn attempts made,
made once more while adding the fix for them. Against a live plan:

| item | `class` | `patternName` | `isSimpleItem` | `isSurveyItem` |
|---|---|---|---|---|
| Mission Start | `MissionSettingsItem` | `""` | false | false |
| waypoint | `SimpleMissionItem` | `""` | true | false |
| corridor scan | `CorridorScanComplexItem` | `"Corridor Scan"` | false | false |

`patternName` already discriminated. The three attempts did not fail for want of a
predicate; they failed because each picked one that Mission Start also satisfies,
and never checked against a plan containing the item they were warning about.

The class name is still the better primitive, for a reason that is *not* the one I
gave: **`patternName` is a translated display string** (`tr("Corridor Scan")`), so
matching on it silently stops working in any other locale. `class` does not move.

Also checked, since the survey layer keys off it: a corridor scan does **not** carry
`surveyAreaPolygon` at the top level, so it is not being mis-drawn as a survey. It is
absent from the map, as this note said — not wrong on it.

**The warning now exists** (`aircast-android`, `undrawnItemNames`/`undrawnItemsWarning`)
as a **persistent banner between the toolbar and the map**, for as long as the plan
holding those items is open.

It shipped first through the transient notice slot, which was wrong twice over: it
cleared after four seconds while the hazard stayed true, and while showing it replaced
the plan filename. A plan is missing from the map for as long as it is open, not for
four seconds. Deriving the banner from the plan rather than from the open event also
covers a plan downloaded from the vehicle, and clears it on New plan for free.

**The rule contains no class names.** An item is drawable if it says `isSimpleItem` or
`isSurveyItem` about itself; element 0 is structurally the settings item, so the scan
starts at 1. Two earlier drafts each hard-coded names, and both were the failure that
sank the previous three attempts wearing a new coat — a takeoff reports
**`TakeoffMissionItem`**, not `SimpleMissionItem`, so a class-name allow-list warns on
almost every plan. `SimpleMissionItem::isSimpleItem` and
`SurveyComplexItem::isSurveyItem` are both `final { return true; }` with the base
returning false, so neither can be wrong and no subclass can change them. The class
name survives only as the label when `patternName` is blank: a good name and a bad
predicate.

That distinction is worth keeping generally. *What is it* can be settled by reading a
signature — type identity has no state to be wrong about. *What does it do when* cannot,
and that is where the device runs earn their keep.

Verified on the OnePlus 6 against a `.plan` **QGC itself wrote** containing a real
corridor scan: opening it says *"The map cannot draw Corridor Scan. Those items are
still in the plan and will still be flown."* An out-of-date AAR under-warns rather
than warning wrongly, since an item with no `class` field yields no name.

The first attempt at this proved less than it looked. That file also held a survey
which the warning correctly did not name — but the survey was **empty** (`path: []`,
`count: 0`, `isValid: false`), because `insertComplexMissionItem` given only a map
centre leaves the polygon for the user to draw. A warning staying silent about an
item with no shape is not evidence it stays silent about one with a shape.

So the check was redone with a survey given four real vertices
(`surveyAreaPolygon.appendVertex`, confirmed by read-back at `count: 4`,
`area: 503300 m²`, `isValid: true`, not by the `ok` of a void call). Opening that
plan on the phone draws the survey polygon with its vertex handles **and still names
only Corridor Scan**. That is the over-firing mode that sank the previous three
attempts, now disproved on hardware rather than argued.

Still to do: actually draw corridor scans, structure scans and landing patterns.
**Android needs an AAR rebuild before it sees the `class` field.** The gate above is
unaffected for surveys but cannot be claimed for a mixed plan.

**Unexplained, recorded so it is not rediscovered from scratch:** a temporary probe
that called `plan.loadFromFile` inside the bridge test, read the items back, then
`plan.removeAll`, made the suite exit 132 (SIGILL) *after* printing ALL TESTS PASSED.
Every test passed; the crash is in teardown. Removing the probe restores exit 0. Not
chased, and not claimed to be a real defect — but a plan load followed by teardown is
where to look if it resurfaces.

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

**Reading back can settle on a value that does not last.**
`MissionSettingsItem` overwrites `plannedHomePositionAltitude` with the terrain
elevation about two seconds after a write, unless the home came from the vehicle:
write 10, read back 10, read back 596. A check that reads immediately after
writing confirms a value that is about to be replaced. The read-back pattern
above verifies *operations* — connected, removed, started — which do not have
this shape, but a read-back that verifies a *written number* needs to know
whether anything else owns that field.

**A `Q_ENUM` crosses as its number, and that was a breaking change.** An
enum-returning invokable used to fail outright: the metaobject records a return
type as written, so an enum declared inside a class is `State` there and
`Class::State` in the metatype, and `invoke` refuses the mismatch. Enum
properties came across as their *name*. Both now give the number (`2b0dce01f`),
which is what a caller can compare — a printed name changes with translations and
refactors.

The macOS head had four controls keyed on those names and all four went silent
rather than erroring: the pickers rendered empty and writes of an enum name were
accepted and did nothing. **Anything reading an enum property by name is broken
by this and will not say so.** The Android head had no exposure — its only enum
read is `readyForSaveState`, already taken as a number.

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
another GCS does not.

Demonstrated rather than reasoned: with a vehicle pushing an unsolicited
`PARAM_VALUE` every few seconds, a `WPNAV_SPEED` row read 220.000 when opened and
still read 220.000 twenty-five seconds and eight changes later.

**`Qgc.unwatch` now exists** (reference-counted, since several screens read the
same path and only the last one out may drop it; `qgcPath` holds through a
`DisposableEffect`, so every reader pairs automatically). That closed a real leak:
the poll set used to grow monotonically for the life of the process, because every
path any screen had ever read stayed in it.

It does **not** make per-row parameter watching the right answer, which is why the
rows are still on demand. The watcher *polls*; watching the eight visible rows would
turn a rare re-read on write into a continuous one on every tick, for values that
essentially only change when someone writes them. The cheap part of the fix was
worth taking, though: the revision counter that invalidated a row lived on the
screen, so one edit re-read every visible row. Each row now owns its own.

The stale-row case above therefore stands, and the honest framing is that it is a
**deliberate trade** — a parameter changed by another GCS is not reflected until the
row is reopened — not a missing primitive.

**The flight controls were the biggest hole in this and I missed them first time.**
Arm, disarm and mode changes discarded the result of a checkable `Qgc.set`. They looked
acceptable because the armed state and mode chip update live — but a MAVLink round trip
makes an unchanged chip ambiguous between *refused* and *still in flight*, which is
exactly the test. They now wait four seconds for the vehicle to report the state that
was asked for and name the command when it does not.

Takeoff, Land and RTL are deliberately still silent: their firmware mode names differ
between PX4 and ArduPilot, so there is no reliable predicate, and the vehicle's own
STATUSTEXT already reaches the message banner directly above those buttons. A guess
there would be worse than silence.

**A predicate that can only fail silently.** The first version confirmed on *any* change
to `(mode, armed)` rather than on reaching the requested state. Freezing the vehicle
mid-command exposed it: the aircraft went to RTL on its own comms-loss failsafe, so the
state changed, the command had not been obeyed, and the check reported success. Its only
failure mode was staying quiet when a warning was owed. The exact predicate — did the
vehicle reach the state asked for — catches it, and the frozen-vehicle run now produces
*"Acro was not confirmed by the aircraft."*

**A fourth instance, in the hardest place to spot one.** Two Setup sections carried the
read-only note as a *hard-coded string* in a static list — the ArduPilot one telling every
operator "this firmware reports every slot after the first as read-only" without asking
their firmware. The facts already carry `readOnly`, so it is now derived: silent when
nothing is locked, "all of these" when everything is, and the specific names otherwise.

Verified on the handset: it renders *"This firmware reports FLTMODE2, FLTMODE3, FLTMODE4,
FLTMODE5, FLTMODE6 as read-only…"* and the rows agree — slot 1 has an editable dropdown,
2 to 6 are plain text. The original claim was true for this firmware; it is now earned
rather than assumed, and it will disappear on firmware that does not lock those slots.

Worth noting where it hid: a sentence in a `listOf(...)` of section definitions reads as
configuration, not as an assertion about a vehicle. The audit that found the others was
looking at UI code.

And the right pattern was already in the tree, one file over. `desktopOnlyNote` in
`SettingsSections.kt` derives its sentence from the facts actually present and returns
nothing when none are — exactly what `readOnlyNote` now does. Setup had the hard-coded
version while Settings had the derived one, so this was not a new idea, it was a rung of
the ladder nobody climbed. A sweep of the remaining prose in both static lists found the
rest sound: they are either honest about what this port has not built yet ("the calibration
wizard is not here yet") or general guidance about a parameter rather than a claim about
the connected vehicle.

### The rule these keep breaking

Three times tonight, in three different costumes, the defect was **a message asserting
something the program does not know**: the withdrawn plan warnings, the flight-control
predicate that confirmed on any state change, and the console refusing ArduPilot on a
firmware rule QGC does not have. The last one blocked a feature, which is the expensive
form; the others merely misinformed.

So, for any user-facing sentence about the vehicle or the system, ask which of these it is:

- **An observation** — "the autopilot has not sent a VIBRATION message". Always safe: it
  says what was seen, not what is true.
- **An attributed report** — "slots the firmware reports as read-only". Safe: the claim is
  the firmware's, and the sentence says so.
- **A guarded claim** — "the vehicle is not ready to fly yet", shown only when
  `readyToFlyAvailable` is true. Safe *because* of the guard; without it the same sentence
  is a fabrication about vehicles that never signal readiness.
- **An assertion** — "this vehicle has no shell to connect to". Needs a verified basis or
  it must be softened to an observation. If it also disables something, the bar is higher
  still, because a wrong assertion then costs the operator a working feature.

An audit of every such string in `app/src/main/java/one/aircast/android/ui/` against this
found the rest sound, so this section is a rule for new work rather than a list of pending
fixes. The one that is *verified* rather than merely plausible is worth copying: "that is
not a plan file, the current plan is unchanged" is backed by a bridge test
(`_aRefusedLoadLeavesTheExistingPlanAlone`) that proves the refusal leaves the plan intact.

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
- Video: GStreamer is not Qt and survives, and the Android build carries `libgstapp` and
  `libgstandroidmedia`. But **the `appsink` → `ImageReader` → Compose path written here does not
  work**: `amcviddec` negotiates `video/x-raw(memory:GLMemory), format=RGBA`, and `videoconvert`
  cannot take it, so the pipeline fails `not-negotiated (-4)` with zero frames. See the Phase 5
  section above. The frames never reach system memory, so the sink has to meet the texture where it
  is — a `Surface`/`SurfaceTexture` the decoder renders into — rather than being fed by a CPU copy.

### What switching the Fly tab actually costs

The bullets above say what to build; they do not say what `FlyView.qml` currently provides,
so anyone switching the tab would find the gaps by removing them. Its layers against what
exists natively today:

| QML layer | Native | Note |
|---|---|---|
| vehicle marker, trail, follow | **`VehicleMap`**, on the Fly tab | `FlyMap` draws it with the plan read-only; the QML view is covered on Fly |
| telemetry readouts | **`TelemetryRow`** | |
| flight mode | **`FlightModePicker`** | reports refusals |
| arm / takeoff / land / RTL | **`FlightActions`** | slide-to-confirm; arm and mode report refusals |
| vehicle messages / warnings | **`VehicleMessageBanner`** | prearm text, unhealthy sensor, not-ready-to-fly, and no-GPS-lock |
| status (sats, HDOP) | **`StatusStrip`** | |
| `PipView` map/video swap | **built** | either view can be the small one; the inset is the control and the main view is not |
| `CameraControlLayer` | **built** | selection, mode and shutter, all verified on the wire |
| `VideoTilesLayer` | **built, as a source picker** | the streams the core lists, and switching between them; the tile grid, dock and tuck belong to the overlay rig |
| `RcControlsLayer` | **built** | renders the configured controls and sends `setRcChannelOverride`; editing the list is still desktop-only |
| `ObstacleDistanceOverlay` (map and video) | **built, in another form** | a sentence — "3.2 m right" — rather than a proximity ring |
| `FlyViewToolStrip` + action list | **built, as an Actions sheet** | pause, gripper, emergency stop, mission start/continue, land abort; Viewer3D dropped, preflight sits under Analyze |
| `GuidedValueSlider` | **built** | altitude, speed, takeoff height, and pause all read their range from the core |
| `DetectionOverlayVideo` | **missing** | |
| `FlyViewCustomLayer` | **dropped** | a placeholder for downstream forks to override; nothing to port |
| `OverlayGlass` / `OverlayRig` / `FlyViewInsetViewer` | **partly replaced** | Compose does the layout; what remains of the rig's job is the camera inset, and the bottom one is fed from the measured controls panel |
| `Viewer3D` | **dropped** | decided in Phase 2; goes with `FlyView.qml` |

So the tab cannot be switched yet, and not because video was missing — video works. Eight
surfaces have no native equivalent, and two of them (camera controls, obstacle distance)
are things an operator uses in flight.

**Camera control is built and verified on the wire.** The Fly view shows the current mode
and a shutter that takes a photo or toggles recording, through the same `takePhoto` and
`toggleVideoRecording` that `CameraControlLayer.qml` calls. Against a sim camera
("SimCam"), each action was confirmed by the command the vehicle received rather than by
the button changing:

| action | command | evidence |
|---|---|---|
| shutter in photo mode | `MAV_CMD_IMAGE_START_CAPTURE` (2000) | sim logged the photo; QGC went `PHOTO_CAPTURE_IN_PROGRESS` → `IDLE` |
| mode chip | `MAV_CMD_SET_CAMERA_MODE` (530) | sim switched to video; the chip followed to "Video" |
| shutter in video mode | `MAV_CMD_VIDEO_START_CAPTURE` (2500) | button turned red and read "Stop" |

A camera in a mode it cannot do offers no shutter rather than a button that fails, an
undefined mode offers nothing rather than guessing photo, and a photo already in progress
disables the button rather than queueing another.

Camera *selection* is built. The chip appears only when `cameraManager.cameraLabels` holds
more than one entry, names the current camera, and cycles `currentCamera`; a vehicle with a
single camera sees no control rather than one with nothing to choose. Verified against a sim
advertising two camera components: QGC queried both compid 100 and 101 for
`CAMERA_INFORMATION`, and the chip moved from SimCam to SimCam Thermal.

The UX pass caught a collision worth recording, because it is the kind a screenshot shows and
a test does not. The mode chip and the shutter sat adjacent and both read "Photo" - the first
toggled the mode, the second took the picture. Users satisfice, so the one they reach for
first was the one that did not take a photo. Video mode had always labelled its shutter with a
verb; photo mode now does too ("Take Photo"), and a test asserts no mode's shutter label
equals that mode's chip label, so a mode added later cannot reintroduce it.

**Changing altitude in flight is built.** The slider carries an absolute target seeded from
the current altitude and bounded by `guidedMinimumAltitude`/`guidedMaximumAltitude`, and sends
the *delta*, which is what `guidedModeChangeAltitude` takes. Verified on the wire: dragging to
68.5 m from 25.0 m put one `SET_POSITION_TARGET_LOCAL_NED` out, frame 7 (`LOCAL_OFFSET_NED`),
`z=-43.53`, `type_mask 0xFFF8`.

Two things came out of building it. The row of flight actions had to wrap - a fifth button
pushed `Alt` off the right edge of a 1080 px screen, where it rendered and could not be
reached, which no test would have caught. And confirming is gated on the change being one the
firmware will act on: `APMFirmwarePlugin` drops anything under 0.01 m, so an enabled button
below that is a control that reports success and does nothing. One constant decides both the
button and the sentence, so they cannot disagree.

**The same blind spot, a third time: the Vibration screen had never drawn a coloured bar.**
Every value it had ever been shown was zero, because the sim sent no `VIBRATION` message. The
thresholds, the verdicts and the three colours existed and had never rendered. Sending 15, 45
and 75 across the bands showed them working - and showed the colours in the wrong order.
Caution was a hard amber; High was `colorScheme.error`, which in this dark theme is the pale
pink meant for text on a surface rather than for filling a shape. The most serious band was
the least alarming thing on the screen. High is a saturated red now.

**A core-owned link opens on Android, and it starves the Qt one.** One JNI entry point
(`coreLinkOpen`) reaches `qgc_core_link_open`. A core UDP link on port 14550 opened beside the
Qt link and took **984 frames in twenty seconds with none dropped** - the Rust transport works
on this platform. But the Qt link took **zero** in the same window and the vehicle went to
"Communication lost".

**The bind is not shared on Android.** The core socket gets the datagrams and the Qt one
starves, so the parallel counter comparison the transport plan describes cannot be run on one
port here. It needs two ports with the simulator sending to both, or a sequential comparison.
Worth knowing before that gate is designed around a measurement this platform will not give.

**The preflight checklist, which Android never had.** `view.preflight` serves the airframe's
checks in groups with a verdict each, so the screen can distinguish what the vehicle can answer
from what only a person can. A passing check shows ticked and cannot be unticked; a failing one
carries the vehicle's own reason; a manual one is the operator's to confirm and is the only kind
that counts toward the tally. The summary leads with what would **stop** the flight rather than
with progress. Ticked state stays in the head, because it records what a person looked at, not
vehicle state.

Verified on the handset: eleven checks for this airframe, three passing from the vehicle, the
manual ones tickable, tally moving 0 to 1 of 11.

It sits under Analyze, which is the wrong home for something done before flying and the right
one until the Fly tab is native. Recorded as provisional rather than left to look deliberate.

**The simulator was hiding every command it received.** Its generic `COMMAND_LONG` branch
acknowledged and did not print; only the camera branch printed `CMD n`. So "no command on the
wire" meant "no *camera* command", and several checks tonight rested on that. It logs every
command now, with its parameters, plus arm, disarm and takeoff. The first run with it showed
`SET_MESSAGE_INTERVAL` traffic that had been invisible all night.

With the gate fixed the speed dialog opens and reads correctly - title "Ground speed" chosen by
the core for a multirotor, its sentence, the slider seeded at 2.5 in a 0.1 to 5 range, and the
sentence following the slider to 3.0 m/s. **Its confirm is still unobserved**: repeated taps at
the measured centre of the Set button leave the dialog open, so the handler is not being reached
by `input tap` at all. That is a harness problem rather than a code one - the same confirm path
is verified sending on takeoff and on altitude - and it is recorded as unverified rather than
assumed.

**The same double gate was on all three guided buttons.** Auditing for the pattern found
takeoff and altitude gated exactly as speed had been - the core's offer *and* a locally watched
range. All three follow the offer alone now and read the range when the dialog opens. Altitude
had worked by luck: its bounds come from settings present from the start, so its watch was never
wrong. Takeoff's minimum comes from a parameter, which is the identical exposure. **A bug found
once is worth grepping for; two of the three sites had never failed and would have failed the
first time a vehicle was slow with its parameters.**

**A watched view can be stale, and a head that re-derives an answer will believe the stale one.**
The speed button was gated on the core's `changeSpeed` offer *and* on a watched
`view.guidedSpeed` being usable. Measured with the vehicle armed and flying: the offer read
`ready`, `haveMRSpeedLimits` read true, `parameterExists` read true - and the button was still
disabled, because the watch had been established before the parameters loaded, when the view
correctly reported `available: false`, and nothing made it read again. The offer already encodes
availability; the button follows it alone now and reads the range fresh when the dialog opens.

**Three explanations were wrong before that one, and each cost a build.** Parameter ordering
(`WPNAV_SPEED` is last of 216, but the fact is stored before the ready check); the advertised
`param_count` (216 names, 216 advertised); the component id (`parameterExists` and
`getParameter` both call `_actualComponentId`). Two of the three were mine and one was the core
session's, and all three were reasoning from source about a runtime disagreement. **Measuring
both sides in the same instant** - the offer and the flag, logged from one call site - ended it
immediately and should have been the first move, not the fourth.

**Takeoff asks how high.** `readTakeoffAltitudeMeters`, `verticalOf`, `verticalUnits` and the
3 m fallback are deleted; `view.guidedTakeoff` serves the range, the firmware minimum, the
initial value and the sentence, and its argument form serves the metric target. Takeoff used to
take whatever the settings said and tell the operator afterwards; it now opens the same kind of
dialog the altitude change uses, because the view marks takeoff as carrying a value and QML has
always let the operator choose one.

**The flight actions are offered by the core, and the `readyToArm` gap is closed.**
`guidedAvailability` is deleted. `view.guidedActions` says whether each action is hidden, ready
or blocked and why, so the head no longer decides from `armed`, `flying`, `guidedModeSupported`,
`takeoffVehicleSupported`, `fixedWing` and two flight-mode name comparisons whether a button
should work. Arm was enabled whenever the vehicle was disarmed; it is now blocked with the
vehicle's own reason when the arming report refuses, and a blocked action reports that reason
where a refusal would go rather than doing nothing.

Scoped deliberately: the view offers fourteen actions and this head wires the five it can
invoke. Rendering a button for `grab`, `release` or `emergencyStop` with nothing behind it would
be worse than not showing it, so the rest stay unoffered until each has a working invoke.

**The camera controls read `view.camera`.** `shutterFor` takes the served camera instead of
five separate bridge reads, `cameraModeLabel` is gone, and the capture-mode constants went with
it. One rule survives the move and is worth naming: **an undefined mode is not photo.** The core
reports `modeKnown` separately from `mode`, so a camera that has not said which mode it is in
offers no shutter rather than a photo button that might start a recording.

Verified on the wire: the shutter sent `MAV_CMD_IMAGE_START_CAPTURE`, the mode chip sent
`SET_CAMERA_MODE`, and the row went from SimCam/Photo/Take Photo to SimCam/Video/Record.

Worth recording how the tap was aimed, because two attempts hit nothing first. Computing screen
coordinates by scaling a cropped screenshot put the tap 100 px below the button. **Finding the
button by its own colour in the full-size capture** - the only large lavender pill in that band -
gave its centre directly, and the first tap after that landed. Measure the target; do not derive
it from a resized picture of the target.

**The `view.setup` migration was attempted, reverted, and the reason I gave for reverting was
wrong.** `SetupPages.kt` is 206 lines of per-firmware parameter tables and `view.setup(<page>)`
replaces them. Every control on the Power page came back with an empty `name` and `label` and a
zero `value`, and I reported that as the core failing on Android.

**It was not.** Read after `parametersReady`, `view.control` on this handset returns
`RTL_ALT` with `label "RTL Altitude"`, `name "RTL_ALT"`, `display "1500"`, min 0 max 8000 -
correct, through the Rust-to-Qt hop and the nested-parenthesis argument. `BATT_MONITOR` returns
the default fact because **this simulator does not define it**. Every control on that page was
a parameter the vehicle does not have, and the empty answer was the right one.

The check that would have prevented the false report was one grep of my own simulator's
parameter list. Instead a peer spent a round reproducing it on macOS. **Before reporting that a
producer is broken, confirm the input it was given exists.**

The migration itself was sound and **has now landed**. With an AAR carrying the core's
drop-the-default-fact change, `view.setup` serves the groups and pages and `view.setup(<page>)`
serves the sections with every control decoded; `ParameterForm` takes a page name rather than a
table of parameter names.

The pages read better than they did. Frame showed `FRAME_CLASS` and `FRAME_TYPE`; it now shows
"Frame Class" and "Frame Type (+, X, V, etc)" with pickers, because the label is the core's
instead of the raw parameter name the table happened to carry. And a page whose parameters the
vehicle lacks says so: the old form rendered a row per table entry whatever the vehicle
reported, so Power showed `BATT_MONITOR` and `BATT_CAPACITY` as 0 on a vehicle that has
neither.

`SetupPages`' tables are dead in production but stay for now - `hasNativeSetupPage` is pinned by
another session's uncommitted test, and deleting it would break their work. The `path` is correct and carries the parameter -
`vehicle.parameterManager.getParameter(-1,BATT_MONITOR)` - so the section titles and notes
render and the rows cannot be labelled or read.

Three attempts were spent guessing at the shape from the outside: first the rows drew blank,
then filtering on `name` emptied the page, then filtering on `label` emptied it too. That was
three guesses too many. **One temporary log line printed the served JSON and settled it in one
build** - the honest move was available from the first failure and was not taken until the
third.

Reverted rather than left broken: the tables work and the Setup pages read parameters, which
is worth more than a migration count. The finding is the core's to chase, and it is a
one-vehicle-on-Android question, since the recorded contract has these fields populated.

**Every decoder audited against the contract, and one real finding.** Having written one test
that asserted a value the core never sets, the question was whether that was a habit. It was
not: checking every JSON key each of the seven view decoders reads against the recorded shapes,
and every enumerated string against `view.contract`, came back clean. Battery's
`normal/caution/warning/critical` and vibration's `normal/warning/danger` both match; the only
keys not in the contract are the log entry fields, which the contract cannot describe because
the recording had no logs, and those were read from the producer and match it exactly.

The one finding was worth the pass. The inspector's field list **rebuilt its own bridge path**
as `mavlinkInspector.activeSystem.messages.<index>.fields`, using an index that now comes from
the core - and the core enumerates `mavlinkInspector.systems.0.messages`. Those are the same
object with one vehicle and not the same by guarantee; with a second vehicle the detail could
have shown another system's message under this one's name. Every message already carries its
own `path`, so nothing needed reconstructing. **A head that rebuilds an identifier the core
already gave it has invented a second source of truth**, and this one had been latent since the
migration an hour earlier.

**The confirmation gap is closed, and closing it caught an invented test.** The picker set any
mode it was asked for, including the ones the core marks `needsConfirm` - the modes that fly
the aircraft somewhere on their own. A mis-tap in a drop-down should not start one. The dialog
carries the core's own summary for the mode rather than wording invented here.

The verification went wrong first, and usefully. Tapping Acro switched straight through with no
dialog, which looked like the feature not working. It was not: the core's rule is
`flying && (mode == rtl || mode == land)`, so Acro never needs confirming and a disarmed
vehicle never needs it at all. **My test had asserted `needsConfirm` on Acro** - a value I made
up when writing the fixture and never checked against the producer. It passed, and would have
gone on passing while describing a rule the core has never had. Armed and flying, RTL opens
"Switch to RTL? Climbs, returns home and lands" and changes mode only on Switch.

A test written from an assumed shape tests the assumption, not the system. The producer or the
recorded contract is the only honest source for what a field contains.

**Ninth migration: the flight mode picker, and a capability rather than a swap.** The picker
listed every mode the vehicle reports in one flat drop-down, so Acro, Circle, Drift, Sport and
Flip sat between Land and the modes an operator actually reaches for. `view.flightModes`
separates the everyday set from the folded one, marks the current mode and says whether the
vehicle will accept a change at all. The current mode carries a tick, and "More modes" reveals
the rest - what the QML picker has always done and this one never did.

`needsConfirm` is served for modes like Acro and **is not honoured yet**: the picker sets any
mode it is asked for. Recorded as a gap rather than half-built.

The inspector's row title is the core's now too, so `inspectorRowLabel` is gone. That rule
lived in the head for about twenty minutes and was wrong in it - it measured uniqueness over
the *filtered* rows, so typing in the filter box could drop the component id from a label. The
core measures over the system. Two heads would each have made that choice separately, and one
of them would have got it wrong; that is the argument for the core owning it, stated better by
the bug than by any reasoning beforehand.

**Eighth migration: the inspector, which crashed on its first run.** `parseInspectorMessages`
and `formatRate` are deleted; `view.inspector` serves each message with its rate formatted and
a path that identifies it.

The crash is the part worth keeping. The view serves messages **per component**, and this sim
carries two camera components, so `CAMERA_CAPTURE_STATUS` arrives twice and a `LazyColumn`
keyed on the message name threw *Key "CAMERA_CAPTURE_STATUS" was already used*. The old
per-system model never produced a duplicate name, so nothing had ever been keyed against one.
The recorded contract could not have warned about it either: MockLink has a single component,
so the fixture's message list has no duplicate names in it.

That also retired a guard written earlier the same night. `openMessageIn` re-resolved the open
message **by name** so a rebuilt model could not show one message's fields under another's;
the moment messages arrive per component, a name stops being unique and the guard has to key
on path. A correct fix can be invalidated by a change in what the data means, not only by a
change in the code around it.

And two identical rows told the operator nothing about which camera they came from. A row
whose name appears more than once now carries its component id; a unique name stays bare.

**Seventh migration: the battery, and a question asked before the work.** `batteryLevel`,
`batteryText`, `firstBattery`, the four charge-state constants and the 98.9% rounding rule are
deleted. `view.battery` serves the level and the text.

The order matters more than the result. The core session had said "time remaining and charge
state stay Qt-side for now", which could have meant the view no longer carries `chargeState` -
and migrating on that reading would have silently dropped the charge-state branch and left the
strip threshold-only. **It would have looked correct**, because this sim's battery reports a
state and a percentage that agree. Asking first cost one message; finding out afterwards would
have cost a regression this rig cannot see.

Both branches then verified on the handset: a LOW state at 90% renders Warning, so the state
beats a healthy percentage, and an undefined state at 70% renders Caution, so the threshold
path still runs. Measured off the pixels, hue 36 against hue 46, because those two ambers are
not distinguishable by eye.

One regression caught in the doing: the core's `text` is the primary value alone and the pack's
`secondaryText` carries the voltage, where the old code joined them itself, so the first
version showed "90%" where it used to show "90% · 11.10V".

**Sixth migration: the log list.** `parseLogEntries` and `formatLogTime` are deleted;
`view.logs` serves each entry with its size and time already formatted, and serves
`canRefresh`, `canDownload`, `canCancel`, `canErase` and `anyDownloaded` instead of the screen
deriving them from a busy flag and a selection count.

**And it found a hole in the contract.** `entries` is recorded as `["empty"]`, because the
recording was taken against a vehicle with no logs - so the fixture describes the list and says
nothing about the fields inside it, which is precisely what a decoder needs. The shape had to
be read from the producer. This is the same blind spot as the screens further down, one level
up: a contract recorded against a vehicle that never sends a thing cannot describe that thing.
A recorder needs a vehicle that has logs, has multiple batteries, has an unhealthy sensor -
not only one that is connected.

**Fifth migration: the guided altitude.** `altitudeRange`, `altitudeDelta`,
`altitudeChangeSummary` and the 0.01 m deadband are deleted; `view.guidedAltitude` serves the
range, the sentence, the metric delta and whether the firmware will act on the change. The
head draws a slider and a dialog and nothing else.

Two details it was worth getting right. The argument form is called when the slider **settles**
and again at **confirm**, never per frame - a Compose slider fires continuously, and each call
is a hop to the Qt thread. And the delta actually sent is the one from the confirm-time call,
because the aircraft is climbing while the operator drags: a delta computed against the
altitude at drag time is already stale when it is sent.

Verified on the handset: "already at 25.0 m and will not move" with the button disabled, then
"will climb 43.3 m to 68.3 m" after a drag, then exactly one `SET_POSITION_TARGET_LOCAL_NED`
with frame 7 and `z=-43.29`.

**Two open items closed by rebuilding, and one self-inflicted lesson.** An AAR at
`9491c6132` starts, the whole regression passes, the four-name argument form resolves now that
the watch splits on top-level commas only, and `FLTMODE1` through `FLTMODE6` are all editable
again - the uninitialised `readOnly` is fixed, so the "Read-only" caption appears only where a
parameter genuinely is.

The lesson: the first attempt showed the parameters *still* read-only, and the reason was that
the APK had not been rebuilt after the AAR was replaced. The AAR is a file dependency; Gradle
has to repackage it. This is the same stale-artifact trap recorded further down, self-inflicted
within an hour of writing it up, and the tell was identical - a fix known to be in the source
having no effect on the device. **When a verified fix does not appear on the handset, suspect
the artifact before the fix.**

**Third migration: the vibration bands.** `barFraction`, `verdictFor` and the
`VIBE_MAX`/`WARN`/`HIGH` constants are gone. `view.vibration` serves each axis with its value,
its fraction of the scale and its severity, plus both levels and the clip counts; the screen
keeps the drawing, the colours and the words. The scale numbers and the band caption are now
built from the core's levels instead of from literals that happened to agree with them.

This one was **guarded before it broke**, which is the first time that has happened tonight.
The recorded contract marks four fields on this view as nullable - `value`, `fraction`,
`severity` and `worst` - so they were read through `isNull` from the first version rather than
after seeing the word "null" on the handset. A contract that records nullability turns a
class of defect from something found on a device into something known before writing the
decoder.

It still broke two things a fixture could not have caught: the heading read `Vibration ()`
because the served units are blank on this build, and the axes came back `x`, `y`, `z` where
the screen had always shown `X`, `Y`, `Z`. Both are the seam between an identifier the core
serves and a label a person reads.

**`optString` returns the string `"null"`, and that is how a view field lands in the UI.**
The second migration - `flightBlocker` deleted, `view.warnings.armingBlocker` in its place -
worked first time and then showed the word **null** in red once the blocker cleared, because
`org.json`'s `optString` renders a JSON null as the literal four characters rather than as
absence. Every optional field on every view path has this trap waiting. Read anything that can
be absent through an `isNull` guard, and pin it with a test: this was caught by looking at the
handset a minute after it looked right, not by the suite.

**The first head logic is gone, replaced by the core.** `TelemetryRow` reads
`view.instruments`; `telemetryLabel` and `telemetryValue` are deleted. The view serves each
reading already formatted with its label, units and a missing flag, so the head joins a value
to its units and draws. Verified against the sim: 25.0 m, 8.1 m/s, 1.2 m/s, --.-- m, 268 deg,
120.0 m.

Two things came out of doing it. The row **wraps** now, because the count belongs to the core
and not to the head: the no-argument view serves six instruments where this row had hardcoded
four, and six in a fixed row collided - "Ground SpeedClimb Rate" ran together and the sixth
spilled off the edge. A head that assumes a served list's length is a head that breaks when
the core changes its mind.

And **multi-argument view paths cannot be watched yet**. `qgc_qt_watch` splits its path list on
commas, so `view.instruments(a,b,c,d)` is shredded into four fragments that resolve to nothing
and the row renders empty. That is how the failure presents - not an error, an empty screen -
which is why the first migration was chosen to be one whose absence is visible. Reported; the
no-argument form has no commas and is correct meanwhile.

**The Rust core runs on the handset.** The first AAR built with `QGC_RUST_CORE` on (cargo-ndk
cross-compiling `libqgc_core.a` into `libAircastQGC`) starts, and the whole regression passes
against it: battery, RC, sats, HDOP, the vibration bands, four status texts, the log list and
both camera components. Eight `qgc_core_` and eight `qgc_bridge_` symbols are exported from the
shipped `.so`. This was the one check the core session could not run, and it is the gate for
every path in `view-shapes.json` reaching the handset.

**Open defect: some parameters come back read-only, and which ones changes between runs.**
The parameter list drew a read-only fact as plain text, which looks exactly like a value
nobody has tapped yet - tapping did nothing and nothing said why. Rows now carry a
"Read-only" caption, and that label immediately showed something the screen had been hiding:
`FLTMODE2` through `FLTMODE6` come back read-only while `FLTMODE1` does not, and the set is
not stable - `FLTMODE3` was a working dropdown in one run and read-only in the next. All six
are declared identically by the simulator (`REAL32`, values 1-6) and `FLTMODE1`'s dropdown
lists the very modes the inert rows display, so the enum metadata is present for all of them.

What was ruled out: it is not the sim (identical declarations), not missing enum metadata
(the list contains those modes), and not the row's control choice (plain text is reachable
only through the `readOnly` branch, since the fallback draws a bordered text field). Keying
the row's read on `parametersReady` did not change it, so the guess that rows compose before
metadata arrives is wrong, and that change was reverted rather than left in as decoration.

**And it is not the metadata XML.** With
`qgc.firmwareplugin.apm.apmparametermetadata:verbose` enabled - the category string, not the
C++ variable name, which is the mistake that cost a run - the parser logs every attribute it
reads. Across the whole of `APMParameterFactMetaData.Copter.3.5.xml` exactly two parameters
are marked read-only, `SYSID_SW_MREV` and `SYSID_SW_TYPE`, and `FLTMODE1` through `FLTMODE6`
are each inserted with no `ReadOnly` read at all. `Fact::readOnly()` is nothing but
`_metaData->readOnly()`. So a fact reporting read-only for a name whose metadata never set it
means **the fact is not holding the metadata object parsed for that name** - which is the
shared-record hypothesis, now with evidence behind it rather than as a guess. Confirming that
is in the core's code, not the head's.
**The cause is not established and is upstream of this screen**, which reflects the flag
faithfully. It matters beyond Android: `view.control` serves the same `readOnly`, so whatever
this is will reach every head the same way.

**A true `PipView` swap is blocked, and it is worth saying why rather than leaving it open.**
`AndroidHost.qml` hosts the whole `FlyView`, not a map component, so shrinking the Qt view to
an inset would shrink that entire UI with its own overlays inside it. The swap is not a
standalone piece of work; it arrives when the Fly tab stops being QML.

What was not blocked was the state either side of it. Expanding the video hid the whole overlay
column, so an operator flying on the camera lost the shutter, the mode chip and the camera
picker at the moment they are the only controls that matter. And in the collapsed layout the
same column drew *over* the video inset, because it starts at the top left and is wide enough
to reach under an inset pinned to the top right. Both are fixed: the column starts below the
inset's height when collapsed, and stays on screen when the video is expanded.

**The Android AAR had not been rebuilt in a long time, and rebuilding it broke the app.**
Nothing verified the claim that the C ABI reroute left Android unchanged, so the rebuild was
the check. It did not compile: `DebugApiServer.cc` includes `QGCBridgeC.h` inside
`#ifdef Q_OS_MACOS` while calling `qgc_bridge_*` unguarded, and `QGCBridge.cc` used
`kJniQGCBridgeClassName` unqualified from the anonymous namespace. Neither is reachable from a
macOS build, which is why both had sat committed.

With those fixed the AAR builds, and the app then stops on the Android splash screen. The first
cause was ours: `AndroidInit.cc` calls `QAndroidApplication::hideSplashScreen`, which Qt
resolves by JNI against the current activity, and Qt's `QtActivity` has that method while this
app's plain `ComponentActivity` did not - `JNI_OnLoad` threw `NoSuchMethodError` and the native
library never finished loading. A no-op method fixes that, and `JNI_OnLoad` now completes with
every native function registered and Qt started.

**The stall was a deadlock in the reroute, and it is fixed.** `qgc_qt_watch` wrapped
`QGCBridgeCore::watch` in a blocking hop to the Qt thread, where the JNI head had always called
it directly and the core posts it. During start-up Qt's Android plugin blocks on the main
thread while the head registers its first watch from that same main thread: the main thread
waits for Qt, Qt waits for the main thread. That is exactly the shape of "the main thread goes
quiet just after qt started". Diagnosed by the core session from that description; the fix is
`1535fb654`, and an AAR built at that commit starts.

Everything this document claims about the Android screens was **re-verified against that AAR**,
not the stale one: battery, RC, GPS, vibration bands, message log, log list and both cameras.
The re-run matters, because until it was done every screen result of that night rested on
native code older than `fork/main`.

The lasting lesson is about the artifact, not the deadlock. The AAR had gone unrebuilt long
enough to hide two compile errors and a start-up deadlock, and it hid them precisely because it
kept working. A prebuilt artifact that nobody rebuilds stops being a convenience and becomes a
place for breakage to accumulate unseen.

Auditing the rest of the sim against what the screens read turned up two more. The vehicle
message log had never carried a message, because the sim sent no `STATUSTEXT`; fed four of
varying severity it was **correct** - counts, severity classification, colours in the right
order, and `Batt & temp < 40C` with its entities decoded. Worth stating plainly, since the
point of an audit is not to find a defect in everything it touches.

The battery had never rendered, because the sim sent no `BATTERY_STATUS`, and that one did
have a defect. Sampling the strip's actual pixels: Caution hue 46 saturation 1.00, Warning
hue 36 saturation 1.00, Critical hue 0 saturation **0.69**. The hues climb correctly and the
saturation climbs and then drops at the step that matters most, so the worst state was the
palest thing on the strip - the same shape as the vibration bug, in a different screen, found
by measuring rather than by eye. Both now use one saturated red.

The last unexercised path, RC RSSI, turned out to be **correct code and a wrong test**, which
is worth as much as a defect. Sending `RC_CHANNELS` with `rssi=80` displayed 31%, and the
temptation was to call that a scale bug. It is not: `APMFirmwarePlugin` rescales
`channels.rssi` by `/254.0 * 100` before `Vehicle` ever sees it, so on ArduPilot the field is
0-254 on the wire and 80 becomes 31.5. Sending 203 displayed 79%, against a predicted 79.9.

The mispredicton is the lesson. `Vehicle::_remoteControlRSSIChanged` carries the comment
`0 <= rssi <= 100`, and that reads like the wire contract; it describes the value *after* the
firmware plugin has already converted it. A comment one layer below the transform is not a
statement about the input.

Two corrections to the method while doing it. The thresholds are 80 and 60, not the 30 and 20
assumed, so a first pass labelled a Warning reading as Caution and never rendered Caution at
all; the levels have to be driven from the real settings, not from plausible ones. And
`.rawValue` had been committed without re-running it on the handset - it is correct, a scalar
read comes back as `{kind: "value", value: …}`, but that was confirmed after shipping rather
than before.

Five screens tonight looked finished because nothing exercised them: the GPS branch behind an empty
sensors-present mask, the log list behind a sim with no logs, vibration behind a sim with no
vibration, the message log behind a sim with no `STATUSTEXT`, and the battery behind a sim
with no `BATTERY_STATUS`. **An empty screen is not evidence that a screen works.** The question to ask
of any surface that looks correct is what the simulator never sends, because that is exactly
the set of paths no one has seen.

**The no-GPS-lock warning was missing and the sim could not have caught it.** `VehicleWarnings.qml`
shows two things - the vehicle's prearm text, which the banner already had, and a no-GPS-lock
warning, which nothing in the native head had. It is gated on `requiresGpsFix`, which is the
GPS bit of the vehicle's own sensors-present mask, not on the fix type: a vehicle with no GPS
is not told it is missing a lock it never had.

Worth recording because it nearly read as a working feature: the branch had never been
reachable in testing, because the sim sent an empty sensors-present mask, so `requiresGpsFix`
was false no matter what the fix type said. The first run showed no banner and the honest
reading of that was not "the code is wrong" but "nothing here exercises the condition".

**A Fact's `value` is the operator's display unit, and commands take metric.** The bridge
serves `cookedValue()`. `guidedModeChangeAltitude` takes metres, and QML converts at the call
site before calling it. The first version of this slider fed the cooked delta straight to the
command, which would have commanded 43.5 m for an operator asking to climb 43.5 ft.

It does not misbehave today, and the reason is worth recording: with Vertical Distance set to
Feet the handset still reports `altitudeRelative` as `25.0 m` - value, valueString and units
all metric, across a process restart. Cooked translation is not happening in this build, so
cooked equals raw and the original code was safe by accident. Anything that decides a command
reads `rawValue`; `value` is for display only.

`altitudeRange`, `altitudeDelta` and `altitudeChangeSummary` are **Rust view-state
candidates** under the shared-core rules - Swift will otherwise copy all three. The answer a
head wants is: given the settings and the current altitude, what range, what delta, and what
sentence.

**Getting a log out of `CameraControlLog` needed one more correction.** It is an old-style
category name with no `qgc.` prefix, so the blanket `*Log.debug=false` rule silences it and
the settings key is the bare `CameraControlLog=true`. Two different key shapes in one
`[LoggingFilters]` group, and neither is the `.debug=true` form the rules print.

**How this was got wrong, twice, before it was got right.** Worth keeping because the
failure is the one this document records most often and here it cost a working feature.

The sim's camera was discovered from the first attempt. I concluded otherwise because I read
the camera log through `head -10`, which showed only the retries for **compId 1** — the
autopilot, which QGC also probes as a possible camera and which the sim never answers as.
Those retries are expected and permanent, and I let them stand for the whole exchange. The
screenshot I checked alongside was cropped to a region where camera controls do not render,
so its emptiness confirmed nothing.

Reaching the log at all took two corrections to the settings key, and they are different
from each other:

- `[LoggingFilters]` entries are the **bare** category name. `categoryLoggingOn` does
  `settings.value(category)` and the rule builder appends `.debug=true` itself, so writing
  `qgc.camera.qgccameramanager.debug=true` puts the generated-rule format into the setting
  and the lookup misses silently. The ini being edited had four correct examples in it.
- `CameraControlLog` has **no `qgc.` prefix at all** — an old-style name that the blanket
  `*Log.debug=false` rule silences, whose key is the bare `CameraControlLog=true`.

So one group holds two key shapes, and neither is the form the rules print. The
"Filter rules" line the app logs at startup is the only confirmation a filter took.

**Obstacle distance is built, and finding a bridge defect along the way.** QGC draws a
proximity ring from `OBSTACLE_DISTANCE`; on a phone the useful form is a sentence, so the
readout says *"3.2 m right"* and turns red inside twice the sensor's own minimum. Verified
against a sim emitting one return at 320 cm in slot 18 of a 72-slot ring with a 5° increment
— which is 90°, which is "right".

Getting there exposed a real gap. `variantJson` special-cased `QMetaType::QVariantList` and
let everything else fall through to `QJsonValue::fromVariant`, which converts `QStringList`
but **not `QList<int>` or `QList<qreal>`**. Those came back null, so
`objectAvoidance.distances` read as empty while the vehicle was sending obstacle data every
200 ms. It now takes any registered sequential container through `QSequentialIterable`,
which also fixes `joystickConfig.stickPositions` and `MAVLinkSystem.compIDs` — both silently
empty until now.

Worth naming the shape: the message was arriving, the sim confirmed sending it, and the
display stayed blank. **A conversion that fails by producing nothing rather than by failing**
is indistinguishable from a vehicle with nothing to say, and only bisecting the path found
which end was silent.

**And the first version kept naming an obstacle after the sensor stopped.** Nothing in
`VehicleObjectAvoidance` is ever cleared — `_distances` holds the last message for the life
of the vehicle object, and `available` is `distances.count() > 0`, so once true it stays
true. A proximity display that goes on reporting a cleared obstacle fails in the worst
direction available to it.

`msSinceUpdate` is now on `VehicleObjectAvoidance` (additive, nothing else reads it) and the
readout hides after three seconds of silence; never having heard from the sensor counts as
stale rather than fresh. Verified by having the sim stop sending obstacles while staying
connected and streaming everything else — the readout disappears.

**On-screen RC is now built.** `rcControls` is a JSON list of controls bound to RC channels
— sliders, buttons, three-position switches, momentaries — for gimbals, lights and payload
releases rather than for flying, which is why it is less dangerous than its name suggests.
They render natively and send through `vehicle.setRcChannelOverride`, the same call
`CameraControlLayer.qml` makes.

Verified on the wire rather than on screen, which is the only verification worth having
here: with the sim logging `RC_CHANNELS_OVERRIDE`, pressing Light put **channel 8 at 2000**,
dragging Gimbal put **channel 7 at 1402**, and press-and-release on the momentary Drop gave
**channel 9 at 2000 then 1000**. Switch3 was not in the test configuration, so its rendering
is unit-tested and not yet exercised on hardware.

Parsing drops a control with no channel or an unrecognised type rather than defaulting,
because one that silently drives channel 0 or guesses it is a slider is worse than one that
does not appear. The layer is hidden without an active vehicle, since every control on it
sends to one.

**The first version commanded a channel before anyone touched it.** `LaunchedEffect(pressed)`
runs on first composition with `pressed` false, so each momentary drove its channel to
minimum the moment the Fly view appeared — an RC override the operator never asked for. It
latches now: the release only goes after a press, and opening the view produces zero
`RC_CHANNELS_OVERRIDE` where it produced a stream before.

**Audited every other effect that writes**, since the defect generalises: an effect that
commands rather than reads fires once at composition and looks exactly like nothing
happening. Two in the whole app. `MainActivity`'s `setNativeRendering` configures this app
rather than the vehicle, so composition is the right moment. `LogDownloadScreen`'s refresh
does command the vehicle, and is guarded by `shouldAutoRefreshLogs(hasVehicle, hasEntries,
busy)` — a named predicate with four test cases. Recorded as a negative result because the
next person adding an effect will not re-derive it.

**That defect was in the log of the run that verified the feature.** Eleven channel-9
minimums against five maximums, when one press and release should be one of each. I read the
log for the values I expected and not for the counts that were in front of me — the same
shape as reading a `tail` and calling it the whole. **Removing them to ship a native map would
be a regression of exactly the kind this plan refuses elsewhere** for plan files.

The video duplication noted above — QML's inset and the native one both drawing — ends at
this switch and not before, which is another reason to do the switch as one piece of work
rather than creeping up on it.

**Gate (HW):** every guided action verified on PX4 and ArduPilot. Confirmation control cannot be
actuated accidentally. Sub-200 ms glass-to-glass on WHEP. A 30-minute flight with flat memory.

### The Actions sheet

The row on the Fly tab carries the six actions an operator reaches for constantly — arm,
takeoff, land, return, speed, height. The core offers fourteen. The other seven had no
native home, so `FlyViewToolStrip`'s "Actions" hamburger becomes a sheet listing exactly the
offers the core marks shown and this head knows how to send: start mission, continue
mission, pause, abort landing, grab, release, emergency stop. The button appears only when
that list is non-empty, so a vehicle with nothing extra to offer shows no control rather
than an empty sheet.

The head does not decide what belongs there. Each row's title, one-line prompt, destructive
flag and blocked reason are the core's, and an offer the core hides is not drawn. Verified
on the handset against the APM sim, each by what the vehicle received rather than by the
button changing:

| action | evidence |
|---|---|
| Grab | `MAV_CMD_DO_GRIPPER` (211) with action 1 |
| Release | `MAV_CMD_DO_GRIPPER` (211) with action 0 |
| Pause | flight mode became Brake, and Pause then left the sheet on its own |
| Emergency Stop | `MAV_CMD_COMPONENT_ARM_DISARM` (400) with the 21196 force magic |

Start mission, continue mission and abort landing are wired but unexercised: the first two
need a plan loaded and the third a fixed-wing on approach, and the sim gives neither.

**An action this head cannot send is not offered.** The sheet filters on a named list rather
than on "everything the core shows minus the row", because the other shape draws a row for
any action the core adds later and does nothing when it is tapped. A dead row on a flight
screen is worse than a missing one.

**Pause is not a height change, and gating it like one made it impossible.** The confirm
button read `view.guidedAltitude.sends`, which is false when the target equals the current
height — exactly the case where an operator wants to stop where they are. QGC's own
`guidedModeChangeAltitude(0, pauseVehicle: true)` switches to the pause flight mode and then
returns before sending a position target, so a zero delta is the *normal* pause, not a
no-op. The head now sends when pausing regardless of delta. The core is still the better
home for this rule: `view.guidedAltitude` reports a change it would send, and pause needs
`sends` to mean something else. The core took it as `view.guidedAltitude(target,pause)` in
`e94b5c2f3` and the head clause is gone — the head now asks with the intent and obeys `sends`
either way. On the handset the dialog reads "The aircraft will stop and hold at 25.0 m.", the
confirm is live at a zero delta, and the sim logged `MODE -> 17` (Brake).

**The slide-to-confirm label sat under the thumb.** "Slide to emergency stop" rendered as
"lide to emergency stop" — the instruction on the most dangerous control in the app was the
one you could not read. The label now centres in the track to the right of the thumb rather
than in the whole track. Short labels are unaffected.

### Video sources, and a reason instead of "No video"

`VideoTilesLayer.qml` is 354 lines, and most of it is the dock, the tuck, the grid and the
pip-width negotiation with the overlay rig — layout that belongs to the rig, not to video.
What an operator actually needs from it is smaller: see which streams exist and switch to
one. `view.video` already serves that — `cameras[]` with a slot, status, connecting,
recording and configured flag, plus `activeSource` and `multipleSources` — so the picker is
head-only work. Chips appear only when the core reports more than one configured stream, and
tapping one calls `video.setActiveVideoSource(slot)`.

Verified on the handset with a second RTSP source added to the device ini: the chips read
"Camera 1 · connecting" and "Camera 2", tapping the second moved both the selection and the
connecting mark onto it, and the ini was restored afterwards so the shared rig is unchanged.

**The inset said "No video" whatever the reason.** It also keyed on `video.streaming`, which
is true once the pipeline starts and says nothing about whether a frame ever arrived — a
started-but-dead stream would hide the placeholder over a black surface. It now reads
`view.video`, hides on `decoding`, and shows the core's sentence: "No stream URL is set.",
"Waiting for a stream.", "Not streaming.", "This build cannot show video." That is one more
answer the head had been re-deriving and getting slightly wrong.

**The core names the streams "Camera 1" and "Camera 2" and throws the configured name away.**
`VideoManager::cameraName(index)` returns the operator's own name and already falls back to
exactly that string, so the core can serve real names with no change in behaviour for an
unnamed source. Raised with the core session; the head must not look the name up itself.

### The Fly tab flies on the native map

`VehicleMap` had been sitting in `map-spike` behind the Plan tab while the Fly tab still
showed the QML `FlyView`. `FlyMap` wraps it for flight: the vehicle marker, its trail and
follow, plus the mission, fences, circles, rally points and surveys read back through the
same `PlanBridge`/`FenceBridge`/`SurveyBridge` the Plan screen uses — read-only, `editable`
false, polled every two seconds rather than the Plan screen's 700 ms, because a plan does not
change much in flight. It is drawn over the QML view whenever the Fly tab is showing, so
nothing of QGC's QML is visible on this head any more.

The QuickView stays in the tree rather than being removed: `VideoManager` initialises against
a `QQuickWindow`, and the video inset is worth more than the frames the covered view costs.
Removing it is a separate change with its own verification.

**The vehicle sat behind the controls panel.** `follow` centred it on the whole map surface,
and the bottom 40% of that surface is under the flight-controls panel, so the aircraft rode
the lower edge of the part you can see. `VehicleMap` grew a `cameraBottomPx` that calls
MapLibre's `setPadding`, defaulting to zero so the Plan screen is untouched, and the Fly tab
measures the panel with `onSizeChanged` and feeds its real height in. The marker now sits in
the middle of the visible band. This is the one piece of `OverlayRig`'s job that Compose does
not do for free.

A matching top inset for the overlay column was deliberately not added. That column changes
height as the obstacle readout and the video source chips come and go, and a camera that
jumps whenever an overlay appears is worse than a marker that occasionally sits near a chip.
The panel at the bottom is always there and always the same size, which is why its inset is
safe.

`MapBridge` is a singleton with one watch set and a `release()` that clears all of it, so two
map screens composed at once would tear down each other's watches. The Fly and Plan maps are
mutually exclusive by construction — the tab decides which one exists — and the Fly → Plan →
Fly round trip was verified on the handset with both maps drawing the vehicle and its trail
afterwards. It is still a latent trap for whoever composes two maps first.

**Named video sources arrive.** With `view.video` titles now coming from
`video.cameraName(slot)`, a source configured as "Thermal" reads as Thermal and an unnamed
one still reads "Camera 1". Confirmed on the handset against the same two-source ini, which
was restored afterwards.

### The inset swaps

With the map native, the thing that blocked a real `PipView` is gone. Tapping the video inset
puts video full screen and the *map* in the corner; tapping the map inset puts it back. Both
composables keep their position in the composition and only their modifier changes, so
neither the `MapView` nor the video `SurfaceView` is torn down and rebuilt by a swap — the
draw order is settled with `zIndex` instead, which is also what makes the corner view take the
touch.

**The inset is the control; the main view is not.** Before, a tap anywhere on full-screen
video collapsed it, which in flight means any stray touch throws away the picture you were
looking at, and a close button in the corner existed to give the gesture a visible partner.
Now only the corner responds — on either view — which is the convention every picture-in-
picture uses, so the close button is gone rather than sitting under the map inset fighting it
for the same corner.

The map's camera inset is dropped to zero while it is the small view: the flight-controls
panel does not cover a corner inset, and padding a 190 dp box by the height of that panel
would push the aircraft out of it entirely.

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
