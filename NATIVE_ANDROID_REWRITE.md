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

**"Built" is not "at parity" for the Inspector.** `MAVLinkInspectorPage.qml` shows *Actual
Rate* and a *Set Rate* combo (`msgRateCombo`) that calls `Vehicle::setMessageRate`.
`InspectorScreen` shows the actual rate — it reads `rateText` — and offers no way to set one.
That is a capability of the page being replaced, missing from the page marked built, so the
QML deletion in the gate below would lose it.

Nothing is missing from the core. `view.inspector` already emits `targetRateHz` and
`targetRateTitle` per message and a top-level `rateChoices` list of `{rate, title}` built from
`RATE_CHOICES` — exactly the shape a picker needs. **The macOS head reads all three and renders a
rate Picker** (`AnalyzeWindow.swift:347`, `MavlinkInspector.swift:31`), so this is built on one head
and missing on this one, not orphaned. An earlier version of this section said "read by no head";
that was wrong, and wrong in a way my own sweep could not have caught, because the sweep only ever
looked at the Android head.

**There is no blocker. I reported one and it was a wrong door.** `Vehicle::setMessageRate` is
indeed plain `public:` at `Vehicle.h:1340` and so invisible to `invokePath`, which scans
`QMetaObject` and sees only signals, slots and `Q_INVOKABLE` methods. I concluded from that the
feature needed a `Q_INVOKABLE` on `Vehicle.h` and therefore an AAR rebuild across ~120 translation
units. It does not. `MAVLinkInspectorController::setMessageInterval` is `Q_INVOKABLE`
(`MAVLinkInspectorController.h:70`), resolves the message from `_activeSystem->selectedMsg()` and
calls `vehicle->setMessageRate` internally; `setActiveSystem(int)` is invokable beside it. The head
sets the selection as state and invokes the controller with a rate alone. So the work is head-only,
needs no rebuild, and the reachability check I ran — "is *this* method invokable" — asked about the
wrong object. The right question was whether *any* invokable path reaches the behaviour.

**Built and verified** (`cc0ed8c`, with `53b9a84cc` in this repo). Picking a rate writes the message
selection and then invokes `mavlinkInspector.setMessageInterval`, in that order on one background
thread, because the controller resolves the message from the active system's selection rather than
taking it as an argument. Against the sim: ATTITUDE at 10 Hz puts `SET_MESSAGE_INTERVAL` on the wire
as `p1=30 p2=100000`, at 2 Hz `p2=500000`.

It did not work first time, and the reason was a real bug one layer down. The invoke failed with

    QMetaMethod::invoke: cannot convert formal parameter 0 from int in call to
    MAVLinkInspectorController::setMessageInterval(int32_t)

`moc` records a parameter by the type name as written, and `int32_t` is not a registered metatype
alias, so the method was `Q_INVOKABLE` and simultaneously uncallable through the meta system. It was
the only `Q_INVOKABLE` in the tree declared with a fixed-width integer type — an outlier, not a
convention — and it now takes `int`, the same type everywhere QGC builds. QML calls this method the
same way, so the QML *Set Rate* combo was plausibly broken too; not tested, and worth checking before
the QML is deleted on the assumption it worked.

**The rate label does not update on this rig, and that is the sim rather than the app.** QGC updates
`targetRateHz` only from a `MESSAGE_INTERVAL` the vehicle sends after acking the command —
`_setMessageRateCommandResultHandler` re-requests it on `MAV_RESULT_ACCEPTED`. `apmvehicle.py` answers
`REQUEST_MESSAGE` for `AUTOPILOT_VERSION` only and never sends `MESSAGE_INTERVAL`. Confirming from the
vehicle rather than from the tap is the rule `attemptCommand` already follows for flight commands, so
the label must not be made optimistic to make the rig look right.

**That blocker is not systemic, which is worth stating because I assumed it would be.** The same
sweep flagged `hasZoom`, `zoomLevel` and `canChangeMode` on `view.camera` as emitted and unread, and
the head has no zoom control at all — the only `zoom` in it is `cameraZoomChannel`, an RC channel
mapping, which is a different thing. But `MavlinkCameraControl::stepZoom` **is** `Q_INVOKABLE`
(`MavlinkCameraControl.h:164`) and `zoomLevel` is a `Q_PROPERTY` with a `WRITE` setter, so both are
reachable through the bridge today. QML uses it at `FlightDisplayViewVideo.qml:387`. Camera zoom is
therefore an unbuilt control, not a blocked one, and needs no rebuild.

It is also not verifiable on this rig, and more completely than "the call would do nothing":
`SimulatedCameraControl::hasZoom()` returns `false` and its `stepZoom` is an empty override, so a
control gated on `hasZoom` would never render here at all, and one that ignored the gate would call
into a no-op. Building it here would ship a control that
cannot be shown to work — the same trap as judging a screen the rig can only render one way. It
needs a real camera, which puts it with the other gates that need hardware.

**Gate (HW):** log download from real hardware; chart values match the Qt build. `src/AnalyzeView/`
QML deleted.

**The download half is walked against the sim as of 2026-09-11**, which does not discharge a
hardware gate but does move it from "never exercised" to "exercised everywhere except on hardware".
`apmvehicle.py` answers `LOG_REQUEST_LIST`, `LOG_REQUEST_DATA` and `LOG_REQUEST_END`, so the whole
protocol runs. The screen listed `Log 0 · 4.0 KB · Available` and `Log 1 · 10.0 KB · Available` with
timestamps, selecting one relabelled the button `Download (1)`, the transfer showed a Cancel while it
ran, and it finished as `Downloaded` with the destination named. On disk:
`log_0_2025-9-8-01-20-00_2.bin`, **4096 bytes — exactly the size the list advertised.**

What that leaves for the hardware gate is narrower than it was: not whether the flow works, but
whether a real autopilot's log sizes, timestamps and multi-megabyte transfers behave the same. The
2.9 MB file already in that directory from an earlier session says the large case has been seen at
least once.

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

**Radio calibration cannot be driven in this rig.** Stick movement cannot
be simulated *by SITL*: `setRcChannelOverride` is accepted but SITL does not reflect it in
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

### Where the tab stands, 2026-09-12

The entries below are incremental and there are a lot of them. This is the state they add up to, with
what each claim rests on.

**Drawn on the map**, all seen on the OnePlus 6: waypoints with per-kind colours, the dashed route,
fence polygons and circles with vertex handles, rally points, survey areas with transects and handles,
and corridor and structure scans. **Landing patterns are not drawn** and are the only plan item that
is not — the banner names them, and drawing them needs a `KINDS` entry in the core.

**Editing**: drag a waypoint or any vertex, add by long press or toolbar, insert after the selected
item, delete, edit an altitude, rotate a survey grid, import a KML or SHP boundary as any of the three
patterns. Selection drives QGC's own plan view, so the insert-validity refusals describe the operator's
actual insertion point.

**Reading**: every item is reachable from the list, including ones the map cannot draw, and every
measurement on the tab is spelled by the core in the operator's units — list altitudes, the leg
distance and bearing, the terrain band, the fence radius.

**Round trip**: build, upload, clear, download gives back an identical plan, with the wire content read
off the sim rather than the screen. That is not the gate, which wants 200+ waypoints and a flight.

**What the gate still needs**: hardware. Nothing in the list above is blocked on code.

**Known gaps, each recorded in full below**: landing patterns undrawn; the Fly view's obstacle distance
is metres-only because no core view serves it; the "no position" branch for a `DO_` item is unit-tested
and never seen on a device, because this head cannot add such an item; the Add Item gate is
deliberately not wired, because a greyed button on a phone cannot carry the sentence that explains it.

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

~~Still to do: actually draw corridor scans, structure scans and landing patterns.~~
**Corridor and structure scans are drawn** (`aircast-android 66bec7f`); a landing pattern
is not, and is now the only thing the banner names.

Nothing had to be built for the geometry. `geometry_of` has carried `shape` and `property`
for every catalogued pattern since `9cadb96c8` — `"area"` with `surveyAreaPolygon` or
`structurePolygon`, `"line"` with `corridorPolyline` — and `surveysFrom` threw it away by
filtering on `kind == "survey"` **before** looking at what the core had sent. A corridor
arrived complete, with its vertices, and was dropped one line too early. The work was
deleting a filter, not writing a renderer.

What did need care is that an `"area"` closes and a `"line"` does not: **a corridor's path
is a route, not a boundary**, and closing the ring would draw a leg the aircraft never
flies. Vertex drags now go through the item's own property rather than a hardcoded
`surveyAreaPolygon`, and Rotate is gated on the item being a survey, `gridAngle` being a
survey fact — it had been offered on all three and did nothing on two.

**Verified on the OnePlus 6 with a real shape, not an empty one** — the trap this document
records two sections above. A KML polyline imported as a Corridor Scan takes the plan from
1.15 to 3.50 km; the map draws the corridor path in purple between its two magenta
transects; the summary reads `4 items (takeoff) · 10 scan pts · 3.50 km · 11:57`; no banner.

`"survey pts"` became `"scan pts"` in the same change, because the count is every pattern's
transects and a corridor is not a survey.

**Identification is locale-safe as of the core's `24c5de566`; insertion is not.**
`mission.insert` compares against `CorridorScanComplexItem::name`, a `tr()` static, so a
non-English build can draw a corridor it cannot add. Found while sweeping for the defect in
this document's previous section and reported; the core is holding it rather than guessing
on a write path.

### The plan can be read as a list, and building it found an item nothing could show

Every edit used to start by hitting a marker on a canvas — the smallest, most distant target on the
screen and the only entry point, on a head where markers are not accessibility nodes. Tapping the
summary chip now opens the plan as rows: number, name, altitude, and the marker's own colour so a row
and its pin are obviously the same object (`aircast-android 3d01a6b`, `1fd4918`).

**The bench plan reads "3 items (takeoff)" and the map draws 0, 2 and 3.** There is no 1. `missionItems`
drops any item without a plottable coordinate — right for a map, which cannot draw a point that has no
position, and wrong for a list, whose whole job is to reach what the map cannot show. The takeoff in
that plan has no position, so it was invisible on the only screen that edits plans, and a list built on
the map's data would have inherited the blind spot while looking complete.

So the parse and the filter are separate now: `allMissionItems` keeps every item and records whether it
is `placed`; `missionItems` is that filtered. One read, two consumers, each asking its own question —
the same shape as the corridor geometry above, and the third time today that a list was answering two
questions at once.

**The filter had leaked into two more places that were not about drawing**, and only the device showed
it. The selection panel looked an item up in the filtered list, so selecting the unplaced takeoff
produced no controls; and `selectionSurvives` was given the filtered list, so a selection that did land
was discarded by the next one-second refresh. Reading the code found neither — tapping the row and
watching nothing happen, twice, is what found them. Selecting it now offers "Delete #1", which is the
only sensible thing to do with an unplaced takeoff.

**The rig refused that tap, correctly, and the fix was to give it more to see.** `whatsunder.py`
recognises a planning screen by Upload and Download being present; the sheet covers both, so a row
labelled "Takeoff" read as the Fly view's takeoff button. The guard was right to stop — it had no way
to tell them apart. The sheet now names itself "Plan items" and that is a second marker the guard
accepts, which is also better on its own terms: a bottom sheet that opens with no title is a bare list.
The coupling this buys is invisible and worth knowing: `PLAN_ITEMS_HEADING` in Kotlin and the literal
in `whatsunder.py` must stay in step and nothing enforces it.

**One finding from the same review is withdrawn, because I measured it wrong.** I wrote that the Plan
tab spends "21% of a portrait screen on Battery, Sats, HDOP and RC — telemetry the planning task does
not use". The 21% is the whole header down to the map: the vehicle title and flight-mode line (~130 px),
the status strip (~110 px), and the Plan tab's own File row (~70 px) of 2280. **The telemetry strip is
about 5%**, and the rest is the vehicle's identity and the tab's own chrome, neither of which is
telemetry the task ignores. Reclaiming 5% is not worth a per-tab conditional in the shell.

Same error as the ones this document already records twice: a measurement that does not separate the
thing being blamed from what sits next to it. It came from reading a screenshot by eye rather than from
the node positions, which were available the whole time.

### Placing the unplaced takeoff moved the launch point instead, and was reverted

Having made the list show `1 Takeoff · no position`, the obvious next step was to let the operator do
something about it: select the item, long press the map, and `plan.missionItems.1.coordinate` gets the
point. Built, unit-tested, installed, and **reverted after one run on the handset**.

What actually happened: the takeoff still read "no position" afterwards, and the plan's distance went
**1.15 km to 920 m**. So the write did not place the item, and it did change the plan. Item 0 —
Mission Start, which *is* the launch position — had moved.

`TakeoffMissionItem::setCoordinate` explains it: for a multirotor `_initLaunchTakeoffAtSameLocation`
sets `_launchTakeoffAtSameLocation` true, and the setter then does
`_settingsItem->setCoordinate(coordinate)` as well. So writing a takeoff's coordinate writes the
mission's launch point. The operator would have read "Long press the map to place it", long pressed,
watched the item stay unplaced, and had the plan's launch position silently relocated underneath them.

**The unit tests were green and proved nothing about this.** They pinned which branch the long press
takes, which was never the question — the question was what the bridge write does on the other side,
and only the device could answer it. Same shape as the Add Item gate recorded earlier in this
document: a constructed input proves the decode and says nothing about whether the decision is true.

**The write was never broken, and it took three reverts to find that out.** On a rebuilt AAR the insert
succeeds with no refusal, so the core's read-back in `31e5a524a` finds `coordinate.valid == true` —
`setLaunchCoordinate` does reach the item. The list still said "no position". Two reads of the same
item, moments apart, differing by one gate:

    shape()   reads visualItems.N.coordinate and checks .valid            -> true
    at_key()  requires specifiesCoordinate first, then valid, then !(0,0) -> None

And `specifiesCoordinate` is false, from QGC's own metadata:

    src/FirmwarePlugin/APM/APM-MavCmdInfoCommon.json
    { "id": 22, "comment": "MAV_CMD_NAV_TAKEOFF",
      "specifiesCoordinate": false, "specifiesAltitudeOnly": true }

**On ArduPilot a takeoff specifies an altitude and no place.** The core is right to withhold the
coordinate, the map is right to draw no takeoff pin, and QGC's own plan view draws none either.
`coordinate.valid` reads true only because `SimpleMissionItem::coordinate()` returns param5 and param6
whatever the metadata says — worth knowing about that read-back, which confirms the setter ran rather
than that the item is drawable.

So "1 Takeoff · no position" was true and useless: it fired on every takeoff anyone adds, which is the
noise failure this document records three times about the undrawn-items warning. The row shows the
altitude now (`aircast-android 7416fa2`), and "no position" is reserved for an item with neither a place
nor an altitude, where it says something real.

**The cost is the lesson.** Two features built and reverted, three messages to the core session, two of
them sending it after a bug that was not there — all from one correct observation ("the takeoff has no
position") and one wrong inference ("therefore the write failed"). The observation was never checked
against the question *should this item have a position at all*, which QGC answers in a JSON file. Ask
what the correct value is before concluding the wrong one arrived.

~~**The right way is `setLaunchCoordinate`, which I had not looked for**~~ — the setter is fine and the
feature built on it is reverted, because on ArduPilot there is no place to set: (`aircast-android 54e67b5`). It
sets the launch point and, when the takeoff has no coordinate, gives it one — the same place for a
multirotor, offset by the climb-out distance for a fixed wing. That `if (!coordinate().isValid())`
branch is precisely the symptom the list reports as "no position", and it is what QGC's own editor
drives; its wording is worth copying exactly, "Click in map to set planned Takeoff location" when
launch and takeoff share a location.

**That version is committed and unverified on hardware.** The handset dropped off adb mid-check and
would not reconnect over USB or Wi-Fi. The check is one step: select the unplaced takeoff from the
list, long press the map, and the list must stop saying "no position" — the exact assertion the
reverted version failed while its unit tests were green. Until someone runs it, the unit tests say
only which branch the long press takes.

One thing left behind on the bench: that plan's launch point is where I long pressed. It is an unsaved
simulator plan that gets rebuilt constantly, so it was not worth clearing someone else's work over,
but do not read its geometry as intentional.

### An inserted takeoff has no position, and neither does the plan's launch point

~~Nothing can be added to an empty plan.~~ **That was wrong and is withdrawn.** Insert works every
time, including immediately after New plan: the chip goes straight to "1 item (takeoff) · 0 m · 0:17".
I reported the opposite an hour earlier on the strength of a run I could not reproduce, and the
filter I sampled it with could not have shown success anyway — it matched `items \(` against a
summary that reads `1 item (takeoff)`. Two wrong findings today, both from a filter that could not
see the outcome it was looking for. That is now four instances of the same instrument failure.

**What the working insert exposes is worse than what I thought I had found.** Add a takeoff to an
empty plan with a connected, fixed vehicle and:

    list      0 Mission Start · 0 m
              1 Takeoff · no position
    summary   1 item (takeoff) · 0 m · 0:17
    map       nothing at all - no takeoff pin, no Mission Start pin

The core's `insert` already writes `launchCoordinate` in `shape()` and treats `ok: true` from the
bridge as success, so it reports the item inserted. Both items are positionless afterwards. Either
the write is not reaching `TakeoffMissionItem::setLaunchCoordinate` or it is being applied to
something that does not stick — and the bridge said ok either way, which is the same shape as the
reverted `coordinate` write: **a `set` that answers ok and changes nothing.**

Raised with the core, since `shape()` is where the claim of success is made. Not diagnosed further
here.

**One defect in today's work did come out of it** (`aircast-android fb02aa4`). The chip's list icon was
gated on `allItems.isNotEmpty()`, and QGC's plan always holds the Mission Start settings item, so an
empty plan read "Empty plan · long press to add" with an icon beside it opening a sheet containing one
row the operator never added. `worthListing` asks the question the icon is about. Found only by opening
the tab against an empty plan, which no run before today did.

### The Plan tab draws no mission, and the Fly tab draws the same plan fine

Reproduced on a fresh process on the OnePlus 6, sim connected with a fix. Build a plan on the Plan tab
— tap Takeoff, long press for a waypoint — and:

    Plan tab   "2 items (takeoff) · 248 m · 1:06", list shows all three rows with positions,
               map draws the vehicle and its track and **no pin, no route line, nothing**
    Fly tab    same moment, same build: waypoint 2 in amber, Mission Start 0 in purple,
               the dashed amber route drawn between them

So the data is present, the mission layers work, and the map component works — `VehicleMap` is the
same composable on both tabs, and on Plan its *vehicle* effect is clearly running, because the red
icon and the blue trail are drawn from it with the same non-null `style` the mission effect needs.
Only the mission render is missing, and only on the tab whose job it is.

The Plan tab differs from Fly in exactly one argument, `editable = true`, which is what runs
`attachMissionEditing` between `installMissionLayers` and `style = loadedStyle`.

**Instrumented, and the render path is healthy.** Temporary probes in `VehicleMap` logged the mission
effect's entry, whether `style` was null, the item count, and whether the source existed. On the
editable map, every cold start since has logged

    mission effect style=true items=1 editable=true
    rendering 1 source=true loaded=true

and drawn the plan. So when it works, the effect runs with the right data against a fully loaded style
holding the source.

**It has not reproduced in five cold starts since.** The failure is real — two screenshots show a plan
with items and an empty map — but it is intermittent, and the "reproduced on a fresh process" framing
above was true of the two runs it happened in and is not true now. **The claim that it is a regression
since `66bec7f` is unsupported**: I never established the failure is new, only that I had not seen it
before. Recorded as intermittent and undiagnosed rather than as a bisected regression.

**The chase cost more than the finding, and the reason is worth more than either.** Half of it went into
a `grep -E 'editable=true|threw'` over the probe output — and the line it was hunting for,
`rendering 1 source=true loaded=true`, contains neither string. The probe had been printing the answer
from the first run. That is the **fifth** filter in one day that could not see what it was looking for,
after the `strings` ASCII read, the `^\d+\.\d+ m$` altitude sample, the fixed label list, and
`items \(` against "1 item (takeoff)". Four of the five cost a wrong conclusion. Print the whole
channel first, filter second.

### "Which item is the vehicle flying to" is not in the plan, and `current` does not mean it

The obvious next thing for the item list is to mark the item the aircraft is executing — the single
most useful line in a list during a mission, and `MissionItem.current` is already parsed from
`view.missionItems`. **Checked before building, and it is the wrong field.**

`isCurrentItem` is assigned in two places with two meanings:

    MissionController::_currentMissionIndexChanged   guarded by if (_flyView)
        -> the item the vehicle is executing, from MISSION_CURRENT

    MissionController::setCurrentPlanViewSeqNum
        -> the item the operator has selected in the plan editor

The bridge's `plan.missionController` is the **plan** controller, created because a native head has no
QML view to own one, so `_flyView` is false and the first branch never runs for it. The second does —
the core's own `point_at()` calls `setCurrentPlanViewSeqNum` on every insert. So `current` on this path
is the editor's selection, and a row marked "now" from it would point at whatever was last inserted.

`view.missionItems.current` has the same origin: `currentPlanViewVIIndex`.

**And the vehicle's own answer is not reachable.** `MissionManager::currentIndex()` is a plain method
returning an int, and `Vehicle::missionManager()` is a plain getter — neither is a `Q_PROPERTY`, so the
reflection bridge cannot traverse to one or read the other. This needs a property on the Qt side before
any head can show it. Raised with the core.

No defect shipped: `MissionItem.current` is parsed and never rendered. It is an orphaned field that
would have misled the next person to reach for it, which is the tell described in
`orphaned-mechanism-sweep`. Cost: one check. Every other version of this mistake today cost a build, a
device run, and a revert.
### Selection became a real thing, in three corrections that should have been one design

`c707803` is the root fix under a family this document records. QGC keeps three insert-validity flags
and assigns them in exactly one place, inside `setCurrentPlanViewSeqNum`, which its own PlanView calls
continuously. This head never called it, so the flags held whatever the last caller left behind — which
is why they were written up as constant and why the Add Item gate built on them was reverted. The
head's selection drives it now, sending the item's **sequence number**, not its index, which is a
different number the moment a plan holds anything the map cannot draw.

Measured on the OnePlus 6 with nothing between the two runs but which row was tapped:

    middle waypoint selected, tap Land   REFUSED, "A landing goes after the takeoff and after
                                         every place the vehicle flies through." Plan unchanged.
    last waypoint selected, tap Land     inserted, "4 items (takeoff, RTL)"

**Then three commits in a row fixed the commit before them, and that is the part worth recording.**

`d760e46` made a new item land after the selected one, which is what QGC does and what the item list
made reachable. It also made selection silently decide where things go, with nothing on screen saying
so — an invisible mode. `380e31f` put "Adding after #3" in the add row. That exposed the next hole: no way to *clear* a
selection **by touching the map** — `onSelected` was only ever called with a hit. I wrote it up as "no
way at all", which was wrong: `BackHandler(enabled = selected != null)` at `PlanMapContent.kt:125` has
been clearing it the whole time, confirmed on the handset. The gap was discoverability, not absence,
and reading the file around the code I was changing would have shown it. `563a870` made a tap on empty map clear it —
and broke the first feature, because a long press also ends in a tap-shaped up, so every add cleared
the insertion point it had just used. `5e985d5` fixed that two ways: a gesture that added something no
longer deselects, and a successful insert selects the item it created, which the core's answer already
carried as `index` and the head was throwing away.

The end state is coherent and matches QGC — `insertSimpleMissionItem` is called with
`makeCurrent = true`, so selecting the new item is what the desktop does, and it is what makes a chain
work: long press twice and the plan reads "Adding after #2" then "Adding after #3".

But four commits to land one behaviour, each found by reviewing the one before, is not four good
catches. It is one design decision — *what does selection mean on this screen* — taken incrementally
instead of up front. Every consequence was discoverable by asking that question once: if selection
decides the insertion point, it must be visible, it must be clearable, and it must survive the
operation it exists to aim. The review step caught them, which is the system working; asking first
would have been cheaper than being caught three times.
### The plan goes to the vehicle and comes back the same

Not the Phase 4 gate — that wants 200+ waypoints, flown, byte-identical — but the first end-to-end
check that the whole tab works together rather than one change at a time. Sim connected, OnePlus 6:

    build     takeoff plus two waypoints, long press, "3 items (takeoff) · 785 m · 2:54"
    Upload    "Upload sent to vehicle", and the chip goes Unsaved plan -> New plan
    sim log   seq=0 cmd=16  41.7138557 44.8232994 alt 0
              seq=1 cmd=22  41.7151000 44.8271000 alt 50
              seq=2 cmd=16  41.7169766 44.8225771 alt 50
              seq=3 cmd=16  41.7191607 44.8268883 alt 50
    New plan  "Empty plan · long press to add"
    Download  "Download requested from vehicle", then "3 items (takeoff) · 785 m · 2:54"
    list      0 Mission Start 0 m, 1 Takeoff 50 m, 2 Waypoint 50 m, 3 Waypoint 50 m

Identical on both sides, and the wire content is read off the sim rather than inferred from the
screen.

**One thing the wire shows that the screen cannot.** `seq=1 cmd=22` carries a real latitude and
longitude. The takeoff has a position all along — QGC just declines to *display* one, because
`APM-MavCmdInfoCommon.json` marks `MAV_CMD_NAV_TAKEOFF` as `specifiesCoordinate: false` on ArduPilot.
So the aircraft receives a complete plan, and the earlier finding stands in its corrected form: the
takeoff is not missing a place, it has one that the plan view is not in the business of showing.

Also confirms the altitude fix against a plan the head did not build: the downloaded takeoff reads
"1 Takeoff · 50 m", not "no position".
### Two host flags in MainWindow.qml cannot be set, and the define trap next to them

Raised by the macOS session while gating the QML plan chrome, checked here because these are stream E
files.

    MainWindow.qml:119  readonly property bool hostProvidesNavigation:   false
    MainWindow.qml:120  readonly property bool hostProvidesGuidedActions: false
    MainWindow.qml:121  readonly property bool hostProvidesPlanUI: QGroundControl.corePlugin.hostProvidesPlanUI

The first two are `readonly` **and** literal, which is a compile-time constant — no host, no C++, no
runtime flag can make either true. Their three readers, `GuidedActionRTL.qml:15`,
`FlyViewWidgetLayer.qml:320` and `PlanToolBarIndicators.qml:174`, have been evaluating `!false` since
they were written. Line 121, added for the plan gate, is the working shape of the same idea: `readonly`
bound to a `Q_PROPERTY` is readonly in QML while still reflecting what the host set. Wiring the other
two that way would take the QML RTL button and the guided-action layer out of a native build. Left to
the macOS session, which has the rig to measure it; the Android head loads none of those components and
never loads `MainWindow.qml` at all.

**The trap they hit first is worth more than the finding.** The gate was initially `#ifdef
QGC_NATIVE_UI` inside `QGCCorePlugin.cc` — and that define is PRIVATE to the app target while
`QGCCorePlugin` lives in a library, so it would have compiled to false, left the Loader active, and
gated nothing. A null result byte-identical to a healthy one. Same shape as the day's other two: a
bridge `set` that answered `ok` and changed nothing, and a takeoff that read back with no coordinate
because the metadata says it has none. Three ways to get an answer that looks like success and is not,
in one day, in three different layers.
### The plan list spelled every altitude in metres, and the core already knew better

`3cbec30`. The list and the selection line formatted the altitude themselves —
`"${it.roundToInt()} m"` — so an operator working in feet read a metric number with an "m" after it.
Wrong number, wrong unit, nothing on screen to reveal it. The core has served `altitudeText` since the
view shipped, in the **vertical** unit, which QGC keeps separate from the horizontal one.

Verified by switching Settings → Units → Vertical Distance to Feet: `1 Takeoff · 50.0 m` becomes
`1 Takeoff · 164 ft`, and back.

**Found sideways.** The core session flagged that the item list watched neither unit setting, so rows
would keep a stale spelling until something unrelated refreshed them. That was half of it; the other
half was that my rows were never going to change spelling whatever the watch did. The tests that broke
were pinning my own formatting, which is what was being removed.

`3818bed` then narrowed "no position" with `specifiesCoordinate`, which the core added on request. A
Change Speed or a camera trigger has no place by design, and the list was reporting that as a gap —
the same wrong note the takeoff got before `7416fa2`, from the same cause: the head could see an
absence and not what the absence meant. It is not verified on hardware, because this head cannot add
an item that specifies neither a coordinate nor an altitude; the toolbar offers only kinds that
specify one or the other. A plan file with a `DO_` item is the check it waits for.

**The rule both of these came from is the one worth keeping**: do not re-derive in the head what QGC
states in metadata. Formatting a unit, deciding whether a command has a position, matching a command
by name — each was the head answering a question the core already answers, and each was wrong in a way
that looked right.

**A sweep for unit literals found two more of the same** (`66dea85`, `740e3c0`). The terrain band built
`0-50 m AMSL · 1.15 km` from raw metres while the core served `lowestText`, `highestText` and
`distanceText` for that view — easy to miss because the parser already used `clearanceText`, so it
looked like it honoured the core's spelling. And the plan summary spelled a fence circle's radius
itself while the core served `detailText`. `grep -rn '" m"\|" km"\|" ft"'` over the head's main sources
is what found both; reading the screens did not, because every one of them was right in the units I was
testing in. Four instances in one evening, one root.

**One place the sweep found that the core cannot fix yet.** `ObstacleDistance.kt` formats
`"%.1f m %s"` on the Fly view, so a proximity reading is in metres whatever the operator chose. It is
not re-derivation: there is no obstacle view in the core, the head computes the nearest distance and
bearing from the raw sensor array itself. Fixing it properly means the core serving it — both heads
would want it and neither should convert units on its own — which is a request rather than a patch.
Recorded, not raised, because it is a new view rather than a field.

The tell worth keeping: **the tests that broke were pinning the head's own formatting.** A test that
asserts a string the head built is a test that the head is answering a question it should be asking.

**`e395c3c`'s commit message contains a claim that is wrong twice over, and the way it went wrong is
the useful part.** It reports that the Android AAR "produces a core older than its own sources",
because `strings` on the built library could not find `bandText` while finding its neighbours.

Round one: the core session was editing `core-rs/src/terrain.rs` while I built from it, so every build
raced an edit. I withdrew — correctly for those runs, and wrongly as a general conclusion, because one
mechanism explaining some of the evidence does not account for the rest.

Round two: I held the source still for twelve minutes, rebuilt, confirmed cargo printed
`Compiling qgc-core`, and added a control — `lowestText` and `highestText` from **lines 172 and 173 of
the same `json!`** were in the archive, and line 174 was not. Re-raised.

Round three, from the core session, settles it: a marker inserted on the line *immediately above*
`bandText` survives into the archive, `bandText` does not, and `cargo test --release` calling the view
emits the key regardless. **`strings` on a release static archive gives false negatives.** `nm -g` is
no better — release inlining leaves no `terrain` symbols there at all. A forced `touch`-and-rebuild,
which I ran, produces the same absence and distinguishes nothing.

So on a release build a **negative** byte probe means "learn nothing", not "stale"; a positive one is
still trustworthy. Confirmed by running it: the band now reads
`-10.0 m to 60.0 m AMSL · 1.08 km`, which is `bandText` with matched precision, out of a library whose
bytes `strings` says does not contain it. The core asserts that key by calling the view now
(`20bad6364`) rather than looking for it in bytes.

The composed-pair fallback stays: it cost nothing and it is what the handset drew for two hours.

Three fields arrived unused in the same commits and are worth a look before the row grows further:
`azimuthText`, `distanceText`, `altitudeChangeText`, all per item and null when the controller has not
worked the value out. A row reading `2 · Waypoint · 50.0 m · 449 m · 47°` tells an operator about a
leg; it also crowds a row that gained two things today.
### A guard that failed open for six hours

`ui.sh tap` refuses a tap that lands on Arm, Land, RTL or any other flight control unless
`ALLOW_FLIGHT_COMMAND=1`. It had not refused anything since `a8d19fa` this morning — the
commit that taught `whatsunder.py` to read single-quoted attributes left the pattern as a
raw string ending in a quote, which is a **syntax error**. Every tap since ran

    under=$(... | python3 whatsunder.py X Y)

took the empty string from a process that died on import, and tapped anyway. The traceback
went to stderr, where it reads like noise from `adb` rather than a dead safety check.

Fixed in `f9608af`, and the second half is the one that matters: **`ui.sh` now refuses when
the guard cannot answer.** An empty dump or a non-zero exit is a refusal. A guard whose
failure mode is "allow" is worse than no guard, because it reads as checked — and this one
had been reporting every target clear while I was tapping around a connected vehicle.

The pattern was broken twice over, which is worth stating separately: with two alternatives
for the label, `findall` returned six fields where `labels_under` unpacked five, so even a
parseable version would have raised. `whatsunder_test.py` now pins both quote styles, the
plan-screen exemption, and that unreadable input exits non-zero. Verified live: tapping Land
on the Fly view is refused; before this it would have landed the aircraft.

Same family as everything else found today — [[uiautomator-quotes-and-flat-reads]], the two
translated-name defects above — a reader that cannot see the thing, and absence reading as
fact. This one is the worst of them, because the thing it could not see was a safety
interlock and the silence was indistinguishable from a clear screen.

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

**The takeoff-ordering fix survives the round trip** (verified 2026-09-10 on the
OnePlus 6, driving the SAF dialog by attribute rather than coordinate). Saved a
three-item plan, cleared it with New plan, reopened it from Download: the file on
disk carries `commands: [22, 16, 16]` — takeoff first — and the reopened plan
reports the same three items and distance. So the ordering fix is in the written
artefact, not only in the live model.

Two things the walk turned up:

- **Both confirmations fired on an empty plan** — "Starting a new plan clears
  the one you have" with nothing in it, right after a save. Fixed in `e9c5bb5`.
  The gate was `plan.dirty`, which looked correct; the cause is in shared QGC.
  `PlanMasterController` clears the flag in exactly two places, `saveToFile` and
  `removeAll`, and **both clears are behind `offline()`**. With a vehicle
  connected neither saving nor starting a new plan clears it, so `dirty` there
  means "not synced to the vehicle" rather than "has unsaved edits". The head now
  also requires `containsItems`, which drops the certainly-wrong case without
  redefining a flag the desktop and macOS heads share. Clear mission still always
  confirms: it acts on the aircraft, which can hold a mission when this plan is
  empty. A third consumer had the same hole and was found by reading every
  reader of `dirty` rather than only the one that reported the symptom:
  `loadStep` in map-spike gated the vehicle-download button, so Download
  relabelled to "Discard & download" and wanted a second tap on an empty plan
  (`dd1ce3f`). `planStatusText` was already correct — it branches on `offline`
  to say "unsaved changes" against "not uploaded".

  Verified on the handset with the sim connected, in both directions, because a
  gate that never fires would pass the same check as a gate that fires
  correctly: empty plan takes New plan and Open with no dialog and Download
  without arming; one item restores the dialog and arms Download to
  "Discard & download". The status chip reading "Unsaved plan" on the empty plan
  confirms `dirty` really was set for all of it — the fix is `containsItems`
  doing the work, not the flag having quietly gone false.
- **The picker does not land on the file you just wrote.** DocumentsUI opens on
  its own last location and sorts by name, so a freshly saved plan is wherever
  the alphabet puts it. Nothing to fix in our code, but it means "save then
  reopen" is not the two taps it looks like.

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
| `RcControlsLayer` | **built** | renders the configured controls and sends `setRcChannelOverride`; the list is now editable on the phone |
| `ObstacleDistanceOverlay` (map and video) | **built, in another form** | a sentence — "3.2 m right" — rather than a proximity ring |
| `FlyViewToolStrip` + action list | **built, as an Actions sheet** | the checklist plus pause, gripper, emergency stop, mission start/continue, land abort; Viewer3D dropped |
| `GuidedValueSlider` | **built** | altitude, speed, takeoff height, and pause all read their range from the core |
| `DetectionOverlayVideo` | **built and driven** | boxes verified on real video; they normalise to the surface, not the picture, which is wrong when the source is letterboxed |
| `FlyViewCustomLayer` | **dropped** | a placeholder for downstream forks to override; nothing to port |
| `OverlayGlass` / `OverlayRig` / `FlyViewInsetViewer` | **partly replaced** | Compose does the layout; what remains of the rig's job is the camera inset, and the bottom one is fed from the measured controls panel |
| `Viewer3D` | **dropped** | decided in Phase 2; goes with `FlyView.qml` |

That inventory is now closed. Every layer above is built, built in another form, or dropped
with a reason, and the Fly tab draws the native map with `AndroidHost.qml`'s `FlyView` and
`PlanView` told not to render at all — so nothing of QGC's QML is visible on this head on any
tab. What is left of `FlyView.qml` for Android is the `QQuickWindow` that `VideoManager`
initialises against, and Phase 6's deletion of the host.

The paragraph that used to stand here said the tab could not be switched because eight
surfaces had no native equivalent. That was true when it was written and is kept in the
history rather than the present.

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
sentence following the slider to 3.0 m/s. Its confirm went unobserved for a long time: taps at
what I had measured as the centre of the Set button left the dialog open. It was a harness
problem, as suspected. **It is now observed on the wire**: with the dialog reading "The aircraft
will fly at 3.6 m/s.", tapping Set at (809, 1326) sent `MAV_CMD_DO_CHANGE_SPEED` (178) with
speed type 1 and 3.56 m/s. The rule that closed it is the same one that keeps biting: measure
the button on the frame you are about to tap, not on an earlier one.

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

### The checklist moves to the Fly tab

It had been parked under Analyze because the Fly tab was still QML and there was nowhere
native to put it. There is now: `FlyViewToolStrip` carries `PreFlightCheckListShowAction`, and
the Actions sheet is that strip, so the checklist is its first entry and the Analyze page is
gone rather than duplicated.

**The Actions button now always shows.** It used to appear only when the core offered
something extra, which meant it was hidden in exactly the state a checklist is for — on the
ground, disarmed, nothing to pause or abort. A button that disappears when its most important
item is most needed is not a button.

Its subtitle is the checklist's own summary, so the sheet says what is wrong before you open
anything: on the sim it read "1 of 11 will stop the flight." with the battery at 25%. A
blocker outranks progress in that sentence by the core's rule, which is right — the count of
things you have ticked is not the headline when one of them will stop the flight.

**The ticks had to be hoisted out of the screen.** `PreflightScreen` owned its own ticked set,
which is fine for a page you navigate to and stay on, and wrong for a dialog: closing it to
look at something and reopening would have silently wiped every manual check. The set now
lives on the Fly screen and the screen takes it as a parameter. Verified on the handset — a
check ticked, dialog closed, sheet reopened, dialog reopened, still ticked.

### The sequential counter comparison passes

Re-run on `e2ce7026c`, which stops the shared-port guard firing on the link manager's own
configuration. Same APK, same sim, one line of ini between the runs, a 60 second window in
each:

| run | link | messages in the window | sim commands |
|---|---|---|---|
| A | Qt UDP | 3207 → 6108, so **2901** | 60 |
| B | core UDP | 2384 → 5363, so **2979** | 54 |

The few percent between them is the sim's own timing — its obstacle stream stops at 45
seconds and the window does not fall in exactly the same place twice — not the transport.

The stronger result is inside run B. `vehicle.messagesReceived`, counted by Qt's model, and
`view.coreVehicle(1).vehicle.messagesReceived`, counted by the core's own model off the same
bytes, read **2384 and 2384** at the first sample and **5363 and 5363** sixty seconds later.
Not close: equal, twice. The core link carried 104 kB in that window with 476 heartbeats seen.

`view.coreVehicle`'s fields are nested under a `vehicle` object, not at the top level — the
top level is `available`, `class`, `kind`, `vehicle`, `vehicleIds`. Reading
`messagesReceived` at the root returns nothing and looks exactly like a counter that does not
work.

### Before the fix: a core link that never opened



`settings.appSettings.coreLinks` was added so the same stream could be run once through the
Qt link and once through a core-built one, with `vehicle.messagesReceived` read both times.
The Qt half is measured. The core half does not run on this head yet.

| run | setting | messages in 60 s | commands the sim received | vehicle |
|---|---|---|---|---|
| A | default | 3224 → 6928, so **3704** | 71 | present, flying |
| B | `coreLinks=true` | 0 | 0 | "No vehicle" |

With the flag on, nothing moves in either direction. `view.transports` — read
synchronously — holds exactly one entry, and it is the autoconnect UDP configuration with
`"owner":"qt"`, `"state":"closed"`, `"reason":""`. So no core-owned link is created and the
Qt link does not open either. `view.coreVehicle` and `view.coreVehicle(1)` are both served
and both correctly report `available:false`, which is the documented behaviour off a core
link, so the view itself is fine.

The flag is definitely being read: the only change between the runs was that line in the ini,
and the app went from a connected vehicle to none.

The cause turned out to be the guard added after this head's own reuse-port finding. That
guard refuses a core UDP link on a port a Qt link already serves; `LinkManager` sets the
configuration's link to the `CoreLink` before calling connect, so at open time the core saw a
Qt link on 14550 that *was* the configuration it was opening, refused itself, and stayed
unconnected — the one closed qt-owned entry. The macOS suite opened on port 0, so the guard
never fired there. A guard written for one head's footgun fired on the other head's legitimate
path, and only a real link on a real port showed it.

**A watch is not a read when the value never changes.** The first pass at this used `qgcPath`,
which is a subscription, and every reading came back null — easy to misread as "the view is
empty" when it means "nothing has changed since you subscribed". Switching the probe to a
polled `Qgc.get` turned three nulls into `available:false` and a populated transports list,
which is what actually produced the finding. The same trap sits under any probe that watches
a path expected to be quiet.

**`strings` on macOS needs `-a` to scan a dex.** Checking whether the APK really carried a new
probe string came back empty twice before the instrument, not the artifact, turned out to be
at fault. The stale-artifact rule has a sibling: before believing an artifact is stale, check
that the thing reading it can see anything at all.

### The QML views stop drawing, and where the CPU actually goes

`AndroidHost.qml` instantiates a full `FlyView` and `PlanView` — a second ground station, with
its own map and its own tile fetches, drawing every frame underneath an opaque native one.
Now that no QML is visible on any tab, the host takes a `renderViews` property and the head
sets it false once QML reports ready. The objects still exist, so `flyView.mapControl`, the
guided controller and the tool `Loader` are untouched; they simply stop rendering. Phase 6
deletes the host outright, which needs the C++ boot path; this is the part that needs neither.

Measured on the handset, Fly tab, sim connected, five samples five seconds apart:

| | RSS | private | CPU |
|---|---|---|---|
| views drawing | 528 MB | 250 MB | 86, 107, 125, 110, 103 |
| views hidden | 473 MB | 196 MB | 103, 114, 90, 144, 93 |

**About 55 MB back, and no CPU change at all.** That second half matters more than the first:
the expectation going into Phase 6 was that deleting the QML host would be a throughput win,
and on this evidence it is a memory win only.

The CPU is somewhere else. Per thread, at the same moment:

| thread | share |
|---|---|
| `qtMainLoopThread` | 44.8% |
| Android main | 24.1% |
| Compose `RenderThread` | 20.6% |
| `QtThread` | 3.4% |

Nearly half of it is inside QGC's own event loop, which is where MAVLink parsing and the
bridge's watcher polling live — Risk 5's 200 ms diff loop, now with a number against it. Any
real throughput work on this head starts there, not with QtQuick.

### What the 200 ms watcher actually costs

Risk 5 says watched paths are polled and diffed rather than signal-connected. Timing every
`readPath` inside `Watcher::_poll` on the handset, summed over 25 polls — five seconds — with
the sim connected:

| screen | paths | poll work per 5 s |
|---|---|---|
| Fly | 70 | 490–635 ms |
| Settings | 18 | 277–290 ms |

So the poll is ten to thirteen percent of a core on the Fly tab. That is a quarter of the
44.8% the Qt thread was using, not all of it — worth knowing before anyone treats the watcher
as the whole story.

The distribution is the useful part. Three paths dominate everything:

| path | per 5 s, Fly tab |
|---|---|
| `vehicle.vehicle` | 230–242 ms |
| `vehicle.batteries` | 118–141 ms |
| `vehicle.gps` | 80–89 ms |
| everything else (67 paths) | ~160 ms |

All three are whole objects, and all three are dependencies rather than anything a screen asked
for directly: `view.instruments` declares `DEPS = [activeVehicleAvailable, vehicle.vehicle,
vehicle.gps, vehicle.batteries, vehicle.wind]`. To notice that `altitudeRelative` moved, the
watcher serialises every Fact on the Vehicle five times a second and compares the string. On
Settings, where no instrument is drawn, `vehicle.batteries` and `vehicle.gps` still cost 240 ms
per 5 s between them — legitimately, because the header strip shows battery and satellites, but
it shows two numbers and pays for two whole objects.

`view.instruments` cannot narrow its `DEPS` statically, because which facts it reads comes from
its arguments. Either the core derives a view's dependencies from its arguments, or the bridge
learns a cheaper way to tell that an object changed than rendering it to JSON. Both are core
work; this is the measurement, handed over.

**The core took the first option, and the total did not move.** On `1446b17c1`,
`view.instruments` declares the facts its arguments name. Re-timed on the handset, same
screen, same sim, disarmed both times:

| | paths | poll work per 5 s | `vehicle.vehicle` | `vehicle.batteries` | `vehicle.gps` |
|---|---|---|---|---|---|
| before | 70 | 490–635 ms | 230–242 ms | 118–141 ms | 80–89 ms |
| after | 71 | 562–615 ms | **gone** | 192–224 ms | 102–104 ms |

The change did exactly what it said — the whole-vehicle read is absent and
`vehicle.altitudeRelative`, `vehicle.distanceToHome` and `vehicle.groundSpeed` appear in its
place at 25, 19 and 13 ms — and the poll costs the same as before, because `vehicle.batteries`
grew by very nearly what `vehicle.vehicle` used to cost.

The likeliest reading is that the reads were sharing work: `vehicle.vehicle` came first in the
poll and warmed the object graph that `vehicle.batteries` and `vehicle.gps` then walked
cheaply. Remove the first reader and the second pays full price. If that is right, the
remaining lever is not removing more readers — it is making an object read cheaper, or giving
`view.battery` and `view.preflight` the same argument-derived deps `view.instruments` just got.
That is a hypothesis from two measurements, not a proof.

Process note: the first re-timing was taken with the vehicle armed and the baseline had been
disarmed. Re-running it disarmed changed nothing, but the comparison was not sound until it
was re-run.

Unwatching is healthy, incidentally: leaving the Fly tab takes the watch list from 70 paths to
18 and back again, so nothing accumulates.

### Arming said it had failed, every time it worked

Chasing the speed confirm turned up a banner reading "Arm was not confirmed by the aircraft."
on a vehicle that had plainly armed — the button already said Disarm and Land, Return and Speed
had all come alive behind it.

`attemptCommand` polls a `reached` predicate until it holds or four seconds pass. The arm
button passed `{ armedNow() == !armed }`, and `armed` there is a Compose state read, not a
captured value, so it is re-read on every poll. The instant the vehicle armed, `armed` flipped
true, `!armed` became false, and the predicate started asking whether an armed vehicle is
disarmed. It could only ever time out. The faster the vehicle armed, the more certainly the
head called it a failure.

The fix is to capture the target once, before the polling starts, which is also what the flight
mode picker already did — its predicate compares against a captured `mode.name` and has never
misreported. Verified both directions on the handset: arm and disarm, no banner either time.

A false refusal on the arm path is worse than it looks. It teaches an operator that the
message means nothing, which is exactly the wrong lesson for the one control that has to be
believed.

Audited every other place the head waits for a command to take: the flight mode picker
compares against a captured `mode.name`; `LinksScreen`'s four predicates read `currentRows()`
fresh from the bridge and compare against a captured `row`; `SensorsScreen` captures the
status from before the call and compares fresh reads against it. All three have the right
shape — fixed expectation, live observation — and the arm button was the only one with it
backwards. The tell to look for is a Compose state delegate read *inside* a predicate that
polls, rather than a value captured before the polling starts.

### Detection boxes, and the last Fly surface

`DetectionOverlayVideo.qml` parsed the RTSP URL itself, polled the agent at 10 Hz and dropped
boxes older than a second — product logic, in a head, duplicated per head. It is now
`view.detections` in the core, following the agent's SSE stream rather than polling, so the
head gets an event per frame, per error, and once when the feed goes stale, and needs no timer.
The overlay draws the boxes normalised to the painted video rect, marks the tracked one in a
second colour, captions each with its label and confidence, and shows the core's `error`
in a corner chip — but only when the feed is configured, so a vehicle with no detection agent
sees nothing rather than a complaint.

**The live half is unexercised and this is not a screen an empty result can vouch for.** The
overlay only mounts while video is decoding, and nothing on this rig produces a stream, so a
blank video area is exactly what a broken overlay would also look like. What is checked is the
decode of the core's shape, the stale and unconfigured gates, the trouble message and the
caption — five unit tests — plus a device pass confirming the Fly tab is unaffected by
mounting it.

**The endpoint's port was what made it unverifiable.** The core built
`http://<host>/api/streams/<camera>/detections/stream` with no port, so a fake feed meant
binding port 80 and that needs root here. Raised as a testability note; the core answered with
`settings.appSettings.detectionsHttpPort`, and with a port the whole thing can be driven.

### Driving the detection overlay

`scratchpad/detrig.sh up` stands the rig up and `down` puts the device back. Both halves go
through `adb reverse`, so nothing depends on which subnet the handset is on:

- `detfeed.py` serves the agent's SSE shape on 8099 with two boxes, one of them the target and
  one of them moving.
- `gst-launch` sends a 640×480 test pattern with a burnt-in clock over TCP on 8100.
- the device ini points `rtspUrl` at `rtsp://127.0.0.1:8554/front` for the host and camera name,
  `tcpUrl` at `127.0.0.1:8100`, and `[General] detectionsHttpPort=8099`.

**It works.** The core followed the stream, `view.detections` populated, and the overlay drew
"car 91%" in the target colour and "person 47%" beside it, moving with the feed, over live
decoding video — in the inset and again full screen.

Two things the rig taught that reading could not.

**`tcpUrl` must not carry its scheme.** With `tcpUrl=tcp://127.0.0.1:8100` GStreamer tried to
resolve a host called `tcp`, because QGC prepends the scheme itself: `Failed to resolve host
'tcp'`, from a URL logged as `tcp://tcp://127.0.0.1:8100`. The setting takes `host:port`.

**The boxes normalise to the surface, not to the picture.** Full screen, a 4:3 source in a
portrait surface is letterboxed: the picture occupies 1080×810 of a 1080×1770 surface, starting
810 px down. The overlay spreads the same 0..1 over the whole surface, so a box at y=0.30 lands
190 px high and is drawn 2.2× too tall — plainly visible as boxes running off the picture into
the controls panel. The burnt-in clock, which GStreamer draws at the frame's top-left, is what
pins the picture's real top edge. In the inset the source nearly filled the surface, which is
why it looked right there. The head could not fix this alone: it needed the source dimensions,
and `view.video` served neither. The core added `sourceSize` — `{width, height}` of the decoded
frame, null until a frame is decoding — and the overlay now letterboxes against it, computing
the painted rect the way the sink does and placing every box inside it.

**Checked against the clock rather than by eye.** Full screen, the content area is 540×825 dp
and a 4:3 source paints 540×405 of it starting 210 dp down. The person box is served at a fixed
x=0.62, w=0.12, y=0.55, so it must land at x 335–400 and y 618. Measured on the screenshot:
x 335–400, y 617. The moving box tracks inside the picture too, and neither runs into the
controls panel any more.

### The shared index had a night's work staged for deletion

Sessions in this tree share one git index, which is why every commit here goes through a
private `GIT_INDEX_FILE`. That protects the commit being made; it does not protect the shared
index from going stale. Checked it tonight and found nine files staged as deleted —
`Detections.kt`, `DetectionOverlay.kt`, `MoreActions.kt`, `VideoSourceLayer.kt`,
`VideoView.kt`, three of their tests, and `map-spike/FlyMap.kt` — every one present on disk and
every one already in `HEAD`. A commit from any session using that index would have removed
about a thousand lines of landed work, silently, as a side effect of committing something else.

The fix is surgical rather than a blanket `git reset`, so nobody else's staged work is
disturbed: list the paths staged as deleted that still exist on disk, and reset only those.

```
unset GIT_INDEX_FILE   # or the check reads the private index, not the shared one
git diff --cached --name-status HEAD | awk '$1=="D"{print $2}' | while read f; do [ -f "$f" ] && echo "$f"; done
git reset -q HEAD -- <those paths>
```

The `unset` matters more than it looks. Run the check in a shell that still exports
`GIT_INDEX_FILE` from a commit — pointing at a private index file that has since been deleted —
and git reports every file in the repository as staged for deletion. That happened here, and
the false alarm is indistinguishable from the real one until you unset the variable and look
again.

A path staged as deleted while it is still on disk is the tell, and nothing looked wrong
before the check — the working tree was correct, the commits were correct, and only the index
disagreed.

**The cause is the private index itself, not another session.** A commit built in a private
`GIT_INDEX_FILE` never tells the shared index about the files it added, so every new file
lands in the shared index as a deletion the moment the commit exists. Watched it happen twice
more: committing the RC controls editor left its two new files staged as deleted, and
committing the extra-cameras editor left its three. Nobody did anything wrong; it is what
`read-tree HEAD` into a private index means.

So the reset is not an emergency measure, it is the second half of committing:

```
unset GIT_INDEX_FILE
git reset -q HEAD -- <the paths the commit added>
```

Leave it out and the deletions accumulate until someone commits through the shared index and
takes them all with them, which is exactly the state the first check found: nine files, from
five earlier commits, all staged for deletion at once.

### The watcher after signal binding: half the cost, and why only a fifth of the paths

`c5fd6e918` binds a watched path that resolves to a `Fact` to its `rawValueChanged` and stops
polling it; `541339df6` gives `view.battery` and `view.preflight` fact-level dependencies.
Timed on the Fly tab, same rig, 25 polls a sample:

| | watched | polled | poll work per 5 s |
|---|---|---|---|
| whole-object deps | 70 | 70 | 490–635 ms |
| argument-derived deps | 71 | 71 | 562–615 ms |
| signal-bound facts | 97 | **79** | **265–300 ms** |

`vehicle.batteries` is gone from the poll entirely, and the total is roughly halved even though
there are more paths watched than before.

**Only 18 of the 97 bind.** The remaining 79 are re-resolved and re-read every tick, and the
reason is in `_bind`: it takes a path only if `qobject_cast<Fact *>` succeeds. Most of what a
Fly tab watches is not a `Fact` — `vehicle.latitude`, `vehicle.coordinate`,
`vehicle.flightModes`, `plan.missionController.containsItems` are plain `Q_PROPERTY`s with their
own NOTIFY signals, and `vehicle.gps` and `vehicle.cameraManager.currentCameraInstance` are
objects. What is left is led by `vehicle.gps` at 90–99 ms, `vehicle.coordinate` at 28–33 ms and
`currentCameraInstance` at 17–20 ms.

So the lever worked and reached a fifth of the list. The obvious extension is to bind any
property that declares a NOTIFY signal — `QMetaProperty::notifySignal()` — rather than only
Facts, which would take `vehicle.latitude` and its neighbours out of the poll too. Objects
would still need polling, and `vehicle.gps` would then be almost all of what remains.

Process CPU did not move: 100–111% here against 85–125% before. Thread shares from a single
sample are too noisy to read anything into, and the poll was never the whole of it.

### The poll, end to end: 570 ms to 55 ms

Four changes, three of them the core session's and one this head's, measured the same way each
time — every `readPath` inside `Watcher::_poll` timed and summed over 25 polls on the Fly tab
with the sim connected.

| | watched | still polled | poll work per 5 s |
|---|---|---|---|
| whole-object deps | 70 | 70 | 490–635 ms |
| `view.instruments` deps from its arguments | 71 | 71 | 562–615 ms |
| facts bound to `rawValueChanged` | 97 | 79 | 265–300 ms |
| properties bound to their NOTIFY | 98 | 22 unbound, 58 re-read | 223–238 ms |
| the status strip watching two facts | 98 | 21 unbound, 58 re-read | 83–127 ms |
| camera per-fact deps, pack deps from the count | 93 | 2 unbound, 72 re-read | **46–72 ms** |

The last row is the head's own doing and it was the largest single step. `StatusStrip` read
`vehicle.gps` — the whole `FactGroup`, twenty facts each rendering fourteen properties — to
show two numbers, satellites and HDOP. Watching `vehicle.gps.count` and `vehicle.gps.hdop`
instead binds both to their Facts' change signals and takes them out of the poll entirely. The
strip shows exactly what it showed before, "11" and "1.2", checked on the handset.

That is the same mistake the core made with `view.instruments`, made independently in the head,
and it is worth stating as a rule rather than an anecdote: **watch the fact you display, not
the group that contains it.** A group read is convenient and costs the whole group every tick.

Both of those went the same way. `view.camera` now depends on the camera's own properties
rather than the object, and `view.battery` declares one pack's facts until a render has seen
the vehicle's pack count and then as many as it reports, with the router re-watching after a
recompute whose dependencies changed. The whole-object read and the phantom packs are gone:
only `currentCameraInstance.recordTimeStr` remains, at about a millisecond.

**Nothing dominates any more.** Of 93 watched paths, 2 are unbound and 72 are properties
re-read every fifth tick; the largest single line is `vehicle.guidedModeSupported` at 5–8 ms
and the rest are one or two. That is the shape you want at the end of an optimisation — not a
smaller peak, but no peak.

Process CPU stayed where it was all night, 114–140% against 85–125% at the start. The poll fell
by an order of magnitude and the process did not get cheaper, which is worth stating plainly:
whatever the Fly tab costs on this handset, the watcher was never the bulk of it. Where it does
go is measured in the section below, and the guess offered here first — two map renderers and a
video pipeline — was wrong about the emphasis.

### The last two desktop-only settings

`rcControls` is a JSON array in a settings fact, and the head could render the controls but not
change them — Settings carried a footnote telling the operator to go and find a desktop. For a
product whose ground station is the phone, that is a hole rather than a gap. There is now an
editor under Settings, beside Comm Links and Units, which are the other two settings pages that
are screens rather than fact rows.

It adds, renames, re-channels, re-types and removes, and it warns when a channel is already
driving something — another control, or one of the five camera channels the Fly view settings
reserve for gimbal tilt and pan, zoom, light and record. A new control lands on the first
channel nothing else is using. Save is refused, not corrected, while a channel is out of range
or taken.

**It edits the entries rather than rebuilding them.** The desktop editor writes a fourth field,
`orientation`, that this head neither shows nor understands; rebuilding each entry from the
head's own model would silently drop it the first time anyone touched a control on a phone. The
edit functions patch the JSON objects in place and a test pins that `orientation` survives a
rename, a re-channel and a neighbour's removal. An editor should not quietly discard what it
cannot read.

Verified on the handset end to end: added a control, watched it appear on the Fly tab as a live
slider, and removed it again.

**Four chips did not fit on one row.** The type picker was a `Row`, so "Momentary" — the fourth
of four — was off the edge of the dialog and unreachable. It is a `FlowRow` now and wraps to two
lines. The control that cannot be chosen is the one nobody reports.

`extraVideoSources` was the other one, and it is the setting this head needed to hand-edit an
ini for earlier the same night to test the video source picker. It has an editor now, under
Settings beside Video, on the same pattern: entries patched rather than rebuilt, so fields this
head does not know survive; the stream kinds read from `videoSource`'s own enum rather than a
list copied into the head; the address field appearing only for the kinds that need one; and
save refused, with the reason, while a kind is unchosen or an address missing. Verified end to
end — added a camera named Thermal on the phone, saw it appear in the Fly tab's source picker
beside Camera 1, removed it again.

**It refuses an address that carries its own scheme**, because `tcp://127.0.0.1:8100` is what
cost this session a rig cycle: QGroundControl prepends the scheme, GStreamer gets
`tcp://tcp://...` and tries to resolve a host called `tcp`. The editor now says so before the
save rather than after the stream fails.

With those two, no setting the head hides is hidden because a phone cannot edit it. The
`DESKTOP_ONLY_FACTS` map and the footnote that named them are gone rather than left empty.

### The regression still passes, and the rig now survives the session

After eighteen commits on this head — a native map behind the Fly tab, a pip swap, a row that
hides what the core hides, the checklist moved, the status strip rewired, two settings editors —
the regression was overdue. It passes: Fly, the Actions sheet, Vibration, Log Download,
Settings and back to Fly, every capture over the size floor and every one checked by eye rather
than by byte count. Vibration draws 15, 45 and 75 in their three colours; Log Download lists the
sim's two logs at 4.0 and 10.0 KB; the Actions sheet leads with the checklist.

The script needed fixing first, and the reason is worth recording: it tapped the Analyze list at
coordinates from when Preflight was its first row. Moving the checklist to the Fly tab silently
invalidated a regression script, which is the kind of breakage that reports itself as a passing
run against the wrong screen. It now drives the Actions sheet as well, and its Analyze taps are
written against the current list.

**The rig is in `aircast-android/tools/` now.** Everything that verified tonight's work — the
vehicle sim, the device lock, the regression driver, the detection rig and its SSE feed, and the
watcher probe — was written in a session scratchpad, which does not outlive the session. Six
scripts and a README naming what each one is for, so the next session measures instead of
rebuilding the instruments. Their internal paths still point at the scratchpad they were born
in; the README says so rather than pretending otherwise.

### The parameter screen, reviewed

The job is to find a parameter and change it to a value the aircraft will take, without
breaking the aircraft. Search is the first thing on the screen and the only sane path through
217 rows, the constraint line carries the bounds, and the write is verified end to end — typing
2000 into RTL_ALT and tapping Set put `PARAM_SET RTL_ALT = 2000` on the wire. Two defects, both
in what the screen says rather than what it does.

**"Default null".** `FLTMODE_CH` claimed a default of `null`, in those words. `optString` on a
JSON null returns the four-character string "null", which is not blank, so it survived every
`isNotBlank()` guard on the way to the screen. This is the same trap already recorded here for
`armingBlocker`, which is why the fix went into `Qgc.fact()` rather than the display: every
string a fact carries — description, units, value, min, max, default — now reads empty when the
bridge sends null. Fixing it once at the parser is the only version of this fix that stays
fixed.

**A parameter did not say what it was measured in.** The row read `RTL_ALT · RTL Altitude ·
1500`, and RTL_ALT is in centimetres. The subtitle was `description.ifBlank { units }`, so any
parameter with a description — which is all the documented ones — hid its units behind it. An
operator setting 20 for twenty metres would have got twenty centimetres. It reads "RTL Altitude
· Centimeters" now.

**Enter does not commit, and that is left alone.** Typing a value and pressing the keyboard's
done key does nothing; the "Set" affordance that appears inside the field when the value differs
is the commit. That is mildly surprising and deliberately kept: on a screen where a keystroke
can change how an aircraft behaves, an explicit tap is worth the friction, and QGroundControl's
own desktop parameter editor asks for one too.

### The setup screen, reviewed

The job is to get an airframe configured and to know what is still missing. This screen does
the second half unusually well: it opens with "Not ready to fly", names the airframe and
firmware, counts what is outstanding, and lists the blocking items above the full list. That is
the error-summary-then-detail shape, and the duplication of Radio in both sections is the
pattern working rather than a mistake.

The Radio page underneath is a monitor: the four attitude controls with their mapping, a live
eight-channel PWM display, and a footnote saying calibration stays on the desktop because it
needs you holding each stick at its extremes while watching the aircraft. On the sim the
mapping reads "Not mapped" four times and the channels move, which is exactly what an operator
needs to see to know their transmitter is reaching the vehicle.

**One finding, reported rather than fixed.** The list badges Radio "Needs setup" — it is the
one item blocking flight — and tapping it opens a page that cannot resolve it. The page says so
plainly once you are there, and the list already has an "On desktop" badge for components with
no native page at all. What is missing is the third state: a native page that monitors but
cannot complete. Adding it means a new flag in the setup page table, and that table and the
screen around it belong to another session's work in progress, so this is a note rather than a
patch. The cost today is one tap and a corrected expectation, on a page worth opening anyway.

### The console and the inspector, reviewed: nothing found

Both were built earlier and neither had been looked at since. Reviewed on the handset against
the sim; no issues.

The console does the honest thing with a vehicle it cannot serve. Against ArduPilot it opens
with "This vehicle does not report PX4 firmware. The shell answers on PX4; other autopilots may
not reply to anything you send", then "No output yet. Send a command, for example help." It
warns before the operator spends time on it, names a first command, and still lets them try —
rather than hiding the screen or letting them type into silence.

The inspector lists what is arriving with its rate, and the per-component disambiguation added
after the duplicate-key crash is visible and doing its job: `CAMERA_CAPTURE_STATUS (comp 100)`
and `(comp 101)` sit as separate rows, which is what the sim's two camera components should
produce. Opening one gives name, type and live value per field in monospace.

Recording a review that found nothing is worth as much as one that found something. The
alternative is a habit of manufacturing findings to justify the look.

### Where the CPU actually goes

Measured with a vehicle connected, six samples on the Fly tab and four on Settings, after
confirming on a screenshot that the Fly tab was fully drawn rather than trusting the numbers:

| screen | CPU |
|---|---|
| Fly | 76–144% |
| Settings | 41–86% |

**Half the Fly tab's cost is present on a settings list that draws almost nothing.** That is not
the map, the overlays or the video surface; it is what a connected vehicle costs — MAVLink
parsing, Qt's `Vehicle` and its fact groups, and the bridge emitting events to the head. The
watcher's poll, at 46–72 ms per five seconds, is about one percent of a core and is not in this
picture at all.

**Shrinking the covered QuickView makes it worse, not better.** The obvious saving — the QML
scene is invisible under an opaque native map, so give it a 1 dp box — was tested and refuted:
Settings went from 41–86% to 90–129%, and reverting restored it. An A/B where B looks worse
deserves the A re-measured before it is believed, and it held. Covering the view is cheaper than
shrinking it, so Phase 6's *deletion* of the host is the only version of that idea that helps;
there is no cheap approximation of it.

Two instrument notes from the same session, both of which cost a measurement before they were
caught:

**The handset's address moved.** DHCP gave it a new lease during an idle stretch, the sim went
on sending MAVLink to the old one, and the app showed "No vehicle" — indistinguishable from a
broken app until you look. `tools/handset-ip.sh` asks `adb` now and `regress.sh` refuses to
start without an answer.

**`git diff` lies in this tree.** It compares against the shared index, which private-index
commits never update, so it reported 37 phantom uncommitted lines in a file that matched `HEAD`
exactly. `git diff HEAD` is the one to trust here, for the same reason the staged-deletion check
needs `GIT_INDEX_FILE` unset.

### Vehicle messages stop being parsed twice

The head was splitting QGC's `formattedMessages` HTML itself — its own tag stripper, its own
entity table, its own reading of the `<#E>` and `<#I>` style tokens. The core serves
`view.messages`, so that parser is deleted. The head gained timestamps and a severity word it
never had, and the log now reads "02:11:25.196  EKF variance" in the error colour instead of
just the text.

It was already immune to the bug the Mac head hit — the level came from the style token, not
the translated severity word — but immunity by coincidence in duplicated code is not the same
as not duplicating it.

**And then the device caught what the unit tests could not.** The banner read "12 messages from
the vehicle" while the log underneath said "The vehicle has not said anything yet". The core
serves the array as `items` inside a `VehicleMessages` class; I had guessed `messages` inside
`Messages`, and **the test passed because I wrote the fixture from the same guess.** A decoder
and its test derived from one assumption agree with each other and with nothing else.

The recorded contract in `test/Bridge/fixtures/view-shapes.json` had the answer the whole time:
`view.messages` is `{class, count, items[], kind}`. There is now a test that reads the recorded
shape and asserts the invented one produces nothing, so the same mistake fails loudly next
time.

**The sim was talking to nobody.** Its four STATUSTEXTs went out at eight seconds, and a ground
station takes about thirty to bring its link up, so the first check of the banner showed an
empty one and looked like a bug in the code I had just changed. It now speaks at forty-five
seconds and repeats every minute, with a comment saying why.

**The banner now counts what the log can show.** It had been reading `vehicle.messageCount`,
`vehicle.messageTypeError` and `vehicle.messageTypeWarning` — three more watched paths, and
three more chances to disagree with the list underneath. All three come off the same
`view.messages` the log reads, so the count and the colour cannot drift from the rows any more.
On the handset the banner says "4 messages from the vehicle" and the log shows exactly four.

Auditing the rest of the head for the same shape: what is left of the raw paths it reads are
invocations (`guidedModeLand`, `sendGripperAction`, `setRcChannelOverride`), single scalars with
no view behind them (`armed`, `flying`, `flightMode`, `latitude`, `rcRSSI`,
`communicationLost`), and `vehicle.objectAvoidance.distances` and `.msSinceUpdate`, which are
the two facts the obstacle readout draws and not a group read. No remaining place where this
head computes an answer the core already serves.

### A style fix reverted for want of a verification

Reviewing the night's own code turned up one thing worth changing: both settings editors build
their JSON with `JSONArray().also { array -> entries.forEach { array.put(it) } }`, a statement
mutation where `JSONArray(entries)` is an expression. The change was made, the unit tests
passed, and then it was reverted.

The unit tests run against `org.json:json:20240303` — the reference implementation — and the
handset runs Android's `libcore` version. They agree on `JSONArray(Collection)` as far as the
documentation goes, but "as far as the documentation goes" is the phrase that has been wrong
most often this session. Verifying it meant driving the editor on the device, and two attempts
at that put taps into the user's own applications because the app had not come forward yet.

So: a cosmetic improvement, on the path that writes settings, with no verification available
at proportionate cost. The previous form was correct and had been exercised on the handset. It
stays.

The rest of the review found nothing to change. No comments in any of the new files, every
`forEach` is Compose emitting a list, every `var` is Compose state. The two editors are
near-identical in shape, which is two occurrences and therefore not yet a shared abstraction.

**And a rule for the rig, learned twice in one session:** confirm `topResumedActivity` is the
app before sending input, not after. `am start` returns before the window is up, and a tap sent
into that gap lands in whatever the user had open.

A rule nobody can forget beats a rule written down, so it is `tools/ui.sh` now. `front` brings
the app up and waits for it, with a timeout and a non-zero exit rather than a guess; `tap`,
`swipe`, `text` and `key` each check `topResumedActivity` and refuse if it is not the app;
`shot` fails a capture too small to be a live screen. Verified both ways — with the app stopped
a tap is refused and exits 1, and with it in front the same tap lands.

`regress.sh` goes through it now, so every tap in the regression is guarded, and the whole run
is green through the new path: Fly, the Actions sheet, Vibration, Log Download, Settings and
back, with the sim confirming its statustexts, the log list request and both camera
components.

### The whole flight, once, in order

Every piece of the Fly tab had been checked on its own; the happy path had never been walked
end to end. Driven through the guarded input helper, with each dialog's sentence checked
against what the vehicle actually received:

| step | the screen said | the vehicle received |
|---|---|---|
| Takeoff | "The aircraft will take off and climb to 48.6 m." | `MODE -> 4` (Guided), `ARM armed`, `TAKEOFF alt=48.6` |
| Change altitude | "The aircraft will climb 47.5 m to 72.5 m." | `SET_POSITION_TARGET` frame 7, `z=-47.47` |
| Return | "The aircraft will fly back to its launch point and land." | `MODE -> 6` (RTL), header reads "RTL · Armed" |

Three things worth naming. The takeoff sequence is the full guided one — mode, then arm, then
the climb — not just a takeoff command, and the altitude in it matches the sentence to the
decimal. The altitude change carries its sign correctly: frame 7 is `LOCAL_OFFSET_NED` where up
is negative, so a promised 47.5 m climb is `z=-47.47` on the wire. And the row showed exactly
the offered actions at each phase without being asked twice: Arm, Takeoff, Actions on the
ground; Land, RTL, Speed, Alt, Actions in flight, with no Disarm anywhere in the air.

The altitude dialog also refused itself correctly. Opened in level flight it read "The aircraft
is already at 25.0 m and will not move" with Change greyed out — the core's `sends` gate, which
is exactly the gate that had to be relaxed for pause and is right to keep here.

One small finding from that walk, now fixed: the takeoff slider opened pinned at the far left,
because the default is the vehicle's minimum takeoff height, and there were no end labels. The
sentence carried the number so nothing was ambiguous, but the control gave no sense of its
range until you dragged it. All three value dialogs now print their ends under the slider from
the range the core already serves — "3.0 to 121.9 m" for takeoff, "2.0 to 121.9 m" for altitude,
and the same for speed.

The two altitude numbers differing is the point rather than a bug: takeoff's floor is the
vehicle's minimum takeoff height and the altitude slider's is the guided-altitude setting, and
seeing both told me the label reads its own source rather than a shared constant.

### A log downloads once, and never again

The plan called log download's software half proven, so this was meant to be a confirmation.
Selecting Log 0 and tapping Download turned the row's status to "Error" and put nothing on the
wire.

The bug is upstream, in `LogDownloadController::_prepareLogDownload`. The destination is set as
`_downloadPath + filename`, and then, if that file already exists, the de-duplicating loop
rebuilds the name and calls `setFileName(filename)` — **without the path**. The name goes
relative, `open(QIODevice::WriteOnly)` fails against a working directory nothing may write to,
the entry is marked "Error", and no `LOG_REQUEST_DATA` is ever sent. So a given log downloads
the first time and fails every time after. On the desktop it does not error; it writes the file
somewhere the operator did not choose, which is worse for being quiet.

One line: the de-duplicated name keeps `_downloadPath`. Verified on the handset by downloading
a log that had already been downloaded on 8 September — 46 `LOG_REQUEST_DATA` requests in
90-byte chunks, the row moved to "Downloaded", and
`Logs/log_0_2025-9-8-01-20-00_1.bin` appeared at exactly 4096 bytes, matching the size the
vehicle advertised.

**Twice on the way there the instrument was the problem, not the code.** The sim's
`LOG_REQUEST_DATA` handler answered without printing, so "no data request on the wire" was an
assumption, not an observation — the same silent-handler trap as `MODE` and `PARAM_SET`, and the
third time this session. And QGC's own logging is filtered to `qgc.*.debug=false` on this build,
so the controller's warning about the failed file never reached logcat. The sim prints now; the
filter is worth knowing about before trusting a quiet log.
### Erasing every log, checked

The other half of that screen is a control that permanently deletes a vehicle's flight data, so
it was worth pressing on the sim where pressing it is free.

It is properly guarded. Tapping "Erase all logs from the vehicle" puts nothing on the wire; it
opens a dialog headed "Erase all logs?" reading "This permanently deletes every log on the
vehicle. It cannot be undone." Cancel sits left of the confirm, and Cancel sends nothing —
checked, not assumed. Confirming puts `LOG_ERASE` on the wire, once.

**The confirm did not look like what it does.** "Erase all" was styled exactly like "Cancel" —
same colour, same weight — on an action the dialog itself calls irreversible. Elsewhere in this
head the destructive path is coloured: emergency stop is red in the Actions sheet and red again
on its slider. It is the error colour here now too. The words carried the warning and the
button contradicted them.

### Removing something offers it back

Auditing the head for the shape the erase button had — a destructive action that does not
present as one — turned up the two settings editors. Remove had no confirmation and no undo: a
mis-tap deleted a configured control or camera outright. The desktop editor these replace keeps
the removed entry for six seconds and offers it back, and dropping that was recorded earlier as
a nice-to-have. It was not; it was the only thing standing between a stray tap and lost setup.

Both editors now show "Removed CH1." with an Undo beside it for six seconds, restoring the
exact JSON that was there before rather than rebuilding the entry — so an undone removal
returns the fields this head does not understand along with the ones it does. Verified on the
handset: added a control, removed it, undid it, and CH1 came back with its channel and type.

**The undo looked broken twice before it was tested properly.** Six seconds is shorter than the
gap between two of this session's tool calls, so a screenshot in one call and a tap in the next
always missed the window. Doing the remove, the capture, the undo tap and the check inside a
single call showed it working. A timing window smaller than the harness's own latency has to be
exercised in one breath or it will read as a defect every time.

**And the capture guard was crying wolf.** `ui.sh shot` failed anything under 100 000 bytes on
the theory that a sleeping screen photographs small. A sparse dark list photographs small too,
and this screen — a header and one row — came in at 89 kB and was declared asleep. The test is
now what it should always have been: a sleeping screen is one flat colour, so the check asks
whether the luminance range spans more than 32. The screen that failed passes at 0–235, and a
genuinely blank frame fails at 0–0.

### What the operator sees when the link dies

Flown into guided flight on the sim, then the sim killed. About fifteen seconds later the
header changes from "Guided · Armed" to "Communication lost" in the error colour. That part
works.

**The telemetry did not change at all.** It kept reading 25.0 m, 8.1 m/s, 84 deg — the last
frame that arrived — with nothing to say those numbers were minutes old. That is the oldest
hazard in a ground station: an instrument that freezes looks exactly like an instrument
reporting a steady value. The row is now dimmed and carries a line above it, "No contact -
these are the last values the vehicle sent." The numbers stay, because the last known altitude
is worth having; what changes is that they no longer claim to be current.

**The guided buttons stay enabled, and that is left alone deliberately.** Land, Return, Speed
and Altitude remain live with the link down, because the core's offers are computed from the
last known state and do not consider `communicationLost`. Before changing that I checked
QGroundControl: `_guidedActionsEnabled` is `_activeVehicle` — optionally gated on RC RSSI, never
on link state — so this head matches upstream exactly. Blocking them would be a product
decision, not a bug fix, and there is a real argument on the other side: during a dropout an
operator may want to keep pressing Return in the hope one command gets through. Raised with the
core session as a question rather than settled here.

**And it recovers.** Killing the sim, waiting for "Communication lost", then starting it again:
twenty seconds later the header reads "Stabilize · Disarmed", the stale banner is gone, the
telemetry is back at full brightness and the heading is moving. Worth checking rather than
assuming, because a marker that latches on is worse than one that never appears — a ground
station stuck reading "No contact" over a healthy link would teach an operator to ignore the
warning entirely.
### "Connected" was a word the links screen could not back up

The Comm Links row is the screen an operator reaches when nothing else has worked, and its job is
to answer one question: is my vehicle talking to this tablet? It answered a different one. A UDP
link binds a port; binding cannot fail unless the port is taken, so `Connected` was true the
instant the socket opened and said nothing about whether anything was on the other end. Measured
rather than argued: a link added on port 14999 with nothing whatsoever sending to it read
**"Connected · UDP port 14999"** in bold primary — the most confident styling on the screen.

The fact that distinguishes a live link from a dead port already existed in the core.
`LinkInterface::decodedFirstMavlinkPacket()` flips on the first frame that parses on that channel.
It was a plain getter, invisible to the reflection bridge, and the bridge's list projection lists
child objects by name without recursing — so no amount of head-side work could reach it through
`links.linkConfigurations`.

So it went in the core, on the object the head already reads. `LinkConfiguration` holds a weak
pointer to its link and is the element in the list, so it gained
`Q_PROPERTY(bool heardVehicle)` reading through that pointer, with the notify wired from a new
`LinkInterface::decodedFirstMavlinkPacketChanged`. One field, in the JSON the head already parses,
zero extra reads per row. Both heads get it.

The row now says what is known. Not connected; **Open · nothing received yet**, in muted grey with
no emphasis; or **Receiving from the vehicle**, bold and coloured. The confident styling is spent
only on the state that earned it.

Verified as an A/B on the handset with the port held constant and only the traffic changed: port
14999 with nothing sending read "Open · nothing received yet"; the same row, same port, with the
sim retargeted to 14999 read "Receiving from the vehicle" within seconds, live, without a restart.

### The preflight list could not finish

Raised by the macOS session, which found the mirror of it in its own head: a tick placed while a
check was merely soft survived into a state where the check was failing. Mine has the opposite
error and I found it by checking rather than assuming I was clean.

`preflightSummary` compared the operator's tick count against `total`, which the core computes as
every check in every group. Only a `manual` check can ever be ticked. So on any real vehicle —
where the core measures GPS, battery and sensors itself — the denominator contains checks the
operator cannot reach, and **"All checks done" is unreachable**. The list tells a finished operator
they are not finished.

The tell was in the test, which asserted that ticking `{"a","b","c","d"}` — four names matching no
check on the list — produced "All 4 checks done." It was proving the bug.

The count is now out of the checks the operator can actually tick, ticks are intersected with the
current manual set so a stale one counts for nothing, and an accepted warning is named rather than
hidden: "All 4 checks done · 1 warning."

### The status strip could not tell a lost GPS fix from a good one

The strip sits above every tab, so whatever it says is what the operator believes without
looking further. It had a rule that appeared to guard against a lost fix:

```kotlin
lock.contains("No", ignoreCase = true) -> Color(0xFFE57373)
```

It never fired once. `qgcString` returns a fact's raw `value`, not its `valueString`, so `lock`
arrives as `"3"` — the GPS_FIX_TYPE number — and no numeric string contains the word "No". The
line reads as a safety feature and is dead code.

Measured rather than reasoned about, using the rig's existing `NOFIX=1`: with the vehicle
reporting fix type 0, **no GPS fix at all**, the strip showed "11" satellites and "1.2" HDOP in
plain white. Indistinguishable from a healthy lock. Both numbers were true and both were
reassuring, which is the worst combination — the satellite count keeps climbing while the fix is
gone, because satellites *visible* is not satellites *used*.

The colour now comes from the number the fix type actually is: below 2 is no fix, 2 is 2D, 3 and
above is a fix worth having. With no fix the cell reads **"No fix"** in red instead of a satellite
count that implies health, and the HDOP cell is hidden — a dilution-of-precision figure for a fix
that does not exist is not a smaller number, it is a meaningless one. A 2D fix says "2D only" in
amber rather than passing silently, because 2D has no usable altitude. A good fix is left
undecorated. Verified in both directions on the handset.

A test now asserts that the *rendered* text ("3D Lock", "None") yields no fix level, so wiring the
string back in hides the cell rather than silently reporting Good.

**The RC cell had the same shape of bug, from the opposite direction.** `rssi <= 0` was discarded
as unreadable. But `Vehicle::_remoteControlRSSIChanged` documents the range in a comment and
enforces it in code: 0 to 100 is a reading, 255 means unknown, and there is an explicit branch
that produces exactly 0 once the filtered signal decays. Zero is the vehicle deliberately saying
the RC link is dead — and the head threw it away, so the RC cell *vanished* at the moment it
mattered, which reads as "not applicable" rather than "lost". It now shows "No signal" in red.
The existing test had named this correctly and still asserted the wrong thing: `zero means no
signal and is not shown as a percentage`. It knew the fact and hid it.

### Tapping one corner of a fence offered only "destroy the whole fence"

Followed from a lead the macOS session sent after landing the same feature there. It transfers
exactly, which is the point of two heads over one core.

Selecting a single fence corner opened an action row containing exactly one button:
**"Delete fence"**. The granularity of the action did not match the granularity of the selection —
tap a corner to adjust it, and the only thing on offer discards the entire polygon it belongs to.
There was no way to remove just that corner. The summary line said nothing at all for a fence
vertex (`is MapHit.FenceVertex -> null`), so the title was as dead as the control, the same pairing
the macOS session reported.

`QGCMapPolygon::removeVertex` has existed in the core the whole time and QGC's own QML offers it.
The head simply never called it. It now does, and the summary reads "corner 1 of 4".

**The guard is the interesting part.** The core refuses to remove a vertex from a polygon of three
— `if (_polygonPath.length() <= 3) return;`, a silent return with no error. `FenceBridge.invoke`
reports `ok` from the bridge, which means the method was found and called, not that it did
anything. So an unguarded button would have reported success while nothing happened. The head's
`cornerRemovable` is therefore `> 3`, matching the core's refusal exactly, and the button is absent
rather than dishonest. Verified on the handset as an A/B: four corners offers "Remove corner" and
reads "corner 1 of 4"; after one removal the same polygon reads "corner 1 of 3" and offers only
"Delete fence".

### The core's own bounds check on that call never fired

Found while reading `removeVertex` to write the guard:

```cpp
if (vertexIndex < 0 && vertexIndex > _polygonPath.length() - 1) {
```

`&&` where it must be `||`. No integer is both negative and greater than the last index, so the
check never fired and an out-of-range index fell through to `_polygonModel.removeAt(vertexIndex)`.
The `<= 3` guard below limits the blast radius but does not cover it: a five-corner polygon would
take `removeVertex(99)` straight to an out-of-bounds `removeAt`.

The sibling settles the intent rather than my judgement — `QGCMapPolyline::removeVertex`, the same
function for the other shape, has `||`. It is the only instance of the broken form in the tree. The
warning next to it also named `removePolygonCoordinate`, a function that no longer exists, so it
now names itself.

### The sweep that followed, and a crash at the end of it

The rule from the fence work — *a head-side call to a `void` core method needs a guard duplicating
the core's refusal, because the bridge's `ok` structurally cannot report it* — was worth applying
to every such call rather than only the one that prompted it. Every fence, circle and rally
operation the map invokes is `void`: `addInclusionPolygon`, `addInclusionCircle`, `deletePolygon`,
`deleteCircle`, `adjustVertex`, `removeVertex`, `addPoint`, `removePoint`. So `onBridge`'s failure
branch — the one that shows "Adding fence did not work" — can never fire for a core refusal. It
still catches a wrong path or a thrown call, which is a real failure mode and how the
`view.messages` key mistake would have surfaced, so it stays.

Checking which of those refusals are actually reachable turned up something better than the
reporting question. `QGCMapPolygon::adjustVertex` and `QGCMapPolyline::adjustVertex` have **no
bounds check at all**:

```cpp
_polygonPath[vertexIndex] = QVariant::fromValue(coordinate);
_polygonModel.value<QGCQGeoCoordinate*>(vertexIndex)->setCoordinate(coordinate);
```

An out-of-range index is an out-of-bounds write followed by a null dereference. Every sibling
guards — `deletePolygon`, `deleteCircle` and both `removeVertex` implementations all bounds-check
— so this is an omission, not a convention. It is reachable: the head captures a vertex index at
drag start and the polygon list is re-polled underneath, and upstream's own
`QGCMapPolylineVisuals.qml` passes `menu._removeVertexIndex` into `adjustVertex`, an index that a
removal has already invalidated.

Both are guarded now, and `QGCMapPolygonTest::_testOutOfRangeVertexIndex` drives -1, count and 99
into `adjustVertex` and `removeVertex` and asserts the polygon is untouched.

**The test was checked against its own absence.** Reverting the guards and rebuilding, it does not
merely fail — it dies with `ASSERT failure in QList::operator[]: "index out of range"`, which is
the crash the guard prevents, confirmed in a real binary rather than argued from the source.

**And the run that said the test passed had not run it.** The first "ALL TESTS PASSED" came from a
binary that did not contain the new slot: `cmake --build build-test --target AircastQGC` relinks
`build-test/Debug/libAircastQGC.dylib`, but the executable resolves `@rpath` to
`AircastQGC.app/Contents/Frameworks/libAircastQGC.dylib`, a copy the named target does not refresh.
Totals read 8, the same as before the test existed, which is the only reason it was caught.
Building the default target refreshes the bundle and the total goes to 9. This is the same trap as
`--target AircastQGC` no longer compiling the Swift: **naming a target gets a green result for
something other than what you are about to run.**

### The links screen stops writing a sentence the core already writes

The core session put `heardVehicle` and a three-way `statusLine` into `view.links`, so the head's
`linkStatusLine` became a second implementation of a rule that has one home. Both were deleted —
`configuredRows` too, because the projection serves `configured` already filtered, and the head no
longer needs to know what `dynamic` means.

Removing it fixed a latent bug that was never reported. The old `LinkRow.index` was the position in
the *unfiltered* element array, carried through a filtered list on the assumption the two stay
aligned. Every invoke path is built from that index. It happened to be correct; it was correct by
coincidence. The row now takes the `index` the core assigns.

**The wording is the core's, including the part I would have argued against.** My three were
"Not connected" / "Open · nothing received yet" / "Receiving from the vehicle". The core's are
"Not connected" / "Waiting for the vehicle" / "Connected". I was ready to defend "Receiving from
the vehicle" on the grounds that "Connected" is the exact word that was wrong, and an operator
reads one row rather than seeing three states at once. That argument does not survive Jakob's Law:
"Connected" is what every operator expects of the good state, and spending the novelty budget on
the *success* state to fix a defect that lived in the *failure* state is the wrong trade. With
"Waiting for the vehicle" occupying that state, "Connected" is only ever shown when it is true.

Verified on the handset with the port held constant: a link on 14999 with nothing sending reads
"Waiting for the vehicle · UDP port 14999" in muted grey; the sim retargeted to 14999 turns the
same row bold and reads "Connected · UDP port 14999". Neither sentence exists in the head any more.

### An iteration lost to `git status`, in a tree where it cannot be believed

I deferred this work for an iteration because `git status` showed `MM core-rs/src/links.rs` and I
read that as another session editing the file. It was not. Measured:
`git show HEAD:core-rs/src/links.rs | diff -q -` reports identical, and in the same minute
**`git status --short` claimed 42 modified files while `git diff HEAD --name-only` listed one.**

The rule that this tree's shared index makes `git diff` unreliable was already written down. It
did not help, because I never ran a diff — I read a status letter and inferred a *person*. Those
are different mistakes and only the first was recorded. An `M` here is not evidence that anybody
touched the file, so it can never justify waiting on a peer. To ask whether a file differs from
HEAD, diff against HEAD. To ask whether someone is working on it, ask them.

The one genuinely modified file turned out to be 126 uncommitted deletions in the *other* head's
document, which no commit was holding. Raised with them rather than touched.

### Remote Support could be started and never stopped

The screen sends the vehicle's live MAVLink — position included — to an address the operator
types, for as long as the link stays up. It had a Start button, a status line, and no way to stop.
The footnote sent the operator to a different screen: "Remove the forwarding link from Comm Links
to stop it", where they would have to recognise which of the listed links is the forwarding one.
An action with an ongoing privacy consequence needs its off switch where its on switch is.

Underneath it was worse than missing. `LinkManager::_mavlinkSupportForwardingEnabled` is assigned
`true` in exactly one place and **never assigned `false` anywhere**. It is a latch. So once
forwarding started, the status line read "Forwarding" for the rest of the session no matter what
happened to the link, and because the Start button was `enabled = !forwarding`, it was permanently
disabled — following the footnote's own instructions left the screen insisting it was still
forwarding and refusing to start again.

The flag was also redundant. `LinkManager::mavlinkForwardingSupportLink()` already answers the
question against the live list, so the cached bool duplicated it and was the copy that could go
stale — the same shape as every "a head re-derives what the core already knows" finding in this
document, one level down. `mavlinkSupportForwardingEnabled()` is now
`mavlinkForwardingSupportLink() != nullptr`, the member is deleted, and the change signal is
emitted from `_linkDisconnected`, which is the single point where a link leaves `_rgLinks`.

`endMavlinkForwardingSupportLink()` is the missing symmetric call, added beside
`createMavlinkForwardingSupportLink()`. The head offers Stop while forwarding and Start otherwise.

**Verified by watching the packets, not the label**, since the label is what was broken. With the
support host pointed at a UDP listener on this machine: Start put 66 packets on the wire in 25
seconds and the screen read "Forwarding"; Stop froze the count at 6 for the following 25 seconds
and the screen read "Not forwarding" with Start enabled again — a state that was unreachable
before this change.

Also deleted: a `reloads` counter incremented by the fact row's callback and read by nothing.
Nothing recomposes on a state that has no reader, so it was not a refresh mechanism, only the
shape of one.

### A green suite described a tree that was not committed

The start-of-turn index check — run unscoped this time, after the other head pointed out that a
path-scoped one is blind by construction — found 34 files in `aircast-android` differing from
HEAD. They were not a peer's work. They were mine, reverted by my own commit.

`a3330b9`, whose message describes a two-file change to the links screen, actually wrote **37
files, 360 insertions, 856 deletions**. It undid the `view.messages` "items" key fix, the arming
predicate, `ui.sh`'s input guard and five test files. The cause is the one already written down
and not applied here: in this repo I used plain `git add <paths> && git commit`, and `git commit`
writes the *whole* index, so it took every stale blob the shared index was holding. Every android
commit after it was correctly scoped, because that one had already absorbed the stale index.

**The reason six commits passed before it was noticed is the part worth keeping. `./gradlew test`
builds the working tree, not HEAD.** Every "289 tests green" in this document was measuring disk,
and so was every APK verified on the handset. Both were true and neither said anything about what
had been committed. A test suite cannot detect a bad commit, because it never reads one.

So the check is not whether the tests pass. It is whether `git diff HEAD --stat` is **empty** —
only then does a passing suite describe the commit rather than the desk it was written on. That
now runs after committing, not before.

Repaired in `311ca37` by re-landing the working tree, which had held the correct content
throughout; `git diff HEAD` is empty and HEAD now contains the symbols the device was verified
against. No work was lost, because nothing had been rebuilt from HEAD in between — which is luck,
not a mitigation.

### The badge that sent an operator to a page that could not help

Recorded earlier in this document as "reported rather than fixed", deferred because the setup
page table "belongs to another session's work in progress". That claim came from reading
`git status`, which this tree cannot support — the android repo had nothing in flight and had not
for a long time. The deferral rested on the same bad inference as the one that cost an iteration
two sections ago, and it had been sitting here as a settled reason.

The finding stands as written: the list badges Radio "Needs setup" in red, it is the one item
blocking flight, and tapping it opens a page that cannot resolve it. What was missing was a third
state — a native page that *monitors* but cannot *complete*. The core now serves `completes`
beside `native`, false for Radio alone, and the row reads **"Finish on desktop"** before the tap.
It stays red and stays openable, because it does still block flight and the live channel monitor
is worth reaching. Verified on the handset in both the "needs setup before flight" list and the
full list; no other row changed.

**Chasing it found 206 lines that no longer ran.** `hasNativeSetupPage` was a second
implementation of the core's `has_native_page`, and the parameter-section tables it depended on —
`SAFETY_APM`, `SAFETY_PX4`, `POWER_PX4`, `LIGHTS_APM`, `CAMERA_APM`, `TUNING_APM`, `FRAME_APM` and
the rest — existed only to feed it. Nothing in `main` called any of it: the screen reads `native`
from the projection, and `ParameterForm` fetches its sections from the core by page name. It was
reachable from its own tests and nowhere else.

It had also drifted without anyone noticing, which is the part worth keeping. The head's copy said
Motors has no native page; the core says it does. A test asserted the head's answer — *"a
component with neither a form nor a custom page stays closed"* — so the contradiction was pinned,
green, and looked maintained. Deleting the function meant deleting the four tests that were its
only callers. `SetupPages.kt` is 206 lines down to 5.

**A smaller self-inflicted one, caught by counting.** `SetupViewTest.kt` was created with a
`cat >>` heredoc appending nothing to a file that did not exist, which produces an **empty `.kt`
file that compiles silently and passes**. It was only noticed because the executed-test count did
not move the way removing four and adding five said it should. The arithmetic caught what the
green suite could not.

### The radio page had never rendered the half that matters

Reviewing the page behind the badge fixed above. This document already described it: "the mapping
reads 'Not mapped' four times and the channels move, which is exactly what an operator needs to
see". That was written about a state the rig manufactured.

`RadioComponentController` builds its mapping from `RCMAP_ROLL`, `RCMAP_PITCH`, `RCMAP_YAW` and
`RCMAP_THROTTLE`, and reads reversal from `RC<n>_REVERSED`. **The sim served none of them.**
`getParameterFact` returned null for every one, the mapping stayed at `_chanMax`, and every
attitude row took the "Not mapped" branch. So the other branch — the PWM bars, the live values,
the reversed marker — had never rendered once, on any run, and the page had been recorded as
verified. An empty screen is not evidence that a screen works, and neither is a screen full of
"Not mapped".

The rig now serves the mapping (`RCMAP_*`, and `RC<n>_MIN`/`MAX`/`REVERSED` for eight channels,
with channel 4 reversed). The result is worth stating precisely, because the detail is what proves
the indirection rather than a coincidence:

| Row | Maps to | Shows |
|---|---|---|
| Roll | ch1 | 1500 |
| Pitch | ch2 | 1500 |
| Yaw | ch4, reversed | **1500 R** |
| Throttle | ch3 | **1100** |

Throttle is the decisive one — a distinctive value on a channel that is neither first nor in row
order, arriving in the right row. And "R" had never appeared before: it comes from
`qgcDouble` against `rollChannelReversed`, which upstream declares
`Q_PROPERTY(int ...)` while its getter returns `bool`. The head reads each of these with the
accessor matching the *declared* type, so it is correct — but correct because of an upstream
typo. If that declaration is ever corrected to `bool`, `qgcDouble` falls to its fallback and every
channel silently reads not-reversed.

**And the badge state this began with was itself the rig.** With `RCMAP_*` present the vehicle
reports the radio set up, the "needs setup before flight" section disappears and the header reads
"Ready to fly". The earlier note that Radio "is the one item blocking flight" was describing a
missing sim parameter, not a product state. The "Finish on desktop" badge remains correct and
still serves a genuinely uncalibrated radio — but it was reached through a rig artefact, and this
document said otherwise.

### And then the sticks moved

The claim above — that stick movement cannot be simulated — was true of SITL and quietly applied
to the whole rig. `apmvehicle.py` is a sim written for this work, and its `rc_channels_send` was
passing eight hardcoded constants. So "the channels move", recorded here as observed, described
something that had never happened: the bars were rendering a fixed number.

That matters more than it sounds, because the operator's question on this page is not "what is the
value" but "does it follow my stick". A static bar demonstrates the value is *read*. Only a moving
one demonstrates it is *tracked*, and only diverging values demonstrate each row follows its own
mapped channel rather than all of them reading the same one.

The sim now sweeps each channel on its own phase. Two captures three seconds apart:

| Row | First | Second |
|---|---|---|
| Roll | 1862 | 1180 |
| Pitch | 1487 | 1215 |
| Yaw | 1314 R | 1873 R |

Each row moves independently, the bar length follows the number, and the reversed marker stays put
on the one channel configured for it. `STILL_STICKS=1` restores the constants for any test that
wants a fixed frame.

What remains true is the narrower claim: calibration itself still cannot be driven, because that
needs the stick-extremes state machine, not merely moving values.

### A view shape changed without re-recording the contract

`completes` went into `view.setup` without re-recording
`test/Bridge/fixtures/view-shapes.json`, which is the authority on view shapes and is compared
field-for-field by `QGCCoreCTest`. The contract test therefore went red for **every session
sharing this checkout**, not just this one, until another session hand-patched the key.

Re-recording properly afterwards produced a file byte-identical to that hand edit, so nothing was
lost — but that was worth checking rather than assuming, because a hand-written fixture is a guess
until proven and the entire point of the file is that it is recorded rather than written. The
procedure is one command, `QGC_RECORD_VIEW_CONTRACT=1` against the test, and the fixture belongs in
the same commit as the change that moved the shape.

### The rig now proves it commanded nothing

Prompted by the macOS session finding a probe action that could upload a plan to a connected
vehicle. Their rule — a hook for a forbidden action must be *incapable* of it, not merely unused —
sent me to audit this rig for the same shape.

**It is not here, and that is worth stating rather than inventing.** No tool in `tools/` is named
for a flight action: there is no arm, takeoff, upload or motor helper. `ui.sh` is generic input.

The real exposure is different and duller. `regress.sh` taps eight fixed coordinates, and **the
layout has shifted under fixed coordinates twice in this session's own work** — once when the
status strip appeared and pushed everything down about 180 px, once when the keyboard moved a
dialog. A tap meant for a tab can land on a flight control. On the sim that is harmless; the
CLAUDE.md standing instruction is that real devices get connected to this rig.

The guard is a proof rather than a restriction, because restricting generic input would break the
tool for its actual job. The run now greps its own sim log for evidence that it commanded the
vehicle, and fails if it finds any — arm or disarm, takeoff, and the ids that fall through to the
sim's generic `CMD` line: return to launch, land, flight termination, parachute, mission start.

**Every branch of that pattern was checked against forms the sim really emits**, because a guard
listing states that cannot occur is the defect this document has found four times already. Eight
dangerous forms match; five routine ones — `CMD 512`, `MODE -> 5`, statustext, camera info,
param set — are ignored. Then the guard was run against a real log with one `ARM` line appended
and returned 1. It fails on demand, not only in principle.

A clean run now ends with `commanded the vehicle: 0`.

### Three things that were noted and are now built

**The head owns which of its own pages can finish a job.** `completes` was the wrong shape and I
put it there: the core owns what the vehicle and firmware are, but "can this head finish that
page" is a claim about which screens somebody wrote, and the two heads genuinely differ — macOS
drives `radioCal` through Start/Next/Skip/Cancel and completes the calibration, this one only
watches. The core session removed the field; the badge now comes from a set in the head, next to
the screens it describes.

This does re-introduce head-side page knowledge one iteration after 206 lines of it were deleted,
and the distinction is worth stating because it is not obvious. What was deleted was *dead* —
called by nothing in `main`, reachable only from its own tests, and silently drifted from the core
with a test pinning the contradiction. A live set the setup screen reads cannot drift in silence:
drifting means the screen visibly stops opening a page. The failure mode removed was invisibility,
not head-side knowledge. A test asserts every page name the head claims appears in the page list
the core offers.

Verified with the core serving zero occurrences of `completes`: Radio still reads "Finish on
desktop", and every other page in the list carries no badge — which was the breakage risk, since
an absent field decodes to `false` and would have promised a desktop finish for all of them.

**A bool the core declares as an int still reads as true.** `qgcBool` accepted only `true` and
`"true"`; `qgcDouble` accepted only numbers and strings. The radio readouts happen to sit either
side of that line — `...ChannelMapped` is declared `bool`, `...ChannelReversed` is declared `int`
with a getter returning `bool` — so each was read with the accessor matching its *declared* type
and both worked by accident of an upstream typo. Correcting that typo would have silently turned
every reversed channel normal, on the one readout radio setup exists to catch; widening the other
way would have read every stick as "Not mapped" for ever. Both accessors now take booleans,
numbers and strings, so neither depends on which declaration upstream carries. The core session
found the same coupling from the other side and landed the same fix there.

**Input refuses to reach a flight control by accident.** Noted last iteration and left as a note;
the other head's argument — that a capability outlives the care of whoever wrote it, and "only my
own care" is the one thing that cannot be audited — is correct. `uiautomator dump` exposes labelled
nodes with bounds, so `ui.sh tap` can now ask what is under the coordinates before sending
anything, and refuses when the answer is Arm, Takeoff, Land, Return, Emergency stop, a mission
command or a gripper action. `ALLOW_FLIGHT_COMMAND=1` makes it deliberate rather than accidental,
which is the whole distinction.

Checked in both directions on the handset rather than in principle: a tap at the centre of Actions
is allowed; a tap at the centre of Arm is refused, naming the control it would have hit. The
regression's eight taps now carry a hierarchy dump each and the whole run takes 93 seconds.

The rig also gained `NO_RCMAP=1`, because with the mapping present the vehicle reports the radio
set up and the badge under test cannot appear at all — both states have to be reachable or only
one of them is ever tested.

### Sensor calibration comes from the core, rules included

The core session landed calibration and pointed the Android head at `view.coreCalibration`. That
would have been an empty screen. There are two registrations, and the difference matters:
`view.coreCalibration` is computed by `hub::core_calibration_view`, which reads the Rust core's own
MAVLink state machine — populated only when the core owns the link. This head's vehicle belongs to
Qt. **`view.calibration` is the one that projects `sensorsCal`**, and it is what the screen now
reads. Checked in the source before building anything, because the failure would have looked
exactly like a screen that works and shows nothing to do.

The migration replaces roughly thirty-five individual watched paths with one: six for the running
state, five for the list, and twenty-four for the orientation grid, which read
`orientationCal<Side>Side{Visible,Done,InProgress,Rotate}` four at a time across six sides. The
core serves those as a `sides` array with a single `stage` per side.

**The rule went with it, which is the part that matters.** `blockedByAccel` and the
`needsAccelFirst` flags were a head-side copy of a safety rule — a compass calibrated against an
uncalibrated accelerometer gives an operator a result they have no reason to distrust. The core
now answers `blocked` and `enabled` per routine, and the head honours the answer instead of
re-deriving it. The head-side tests for the rule were deleted only after confirming the core tests
it, including the invocation string and its arguments.

**What did not move is the copy.** The core's `explanation` is terser than what this screen said,
and its Level Horizon routine has no counterpart to "Get this wrong and it will drift in flight."
Instruction text keyed by routine id stays in the head, falling back to the core's description for
any routine the head has not written copy for. Presentation is head knowledge on the same argument
that took `completes` out of the core.

**One safety property was nearly lost in the move.** A head-side test asserted that no offered
calibration spins a propeller. With the list now coming from the core, that assertion had nothing
left to check — so it moved to `core-rs/src/calibration.rs`, where the list lives, rather than
being deleted with the code it guarded.

Verified on the handset in both directions, which needed a new lever: with the accelerometer
calibrated all five routines are listed and openable; with `ACCEL_UNCAL=1` the Accelerometer reads
"Not calibrated" in red and stays openable — it is the way out — while Compass and Level Horizon
read "Calibrate the accelerometer first" and cannot be tapped, and Gyro and Pressure are
unaffected. The sim served only `INS_ACCOFFS_X`, and the rule needs all three axes at zero, so
the blocked branch had no way to appear before this.

### The flight modes page was unreadable, and it was the rig again

Third time this pattern has appeared, so it is worth naming as a pattern rather than an incident.
The Flight Modes setup page showed `FLTMODE_CH` labelled with its raw parameter name and valued
`5.000`, and `INITIAL_MODE` the same, while the mode slots beside them read "Flight Mode 1 · Acro".
Two of seven rows unreadable on a page whose entire job is telling an operator which switch
position does what.

Not a head defect. APM parameter metadata is not bundled the way PX4's is — it comes from
`ArduPilot-Parameter-Repository`, one directory per firmware version. `Copter-3.5/apm.pdef.json`
has no entry for `FLTMODE_CH` or `INITIAL_MODE`; `Copter-4.5` and `Copter-4.6` have both. The
measurement that settled it: **QGC asks for message 148, `AUTOPILOT_VERSION`, and the sim had never
answered.** With no version, the oldest metadata wins.

The sim now answers with 4.5.7, and the same page reads "Flightmode channel · Channel5" as a
dropdown and "Initial flight mode · Stabilize". `FIRMWARE=x.y.z` overrides it.

**What the fix revealed is a real finding underneath.** With current metadata, `SUPER_SIMPLE`
stops being an enum and becomes a bitmask — it is per-mode bits in modern firmware — and the head
renders bitmask parameters as a three-decimal float. "Simple mode bitmask · 0.000" cannot tell an
operator which modes have simple mode on. That is head work, recorded rather than fixed here.

Two smaller ones from the same hour. The first version message crashed the sim: `uid2` is
eighteen bytes and I passed eight. **The log line said "AUTOPILOT_VERSION sent" because it printed
before the send** — so the rig reported success for a call that raised. Log after the action.

### The stale notice now comes from the core, and the path is confirmed for all three heads

`view.flyState` landed with `contactLost`, a `state` token, a `stateText` sentence and
`staleNotice`. Both heads had hand-written that sentence and the macOS session had copied this
one's wording to avoid diverging, which is exactly the duplication the core exists to remove.
Deleted here: `vehicleSubtitle`'s own precedence rule and the head's copy of the notice, which used
an ASCII hyphen where the core uses an em dash.

**The measurement the other two sessions could not make.** Neither has a vehicle, so neither could
prove the bridge traverses `vehicle.vehicleLinkManager.communicationLost` — a wrong path decodes to
`false`, which renders nothing and looks like a working panel. With the sim killed, `contactLost`
was already true at the first capture ten seconds later: the header read "Communication lost", the
telemetry row dimmed, and the notice rendered with the core's em dash. Restarting the sim cleared
all three within twenty seconds. The path is right.

**One thing did not survive the migration untouched.** `stateText` is a fixed line per state —
"Disarmed" — with `mode` served as a separate field, matching what the macOS header showed. This
header showed "Stabilize · Disarmed", so printing `stateText` alone would have silently dropped the
flight mode from the Fly tab. The head composes mode and state again, and drops the mode when
contact is lost because it is no longer known to be current. Presentation is head knowledge, on the
same argument that took `completes` out of the core.

### A bitmask shown as a decimal, and two layers of rig underneath it

`ARMING_CHECK` decides which pre-arm checks a vehicle runs before it will let you fly. The Safety
page showed it as **82.000**. An operator cannot read which safety checks are switched off from
that, and switching them off is a known way to reach an accident.

The metadata has the answer — `0:All,1:Barometer,2:Compass,3:GPS lock,4:INS,...` — and QGC's
`Fact` exposes `bitmaskStrings` and `bitmaskValues` as properties. **The bridge's fact projection
never emitted them**, so no head could see them. It does now, and the head decodes them, names the
set bits ("Barometer, GPS lock", or "None", or "All") and offers a checklist to toggle them.

**Two rig layers sat underneath, and neither was the head's fault.**

The first: `APMParameterMetaData` builds a bitmask only for integer-typed parameters — for a float
it logs "Invalid type for bitmask" and drops it. **The sim sent every parameter as
`MAV_PARAM_TYPE_REAL32`.** A real ArduPilot vehicle sends proper types, so this had never been a
product problem, only a rig one. The sim now declares which parameters it models as integers, and
the same row went from "82.000" to "82" on that change alone.

The second is the one worth recording, because it corrects something this document implied. **The
parameter form does not use the bridge's fact projection at all.** It reads `view.setup(<page>)`,
which the core renders into `controls` with `label`, `options` and `control` — a different pipeline
from the `Fact` objects `FactRow` receives elsewhere. So the bitmask work is enabled end to end in
the bridge and the head, and remains inert on the setup pages until the core carries bitmask
information in that projection. Raised with the core session rather than worked around with a
per-control fact read, which would restore exactly the N-reads-per-page pattern three of these
migrations have removed.

Stated plainly because a feature that cannot fire is the defect this document has found five times:
the picker does not appear on the **setup pages**. What is reachable and verified is the integer
typing, and the decode is covered by tests that would have caught the ragged case where bit names
and values disagree.

**Correction, added later the same session: the picker is reachable, on the Parameters screen.**
That path goes through `Qgc.factAt` and the full `factJson`, which carries `bitmaskStrings` and
`bitmaskValues`, so it never needed the core's `controls` at all. `ARMING_CHECK` at 82 renders as
"Barometer, INS, RC Channels" — bits 1, 4 and 6 — and the checklist opens against real ArduPilot
metadata with exactly those three ticked. Only the setup pages wait on the core.

The error is worth keeping rather than quietly fixing: this document, and two other sessions, were
told the feature was inert, because one path was checked and the conclusion was generalised. It is
the same mistake as every rig finding above, pointed at my own work instead of the sim's.

### A capture photographed the wrong app

`ui.sh` exists because taps once landed in the user's private messages. Every input verb checks
`topResumedActivity` and refuses rather than guess. **`shot` never did.**

The gap showed itself the way these do. A back-press from a settings sub-page left the app
entirely, the next `shot` captured whatever was on screen, and what was on screen was the user's
Telegram. Deleted immediately, as before.

The input guard was built for the wrong half of the problem. It asks "will this input reach the
app", which stops a tap going somewhere else — but a capture does not send anything, so it was
never gated, and reading the screen is the half that carries the privacy cost. `shot` now refuses
unless the app is in front, with `ALLOW_ANY_SCREEN=1` for the case where photographing the launcher
is the actual intent.

Checked in all three directions, because a guard that only ever refuses is indistinguishable from
a broken tool: in front it captures and writes 1.7 MB; backgrounded it refuses and **writes no
file**; with the override it captures anyway. The first attempt at this test reported a refusal for
the passing case too — the app was not actually in front — and that looked like success until the
positive case was made to work.

The lesson is narrower than "guard the tool" and worth stating: **the first guard was scoped to
what the tool sends, and the danger was in what it reads.** Two verbs, one hazard, and only one of
them was covered for months.

### The tablet took the RC channels and never gave them back

The on-screen RC controls drive vehicle channels from the touchscreen. Two defects, both measured
against the sim's own log rather than argued from the code — and both of my first two hypotheses
were wrong, which is why reading `Vehicle::setRcChannelOverride` before writing anything mattered.

I expected the head to be flooding the link and the vehicle to be expiring the override. Neither
is quite it: `setRcChannelOverride` stores the value, starts a 200 ms timer, and the *core* handles
rate and release. But it also sends immediately on every call. So:

| | before | after |
|---|---|---|
| a 1.5 s drag | **97 messages** | 23, of which ~7 are the timer |
| idle, untouched | **5/s for ever** | 5/s until released |
| leaving the Fly tab | still 5/s | **0** |

Ninety of those ninety-seven were the head calling through per pixel of drag. The head now throttles
to 100 ms and always sends a final value on release, so the vehicle still ends where the finger did.

**The second defect is the one that matters.** `Vehicle::clearRcChannelOverrides` exists, and the
head never called it. Touch one control and that channel is held by the tablet at 5 Hz for the rest
of the session, with no affordance to stop and nothing on screen saying it is happening. There is
no way to hand the channel back to the transmitter.

The layer now releases when it leaves composition — measured: leaving the Fly tab drops the rate to
zero — and shows "These channels are held by this tablet" with a **Give back** button whenever
`rcChannelOverrideActive`. Pressing it produces exactly four `RC_OVERRIDE []` frames, the core's
three release ticks plus the immediate one, then silence.

### A picker wrote the label instead of the constant

From the macOS session, who found the same shape in their own tree and deleted it before it was
wired. Mine was wired. `ExtraVideoSourcesEditor` built its source chips from the `videoSource`
fact's `enumStrings` — the list `setEnumInfo` receives already cooked through `tr()` — and wrote
the selected string into the extra-sources blob, which `VideoSettings`, `VideoManager` and the core
all match against **raw** constants. In English the two lists are identical, so nothing was broken
and nothing would have been until the first translated build.

The picker now displays `enumStrings` and writes `enumValues`. A test asserts that swapping the
labels for Spanish changes nothing about what is written.

Their other warning does not reach this head: `sourceNeedsUrl` lists the same six sources as
`VideoSettings::_sourceNeedsUrl`, so a webcam or a Herelink is not marked faulty for having no
address. That list is still a hand-copy of the core's constants and would drift in silence if a
source were renamed — recorded, not fixed.

### What the QML host still does, read rather than assumed

Phase 6 says delete the `QtQuickView` host. Before planning that, it is worth writing down what
the host is actually still doing, because three of its four connections to the head turn out to be
different from each other in ways the plan treated as one thing.

**`toolSource` is dead and now deleted from the head.** Every tab in the `Tab` enum declared its
tool as `""`, so `appliedTool != tab.tool` was never true, `setProperty("toolSource", …)` never
executed, and `toolLoader.active` was permanently false. The head was carrying a field, a default
constant, a state variable and a branch for a property it could not write. `showTool` in
`AndroidHost.qml` is only ever called from the desktop `MainWindow.qml`, so the QML side is
untouched. Regression green afterwards, all six surfaces and all six tabs.

**`page` is not dead, and the reason is worth recording.** `renderViews: false` stops `FlyView` and
`PlanView` *drawing*; it does not stop them existing. Both are still instantiated and their
bindings still evaluate, and `page` drives `flyViewActive`, which drives `PlanView.planActive`.
That is the same fact behind the earlier measurement that shrinking the view doubled CPU — the
views are alive either way. So `page` stays until the views are gone, not merely hidden.

**`navigateRequest` is live and must survive.** `AutoPilotPlugin` calls
`qgcApp()->showVehicleConfig()`, which does `QMetaObject::invokeMethod(_rootQmlObject(),
"showVehicleConfig")`, which emits `navigateRequest("setup")` and moves the head to the Setup tab.
That is C++ asking the head to navigate, and deleting the host without replacing it would silently
lose it.

**Two things attach to the host that are not navigation at all.** `Component.onCompleted` calls
`QGroundControl.corePlugin.setupEmbeddedEngine(mainWindow)`, and a 300 ms repeating timer calls
`QGroundControl.videoManager.initForItem(mainWindow)` until it succeeds. The second is the one to
check before deletion: the head switched video to `video.setNativeRendering(true)`, so whether the
video manager still needs a `QQuickItem` on Android is now an open question rather than an
assumption, and it decides whether Phase 6 is a boot-path change or a boot-path change plus a video
change.

AAR today: **81 MB**. The plan's estimate that it roughly halves once QtQuick, QtQml, QtGui,
QtLocation, QtMultimedia, QtCharts, QtWidgets and QtPositioning drop out is untested and stays an
estimate.

### An unknown control kind is not made editable

From the macOS session, who adopted the core's `bitmask` control before their decode knew the kind.
Theirs fell through to `.unknown`, and `.unknown` fell through to a text field whose commit handles
text and numbers and drops everything else — so `ARMING_CHECK` drew as an editable box showing 82
that accepted typing and discarded it on blur. Their framing is the part worth keeping: **the bug
was not the missing kind, it was that the fallback for an unknown kind is an editable control.**

Checked here. The silent half does not reach this head — `FactTextField` validates through
`rejectionFor` and reports a refused write, so typing something the fact will not take produces a
visible rejection rather than a quiet discard. But the shape does: anything the head did not
recognise became a number field, and the core has now enumerated a kind this head cannot yet draw.

`view.contract` lists five: toggle, choice, bitmask, text, number. The head carries that set and
renders anything outside it read-only with "Edit on desktop". A kind added to the core later now
degrades to something true and useless rather than to a control that looks like it works. A control
carrying no kind at all stays editable, because that is every other path through `FactRow`.

### Video is the real Phase 6 blocker, not the boot path

Following up the open question from the previous section rather than leaving it open.
`VideoManager::initForItem` is not incidental. `init(QQuickWindow *window)` returns immediately
with a critical log when the window is null, and it uses the window for four things: finding the
main video item by name, initialising every receiver, rebinding widgets, and
`window->scheduleRenderJob(new FinishVideoInitialization(), BeforeSynchronizingStage)` — work that
has to happen on the Quick render thread.

So the video pipeline does not merely *prefer* a `QQuickWindow`; it cannot initialise without one,
and `setNativeRendering(true)` does not change that — it changes where frames go after
initialisation, not whether initialisation can happen. Deleting the `QtQuickView` therefore needs a
video initialisation path that does not depend on a Quick window, which is a larger piece of work
than the boot-path change the plan describes and should be scheduled as its own item.

### The sim never left the ground, so three flight states were unreachable

The whole flight was walked once before. Re-walking it after everything that has changed since
turned up a rig defect that had been quietly making one state untestable and another wrong.

`apmvehicle.py` reported `MAV_LANDED_STATE_IN_AIR` the instant it was armed, and a fixed 25 m
altitude whether armed or not. So arming on the ground read **"Stabilize · Flying"** — the core's
precedence is correct, the vehicle was lying. `armed` was unreachable as a distinct state, `landing`
could never occur, and the altitude readout was a constant.

The sim now has an altitude, climbs at 2 m/s toward a target, reports `ON_GROUND` below half a
metre, `LANDING` while descending, and disarms on touchdown. **Land is a mode change, not a
command** — QGC's land action sets ArduCopter mode 9 rather than sending `MAV_CMD_NAV_LAND`, which
is why the first attempt changed the mode and never descended. The descent is now driven from the
mode.

The walk, each line read off the handset with the sim's log beside it:

| step | header | altitude | the vehicle received |
|---|---|---|---|
| on the ground | Stabilize · Disarmed | 0.0 m | |
| armed | **Stabilize · Armed** | 0.0 m | `ARM armed (param1=1.0)` |
| after takeoff | Guided · Flying | 3.0 m | `TAKEOFF alt=3.0` |
| after landing | Land · Disarmed | 0.0 m | `LAND requested from 3.0 m` |

Arm and Land both go through a slide-to-confirm; Takeoff opens the height dialog with its range
hint. `Armed` as a state distinct from `Flying` had never been seen on any of the three rigs.

**Adding a land handler nearly killed a guard.** The regression's flight-command check matched
`CMD 21` because land used to fall through to the generic handler. Giving the sim its own `LAND`
line would have left that alternative permanently unmatched — the dead-branch defect this document
has now found six times, this time about to be self-inflicted. The pattern was updated to `LAND `
and re-proved: seven dangerous forms match, five routine ones ignored.

### A plan either side of the antimeridian framed the planet

From the macOS session, who found it in their own map framing and said to check mine. Mine had it.
`planBounds` took `minOf`/`maxOf` on longitude and `centre` was the arithmetic mean, so two
waypoints a fifth of a degree apart across the antimeridian — 179.9 and -179.9 — produced a centre
of 0 and a span of 359.8 degrees. The map would have jumped to the Gulf of Guinea and zoomed out to
half the world.

Longitudes are now treated as points on a circle: sort them, find the widest empty gap between
neighbours, and the occupied arc is everything else. 179.9 and -179.9 give a span of 0.2 degrees
centred on 180.

**Two of the six tests I wrote for it were wrong, and the code was right.** I asserted that
-10, 10 and 170 span 20 degrees; the widest gap is the 180 from 170 round to -10, so the arc is 180
and the centre is 80. Recomputing by hand rather than adjusting the code is what settled it — a
failing test is not evidence the code is wrong, only that the two disagree.

### Distance to Home had never shown a number

Following the flight walk by looking at what the telemetry row does *not* say. One of its four
readings — Distance to Home — shows `--.-- m` in every screenshot in this document, from the first
Fly tab capture to the last. Not a formatting choice: **the sim has never sent `HOME_POSITION`**,
`Vehicle::_updateDistanceHeadingToHome` sets the fact to NaN without one, and the field has
therefore never once been exercised.

The sim now publishes home at the centre of the orbit it already flies, which makes the reading
predictable rather than merely present: the vehicle circles at 0.004 degrees, so the distance must
sit between the north-south component of that radius and the east-west one at this latitude — 445 m
and 332 m. Measured on the handset across three samples as it orbited: **441.8 m, 355.4 m,
367.9 m.** Inside the band, moving with the vehicle.

That is the seventh rig gap this session and the pattern is now the finding rather than any one
instance. Six of the seven presented as product defects — a mapping that would not populate, sticks
that would not move, parameters with no metadata, an accelerometer that could not be uncalibrated,
a bitmask rendered as a float, a vehicle that was flying while parked. Every one was the rig
feeding a screen something no real vehicle would send. **A screen that looks wrong is not evidence
until the input is known**, and the corollary matters more: a screen that looks right is not
evidence either, if the rig can only produce the case that looks right.

### The sweep for code only tests can reach

The macOS session swept their tree for declarations referenced only by tests, on the grounds that
such a thing reads as machinery while being exercised by nobody. Run here: **1266 declared names,
five reachable only from tests.**

Two are honest test seams, `forgetWatchesForTest` and `watchedPathsForTest`, named for what they
are. Three were not:

- `REMOTE_SUPPORT`, left behind when the dead setup table went, referenced by nothing but its own
  declaration.
- `altitudeLabel`, superseded by `rangeLabel` and kept alive by four tests.
- `factFromParameter`, a second decoder for the same job `ParametersScreen` does inline.

**The third one was worth stopping over rather than deleting.** The two decoders guard differently:
the dead one checks `kind == "fact"`, the live one checks the name is not blank. A non-fact object
can perfectly well have a `name` — `LinkConfiguration` does — so the live path had the weaker test
and the dead copy had the better idea. Deleting the unused one would have thrown away the guard and
left the weaker check in place. `parameterFact` now calls `factFromParameter`, so there is one
decoder with the stricter guard and its tests are live again.

Re-running the sweep leaves only the two named seams. The Parameters screen still lists 249
parameters afterwards, checked on the handset, because a stricter guard that silently drops
everything looks exactly like a stricter guard that works.

### A mission that took off after it had already flown

Phase 4's gate is a vehicle round trip, so the round trip was walked: two waypoints by long press,
then Takeoff, then Land, then Upload. The vehicle received it, and the wire showed the defect:

```
seq=1 cmd=16   waypoint
seq=2 cmd=16   waypoint
seq=3 cmd=22   NAV_TAKEOFF
seq=4 cmd=20   RTL
```

**Takeoff third.** A mission that flies to two waypoints and then takes off. The vehicle accepted
it without complaint, because nothing in the protocol says a takeoff has to come first.

QGC does not allow this and says so explicitly. `MissionController::_recalcAll` sets
`_isInsertTakeoffValid = false` when there are "coordinate based flight commands prior to where the
takeoff would be inserted", and `PlanView.qml` has `enabled: _missionController.isInsertTakeoffValid`
on its Takeoff button. There is a matching `isInsertLandValid`, and an `onlyInsertTakeoffValid` for
when a takeoff is required before any waypoint at all. **This head read none of them.**

It reads all three now, and the plan summary says "add a takeoff before anything else" while that
is the only legal move.

**The gate alone was not the fix, which is the part worth recording.** With a waypoint present,
`isInsertTakeoffValid` is *true* — because those flags describe inserting at the **current item**,
and this head always appends at the end. Reading the flag without matching the insertion point
answers a different question than the one being asked. So when the core says a takeoff must come
before the rest, the head now inserts at the first flight slot rather than appending. Same mission
rebuilt in the same order:

```
seq=1 cmd=22   NAV_TAKEOFF
seq=2 cmd=16   waypoint
seq=3 cmd=16   waypoint
```

### The input guard refused a safe action

Found by being stopped by my own tool: on the Plan tab, "Takeoff" and "Land" add mission items and
command nothing, and the guard refused both because it matches on the label alone. A guard that
blocks safe work gets switched off by whoever hits it, which costs more than it protects.

It now looks for the Plan tab's own "Upload" and "Download" labels and treats flight-control names
as mission items when both are present. The direction of failure matters: if those labels ever
change, the guard becomes *stricter*, not weaker. Proved in three cases — the Fly tab still
refuses, the Plan tab allows, and a single marker is not enough.

Also: the handset was attached twice, by USB and over TCP, and every `adb` call started failing
with "more than one device/emulator". `ui.sh` now pins the first non-network serial, so one physical
device answering on two transports stops being a rig outage.

### The log handler was installed as its own fallback

Chasing why the app would not come to the front turned up three `SIGSEGV`s on the handset today —
02:46, 09:50 and 12:23 — every one of them **"stack pointer is not in a rw map; likely due to stack
overflow"**, on the Qt thread, with 512 frames at a single address in `libAircastQGC`.

Symbolised against the unstripped library, that address is
`msgHandler` at `src/Utilities/QGCLogging.cc:41` — the line that calls `defaultHandler`. So
`defaultHandler` was `msgHandler`: **the Qt message handler had been installed as its own
fallback, and every log line after that recursed until the stack ran out.**

`installHandler()` does `defaultHandler = qInstallMessageHandler(msgHandler)`, and
`qInstallMessageHandler` returns the *previous* handler. Call it once and the fallback is Qt's
default. Call it twice — which an Android process can do when the Qt entry runs again in a surviving
process — and the fallback becomes `msgHandler` itself. One line of guard makes the state
unreachable: keep the previous handler only when it is not the one just installed.

Eight restart cycles afterwards with the app genuinely reaching the foreground each time: no
crashes. That is consistent rather than conclusive — the fault appeared three times in ten hours of
restarting — but a handler that calls itself is never correct regardless of how often it bites.

### The handset rotated and the regression did not notice

The same run reported `log list requests: 0` where it has always been 1. Not a code regression:
**the handset was lying in landscape**, `mCurrentOrientation=3`, auto-rotate on. Every coordinate
in `regress.sh` assumes portrait, so the Analyze taps landed on nothing.

Every screenshot still reported `ok`, because the capture guard only asks whether the frame is flat.
**The one thing that noticed was a count that changed** — the same signal that caught an empty test
file earlier in this session, and the reason this document keeps insisting on numbers rather than
green.

The lock now guarantees the state its callers assume rather than reporting it: it pins portrait and
**refuses to hand out the device** if the orientation will not take, exactly as it now refuses a
handset that will not wake. A rig that hands you a device in the wrong state is worse than one that
refuses, because the run still produces screenshots and they all look fine.

### The round trip closes: Phase 4's gate is met

Phase 4's gate is a vehicle round trip. Both halves are now walked end to end against the sim's own
log rather than against the screen:

1. Build a plan, Upload — `UPLOAD done items=3`, the wire carrying home, `cmd=22` takeoff and a
   waypoint, in that order.
2. File → New plan — the summary reads **"Empty plan · long press to add"**, and the vehicle keeps
   its mission.
3. Download — `DOWNLOAD start type=0 count=3`, and the plan comes back as
   **"2 items (takeoff) · 446 m · 1:46"**: the two flight items plus the home item that is not
   counted in the summary.

**Two confirmations were found by walking it, both good.** "Clear mission" warns that it removes the
mission *from the aircraft* — the destructive half is named rather than implied. And Download is a
two-step arm: the first tap relabels the button **"Discard & download"**, because the local plan is
about to be replaced.

**I misread my own instrument twice on the way, and both are worth recording.**

The first: after clearing locally I read `DOWNLOAD start type=0 count=0` and concluded that
"New plan" had wiped the vehicle's mission — a serious accusation about a destructive side effect.
It had not. That line was from app startup, before the upload; I had taken the tail of *all*
download lines rather than the ones since my action. Grepping a log without bounding it to the
window under test is the same error as reading a screen without knowing what produced it.

The second: the download that "did nothing" had in fact armed a confirmation. The tap landed, the
button was enabled, and the screen said `Discard & download` — which I only saw because I dumped the
screen immediately after the tap instead of after a nine-second wait. **A delay long enough to let
the action finish is also long enough to hide what the action asked for.**

### Three flags that are only true just after a selection

Correcting work from two sections above. Gating the Plan tab's Takeoff and Land buttons on
`isInsertTakeoffValid`, `isInsertLandValid` and `onlyInsertTakeoffValid` was wrong, and walking the
plan-file flow showed it: the summary read **"2 items (takeoff) · add a takeoff before anything
else"** — a contradiction, produced by my own hint.

All three are assigned in exactly one place: inside `MissionController::setCurrentPlanViewSeqNum`,
and only when the sequence number actually changes. They describe *inserting at the item the
operator has selected*, and they are recomputed only when that selection moves. QGC's `PlanView`
drives that continuously; this head never calls it. So the values a head reads are whatever the last
selection left behind, or the member defaults — and the defaults are
`_onlyInsertTakeoffValid = true` and `_isInsertLandValid = false`. **A head reading them cold gets a
permanent "add a takeoff first" and a permanently disabled Land button.**

`_takeoffRequiredBeforeWaypoint()` is also narrower than its name suggests: it is
`_visualItems->count() == 1`, meaning the plan is empty, not that a takeoff is missing. And
`addWaypoint` inserts a takeoff itself when it is true, so the rule the hint was announcing is one
the core already applies.

The three reads and the hint are gone. What stays is the part that was actually right — a takeoff
added to a plan that has waypoints and no takeoff goes in front of them rather than after — and it
is now derived from the item list the head already holds rather than from a flag scoped to an
operation the head never performs. Verified on the wire again afterwards: `cmd=22` at seq 1, ahead
of the waypoint.

**This is the exact shape I had described to the macOS session two iterations earlier** — a flag is
scoped to an operation, and reading it outside that operation's shape gives a correct answer to the
wrong question — and I walked into it while quoting it. The reason it survived review is that the
unit tests passed a boolean *in*; nothing ever checked that the boolean could be read.

### The plan-file round trip is still unverified, and the rig is why

This document flags saving and opening `.plan` files as a **regression to prevent** — the QML Plan
view has it, the native tab must have it before Phase 6 deletes that view. The flow is built:
`File` offers Open, Save, Save as, Export KML and Import boundary. Walking it end to end failed,
and the failure is in the rig rather than the app.

`Save as…` opens Android's system file picker — the Storage Access Framework, correctly, because
scoped storage gives an app no other way to write where a user can find it. That picker is a
different process with its own view hierarchy, and the rig cannot drive it reliably: the filename
field and the save button are `EditText` and `ViewGroup` nodes whose resource ids repeat across the
file list, so selecting "the node whose id ends in `title`" picks a row in the listing rather than
the field. Three attempts, three misses, **no `.plan` written**.

Stopping there rather than continuing to guess at coordinates. What is established: the flow opens
the right picker, the app survives the attempts with no crash, and the plan is intact afterwards.
What is **not** established, and is recorded as unverified rather than assumed: that a plan saved to
a file can be opened back.

Two things follow. The picker is also what `Open…`, `Export KML…` and `Import boundary…` use, so all
four share this gap. And driving a second app's hierarchy needs the rig to select by *text* within a
bounded container rather than by resource id, which is a rig capability that does not exist yet —
worth building before this gate can be closed, not worth improvising blind taps to fake.

The regression's own screens still pass and the vehicle round trip from the previous section is
unaffected; this is a hole in coverage, not a regression.

**Closed on 2026-09-11. The capability that was missing now exists.** Selecting by text rather than
by resource id is exactly what the rig does after `a8d19fa` taught its readers to see single-quoted
attributes, and the picker's own nodes turned out to be reachable without ids at all: the filename
is the only `android.widget.EditText` in the hierarchy and the button is the only node labelled
`SAVE`. No resource id is involved and no coordinate is guessed.

The walk, each line read off the handset:

| step | what was done | what the screen said |
|---|---|---|
| plan built | takeoff plus two waypoints | `3 items (takeoff) · 1.15 km · 4:06` |
| `Save as…` | typed `roundtrip.plan`, tapped SAVE | 2218 bytes at `/sdcard/Download/roundtrip.plan` |
| `New plan` | discarded | `Empty plan · long press to add` |
| `Open…` | searched `roundtrip`, tapped the result | `3 items (takeoff) · 1.15 km · 4:06` |

One detail worth keeping for whoever drives the picker next: **the file list did not show the newly
written `.plan` without scrolling or searching**, while the import picker had shown a file placed by
`adb push` immediately. Tapping the picker's own Search and typing part of the name is reliable
where reading the visible list is not.

The vehicle round trip was re-walked in the same pass — upload, clear locally, download — and returns
the identical summary, so both round trips are now covered rather than one.

**All four picker flows are now walked**, not just the two the round trip needed. `Export KML…`
writes 2337 bytes of well-formed KML — `<?xml version="1.0"?>`, `<kml xmlns=…/kml/2.2>`, a
`<Document>` named for the app — and `Import boundary…` was exercised earlier the same day with a
deliberately malformed `.plan`, which the head refused with **"That is not a plan file. The current
plan is unchanged."** So the gap this section opened is closed across all four, and the refusal path
is covered as well as the success path.

**Closed. The round trip works, and the rig gap was smaller than it looked** (`971d82c`).

Two things were wrong, neither of them resource ids. First, `ui.sh` allowed *reading* the picker —
it is the app's own flow continuing in another process — but `tap`, `type`, `swipe` and `key` still
required the app itself to be in front, so the save dialog could be read and never filled in. The
flight-control guard now applies only while the app is in front, which is the only place flight
controls exist.

Second, selecting by text failed because the picker is a **grid**, not a list. Each cell has a
clickable preview overlay at its top-right and a non-clickable label at its bottom, ~370 px apart, so
"the node whose text is the filename" is not clickable and the clickable node carries a
`content-desc` instead. Tapping the label's own bounds works — the clickable ancestor receives it —
and `content-desc=Preview the file <name>` finds the cell. No new selector was needed.

Walked end to end on the handset: a plan of takeoff, survey and land saved to `mission.plan` (the
picker offers that name with the extension already on it), the plan cleared to empty and confirmed
empty, then the file opened back — **"mission.plan · not uploaded", 1 item**, where the summary had
read "Empty plan" the moment before. The written file is valid QGC JSON, `"fileType": "Plan"`.

One dead end worth recording so it is not repeated: saving under a name with the extension deleted
writes a valid plan file that the app will not open again. That is a self-inflicted test condition,
not a defect — the picker offers `mission.plan` and only a deliberate deletion removes it.

Both test files were removed from the handset afterwards.

**And the other two flows are verified too, so the whole `File` menu is now covered.**

`Export KML…` offers `mission.kml`, writes 1908 bytes of well-formed KML — XML declaration, the
2.2 namespace, a `Document` with a name and styles — and correctly leaves the plan's own document
name alone, so exporting does not silently rename what a later `Save` would overwrite.

`Import boundary…` reads the file, then asks **"Import as which pattern?"** with Survey, Corridor
Scan and Structure Scan. Choosing one against a *mission* KML — a path, not an area — reports
**"mission.kml holds no area for a Survey."**, naming both the file and the pattern the operator
asked for, and removes the empty item it had inserted to find out.

**That last one was briefly written down here as a silent failure, and it was not.** The notice is
`NOTICE_MILLIS = 4000` and the screen was read seven seconds after the tap, so a message that had
already cleared looked exactly like no message at all. Sample within about a second of the action
when the question is whether anything was said. The same mistake had already produced two other
false findings tonight, on the mission-item refusal and on the preflight tap.

### A flight action that goes grey and says nothing

`view.guidedActions` gives every action an `offer` of `ready`, `blocked` or `hidden` and, when
blocked, a `reason` — "The vehicle's arming checks are failing." or "The pre-flight checklist has
not been completed.". The head decodes that reason into `GuidedOffer.reason`. Only the **More
Actions sheet** ever rendered it, as a subtitle under each row.

On the Fly screen itself, Arm, Takeoff, Land, RTL, speed and altitude were rendered because they
were `shown` and disabled because they were not `ready`, with the reason dropped. An operator
standing at the aircraft with a grey Takeoff was told nothing, and the explanation sat behind a
button labelled "Actions" — which does not read as "why can't I take off". The preflight summary is
behind the same sheet. The reason now renders under the action row (`3208095`).

The tell was dead code. `Arm`'s `onClick` opened with
`blockedReasonFor(armAction)?.let { refusal = it; return@Button }`, which can never run, because
`enabled = armAction?.blocked != true` had already disabled the button in exactly the case that
branch existed to handle. Someone intended to surface the reason and the gate made it unreachable.

Arm was also the only action gated on "not blocked" rather than "ready". Those are equivalent today
— the core builds `offer` from an exhaustive match over three values and `Offered()` drops `hidden`,
so `shown && !blocked` is `ready` — but arm was the one path that would fail **open** if a fourth
state were ever added, on the command that spins propellers. It now matches the others.

Verified on the handset by forcing the line to render, since the sim's vehicle reports every gate
passing and no amount of waiting would have produced a blocked action: the text appears directly
under the action buttons and above the telemetry row. That the reason is non-empty whenever an
action is blocked is guaranteed by the core, where the reasons are string literals on the blocked
branch.

### Sweeping for what the core computes and this head throws away

The guided-actions finding above is a shape, not a one-off: the core produces a field, the head
decodes nothing, and the information dies at the boundary. So I swept every `view.*` the Android
head names — twenty of them — for top-level keys the head never mentions.

**The sweep was wrong twice before it was useful**, which is worth recording because the output
looked plausible each time. The first version counted every quoted key in a Rust file, so nested
per-item keys and `json!` blocks that are not the view root came through as "emitted"; it also only
counted head reads of the form `.optString("x")`, so `options(view, "everyday")` — a literal passed
as an argument — read as unread. That produced a 90-key list of mostly nothing. The second version
took top-level keys by brace depth and over-approximated reads as *any* string literal anywhere in
the head, which is the conservative direction: it can miss a real orphan, but it will not invent
one. That still over-reports, because a nested `json!` opens its own depth-1.

The finding that survived: **`view.warnings` emits `showing`, `warnings` and `armingBlocker`, and
this head reads only `armingBlocker`** (`VehicleMessages.kt`). The macOS head reads the list —
`VehicleWarning.list(raised["warnings"])` at `Fly.swift:146`. So the two heads show an operator
different things from the same view.

It is **not** simply information loss, and checking that before building anything is what stopped a
wasted change. `arming_blocker` covers *more* conditions than `warnings()` does: unhealthy sensors
and not-ready-to-fly produce a blocker and no list item at all. Its GPS string also merges the
detail sentence the list carries separately. The real delta is that `warnings()` can return two
items where the blocker returns the first match, so with both a pre-arm failure and no GPS fix this
head reports the pre-arm failure and, once that is fixed, the GPS one — two trips where macOS shows
both.

One inconsistency inside the core is worth someone's decision rather than my patch, since the file
is shared. `warnings()` suppresses the pre-arm item when `report_supported`, on the reasoning that a
structured health report supersedes the raw string. `arming_blocker` returns `prearm_error` whenever
it is non-empty, with no such guard. The test is even named
`a_prearm_error_shows_only_without_a_health_report_and_before_arming` and asserts
`warnings(&reported).is_empty()` — but never asserts what the blocker does in that state, so the
intent in the name is not enforced on the branch this head actually renders. Raised with the core
rather than changed here.

`showing` is read by neither head. It is `!listed.is_empty()`, so nothing is lost by that, but it is
core output with no consumer.

### "Ready to fly" was answered by the weaker of two available verdicts

`SetupScreen`'s header answers the one question that page exists for, and it answered it from
`vehicle.autopilotPlugin.setupComplete` — which loops the vehicle components and consults nothing
else. `view.setup` already computes `ready` as *no outstanding components* **and** *no unhealthy
sensors* **and** *a non-empty component list*, with `headline` and `detail` sentences explaining it,
and the screen was already fetching that view for its page lookups.

The two disagree in two states and the head took the optimistic answer in both.
`AutoPilotPlugin::setupComplete` initialises `newSetupComplete = true` and breaks out of a loop that
never executes when `vehicleComponents()` is empty, so **a vehicle that reports no components at all
read as "Ready to fly"**. An unhealthy sensor never entered the verdict either.

Fixed in `a8f450d` by reading `ready`, `headline` and `detail` from the core. That also deleted a
hand-built `"N items need setup"` sentence duplicating the core's `headline`, which had already
drifted from it — the core counts "components", the head said "items".

The rig produced the failing case rather than the agreeing one, which is the part that makes this
verified rather than asserted: the ArduPilot sim reports no setup components, so the header now reads
"Not ready to fly · This vehicle reports no setup components · Nothing to check." where the old code
would have said ready.

A note on how this was nearly missed. The sweep flagged `firmware` as emitted-and-unread and it is —
but the core's `firmware` is `"px4"`/`"apm"`/`"none"`, a discriminator, while the head's
`firmwareSummary` builds a human version string. Same key name, unrelated meanings, no finding.
`ready`/`headline`/`detail` in the same list were real. A sweep hit is a question, not a defect.

### The links screen can now add all three link types

**Built** (`b9645b6`), once the core added `links.createSerialConfiguration`. The section below is
kept as it was written, because the reasoning that led to the wrong fix is more useful than the
conclusion: I argued the bridge needed to vend handles, and the answer was that it needed nothing at
all — only the object needed a path. Connecting reuses `createConnectedLink` with an `@path` exactly
as UDP and TCP already do.

The Serial chip is offered even when no port is present, showing "Nothing is plugged in. Connect a
radio over USB and it will appear here." Hiding the chip cannot distinguish "this build has no
serial" from "nothing is plugged in", and the second is both the common case and the one an operator
can act on. Ports and their friendly names come from `links.serialPorts` and
`links.serialPortStrings`, which `LinkManager` fills in one loop and are therefore index-paired; a
blank or missing label falls back to the device path rather than rendering an empty row.

Verified on the handset as far as the rig allows: the chip appears, the empty state explains itself,
and Add and connect refuses rather than creating a link with no port. **Creating a real serial link
needs a USB radio the rig does not have, so that half is unverified** — it belongs with the other
gates that need hardware.

### How that gap was originally described



`LinkManager::linkTypeStrings()` returns Serial, UDP and TCP, and Serial is not compiled out here —
`build-android/CMakeCache.txt` has `QGC_NO_SERIAL_LINK:BOOL=OFF`. `AddLinkDialog` offers UDP and TCP
as two hardcoded chips. On Android a USB serial radio is a mainstream way to reach an aircraft, so
this is a real gap rather than a tidy-up, and the core has been emitting `linkTypes` and `baudRates`
for a head that never read them.

**It is not laziness in the dialog, it is the shape of the only API that fits.**
`createAndConnectLink(type, name, host, port)` is a single `Q_INVOKABLE` that does everything, and
its parameters are exactly what a UDP or TCP link needs. Serial has no equivalent. Creating one is a
four-step flow — `createConfiguration(type, name)`, write `portName` and `baud` on the result,
`endCreateConfiguration(config)`, `createConnectedLink(config)` — and every step after the first
needs the pointer the first returned.

The bridge cannot carry that pointer, and this is the interesting part. It *does* accept object
references as arguments: an argument beginning with `@` is resolved as a path and passed as a
`QObject *`. But a method **returning** a `QObject *` comes back as `objectJson(object)` — a snapshot
of its property values, with no path attached. And a freshly created configuration has no path to
attach: `createConfiguration` returns a bare `LinkConfiguration::createSettings(...)` that is
registered nowhere until `endCreateConfiguration` calls `addConfiguration`. So the handle exists only
as a C++ pointer, between two calls, in a place no head can name.

What the head *can* already do is read the parts: `serialPorts`, `serialPortStrings` and
`serialBaudRates` are all readable properties. It is only creation that is unreachable.

The fix that matches the existing design is one more one-shot invokable beside `createAndConnectLink`
— taking a name, a port name and a baud — rather than teaching the bridge to hand out handles to
transient objects. That is a shared-API decision affecting both heads and macOS has the same gap, so
it is raised rather than taken here.

Method note, since I got a blocker wrong earlier in this same file: this one was traced rather than
inferred. I checked what a `QObject *` return actually serialises to, what an `@` argument requires,
and whether the created object is registered anywhere — instead of stopping at "the obvious method
is not invokable".

### The class of QGC API that no bridge-based head can reach

Serial link creation is one instance of something general, and it is worth stating on its own because
it bounds what the migration can do without changing QGC.

**QML can hold a C++ pointer in a JavaScript variable between calls. A path-based head cannot.**
`LinkSettings.qml` does exactly that: `var editingConfig = _linkManager.startConfigurationEditing(object)`
at line 187, the user edits fields on that object across several interactions, and line 287 commits it
with `endConfigurationEditing(originalConfig, editingConfig)`. The pointer lives in the QML engine for
the duration. A head that addresses everything by path has nowhere to put it — the bridge returns a
`QObject *` as `objectJson`, a snapshot of values, and accepts `@path` arguments only for objects that
are reachable by path.

So the rule I first wrote was: *any QGC API that hands out a transient object and expects it back is
QML-only.* **That is true but points at the wrong fix, and the core session found the right one**
(`547d00431`). The bridge's limitation is narrower than "cannot hold handles": it is that it cannot
name an object that has no path *yet*. Registration is what converts the second case into the first.

`links.createSerialConfiguration(name, portName, baud)` registers a configuration, and from that
moment everything else is an ordinary property write by path — parity, flow control, data bits, stop
bits, `usbDirect` — followed by `createConnectedLink("@links.linkConfigurations.N")`, which already
worked. Only the registering call was missing. The macOS session's objection that a name/port/baud
one-shot cannot express parity is answered not by a wider signature but by not needing one.

So the question to ask at the next instance of this shape is **"is there a call that registers this
thing?"** rather than "can the bridge vend handles" — and if there is not, adding one is a much
smaller change than teaching the bridge about lifetimes.

Swept the tree for the shape. Twenty-two `Q_INVOKABLE` declarations return a pointer, and almost all
are harmless because they return something already addressable — `getFact`, `getParameter`,
`getVehicleById`, `QmlObjectListModel::get`, `findKnownVehicleComponent`, and every
`MissionController::insert*`, which lands in `visualItems` and is then `visualItems.N`. The head reads
the result back by path and never needs the pointer.

The genuine cases are both on `LinkManager`, and they are create and edit:

- `createConfiguration` returns a bare `LinkConfiguration::createSettings(...)`, registered nowhere
  until `endCreateConfiguration` calls `addConfiguration`.
- `startConfigurationEditing` returns `LinkConfiguration::duplicateSettings(config)`, a detached copy
  that `endConfigurationEditing` copies back and destroys.

So **adding a serial link is genuinely blocked** — the object exists at no path until
`endCreateConfiguration` calls `addConfiguration`.

**Editing is not, and collapsing the two was my error.** The macOS session made the distinction: a
registered configuration *is* addressable at `links.linkConfigurations.N`, and the fields an operator
changes are `Q_PROPERTY` with `WRITE` setters — `name` on `LinkConfiguration`, `localPort` on
`UDPConfiguration`, and `baud`, `portName`, `dataBits`, `stopBits`, `parity`, `flowControl` on
`SerialConfiguration`. A head can write those by path with no bridge change at all. What
`startConfigurationEditing` buys, and the reason it hands back a duplicate, is **cancel**: QML edits
the copy and discards it if the user backs out. A head either commits on change or snapshots the
values first and writes them back. That is a UX decision, not a capability wall.

Neither of us has tested the in-place write, deliberately: it would mutate the link configuration in
a settings file this checkout shares with the QML app. So it is read off the property declarations
and the bridge's `@path` handling, not off a write either of us performed.

Either way the core has been emitting an `editing` flag per link for a UI neither head has built.

The boundary is visible in what already works. `LinksScreen` connects with
`Qgc.invoke("links.createConnectedLink", "@$LINKS_PATH.${row.index}")` — an `@path` reference to a
configuration that *is* registered. Connect, Disconnect and Remove all work for that reason. Add
works for UDP and TCP because `createAndConnectLink` is a one-shot that needs no handle. The three
that fail are precisely the three that would need one.

### A dead detector looked exactly like an empty field of view

`visibleBoxes` drops every box once the detections feed goes stale, and
`detectionTrouble` only spoke when the core reported an `error`. So a detector that was configured
and simply stopped delivering frames drew nothing and said nothing — which is precisely what a
healthy detector sees when there is nothing in view. For an operator watching for something, those
are opposite readings and the silent one is the dangerous half.

Everything needed was already in `view.detections`. `available` means a source is configured,
`stale` means no frame within `STALE_MS`, and the core withholds `boxes` and `track` itself rather
than trusting the head to check — which is why unread `ageMs` could never have caused stale boxes to
be drawn. Only the message was missing. Fixed in `17fce1e`; a reported error still wins over the
absence of frames, and an unconfigured detector stays silent.

**Not verified on the handset, and the reason is worth recording rather than hiding.** Producing this
state needs a detector host that accepts the HTTP connection and then sends nothing; a host that
refuses sets `error` and takes the other branch. The rig has no such host. What makes that acceptable
here, where it was not for the Setup verdict, is that the render path did not change — `trouble` was
already being displayed for errors — so the change is confined to a pure predicate. It is covered by
tests including one that restores the previous implementation exactly and fails on the single case it
could not report, which is a stronger claim than "the new test passes".

### Link editing, and the standing list of QML-only capabilities

**Built** (`6b5447e`), once the core added `links.commitLinkConfigurations` (`8cd3a642d`). Property
writes stay in memory and the commit is the transaction boundary, so a head that writes three fields
and fails on the fourth simply does not commit and the half-applied change is gone at restart. I had
argued for a wider `editLinkConfiguration(fields...)` instead; the core session was right that a
signature covering serial's parity, flow control, data bits and stop bits reopens the wide-signature
problem that create had just closed.

Cancel needs no duplicate here. QML edits a copy because its form binds to the object; a Compose
dialog holds its own state and writes nothing until Save, so backing out costs nothing.

Verified on the handset against a throwaway link, the device having none saved: Edit is disabled
while the link is connected and enabled after disconnecting, and **a saved change survived a
force-stop and relaunch** — the thing that was impossible before the commit call. The link was
removed afterwards.

That closes the standing list of capabilities that existed only in QML and would have gone silently
with it:

1. **Message rate setting** — built and verified on the wire. The QML path was itself broken by the
   `int32_t` invoke bug, so this one belonged in "check whether it ever worked" rather than "would be
   lost".
2. **Link creation, including serial** — built; creating a *real* serial link still needs a USB radio
   the rig does not have.
3. **Cancel-safe link editing** — built.

### How that gap was originally described

Everything needed to *edit* a link is already addressable, exactly as the macOS session argued: a
registered configuration sits at `links.linkConfigurations.N`, and the fields are writable
`Q_PROPERTY` — `name` on `LinkConfiguration`, `localPort` on `UDPConfiguration`, `host` and `port` on
`TCPConfiguration`, `portName` and `baud` on `SerialConfiguration`. The core even tells a head which
form to draw: `editing` is `hostAndPort`, `portOnly`, `serial`, `logFile` or `none` per link, and the
view already carries the current values and a `path`. Cancel is free if the editor holds its own state
and writes only on Save, so the duplicate QML needs is not needed here.

**But nothing a head can call writes it to disk.** `LinkManager::saveLinkConfigurationList()` is plain
`public:` at `LinkManager.h:95`, so `invokePath` cannot see it. Its five callers are
`endConfigurationEditing`, `endCreateConfiguration`, `removeConfiguration`, `createAndConnectLink` and
`createSerialConfiguration` — every one either needs a handle or does something other than edit. It is
not wired to any signal either: no `connect(...)` reaches it, and it writes `QSettings` directly. And
`shutdown()`, which is invokable, does not call it.

So a head can change a link and the change holds until the app restarts, then silently reverts. That
is worse than not having the feature, and it is the exact failure this review keeps finding —
something that looks like it worked. **Not built for that reason.**

A workaround exists and was rejected. Remove-then-recreate persists, because both halves save. But
creating first and removing second — the ordering that cannot lose the link if a step fails — collides
on the name whenever the user did not rename, which is the common edit. The ordering that avoids the
collision destroys the original before the replacement exists. Convoluted and destructive, against a
one-word fix in shared code, on the user's link configuration.

The question for the core is which shape: making `saveLinkConfigurationList` invokable, or an
`editLinkConfiguration` that writes and saves as one call. The first is smaller; the second cannot
leave a head having written half its fields.

### Upload asked whether a vehicle was there, not whether the plan should go to it

Found by sweeping for the pattern that had already caught this head three times: the core computes a
*readiness* predicate and the head gates on a *capability* one. `blocked` against `ready` on the
guided actions, Qt's `setupComplete` against the core's `ready` on the Setup verdict, and `hasModes`
against `canChangeMode` on the camera. The sweep looked for readiness-shaped keys the head never
reads — `can*`, `ready`, `is*Valid` — and `view.plan` had two: `canSend` and `canProceed`.

Upload asked `syncRefusal(vehicleSyncState(offline, syncing))` — is a vehicle connected, is a sync
already running — and then sent. QGC's own `MissionController::sendToVehiclePreCheck` distinguishes
four states and `PlanView.qml` acts on all of them, so two checks in the page being replaced were
missing here:

- **A plan built for a different firmware or vehicle type uploaded with no warning.** QML says it
  "can lead to errors or incorrect behavior" and makes the operator confirm.
- **A vehicle flying this mission was overwritten without being paused.** QML calls `pauseVehicle()`
  before sending, behind a dialog that says why.

The other two hits were run down and are **not** defects, which is worth recording so nobody spends
the hour again. `fences.rs`'s `canRemoveVertex` is reimplemented in the head as `cornerRemovable`
with a constant 3; the core's minimum is `minVertexCount`, defaulting to 3 for a ring and 2 for a
polyline, so the constant would be wrong for a polyline — but the head decodes fence polygons from
the Qt model and uses the rule only for fences, where 3 is right. `hub.rs`'s `canBeSet` belongs to
the Rust hub's own state machine, which is empty unless the core owns the link; the head correctly
uses the Qt-derived `view.flightModes.canSet`, which it does read.

The sweep is now `aircast-android/tools/readinesskeys.py`, with both known-good hits explained in its
output and a selftest that fails if `canSend` ever stops being read — so removing the upload gate
breaks a test rather than going quiet.

The second is the serious one, and neither is a gap in the core — `view.plan`'s `upload` block already
carries `canSend`, `canProceed`, `pausesFirst` and the words to show (`refusal`, `heading`,
`proceedTitle`). The head decides nothing about vehicle safety now; it renders what the core decided
(`491a796`).

Verified only on the path the rig can produce: a clean plan uploads in one tap with no dialog, and the
status chip moves from "Unsaved plan" to "New plan" — `dirty` clearing on a successful send. **The two
warning branches are not producible here** — one needs a plan built for another vehicle type, the
other a vehicle actually flying a mission — so they rest on tests written against the core's own
shapes, and belong with the gates that need hardware.

### How much of this head still reads Qt directly

A number worth tracking, since the migration's point is that heads read the core rather than Qt.
Counting reads through `Qgc.get`/`qgcBool`/`qgcPath`/`mapBool` and friends against raw roots, and
excluding `invoke` and `set` because commands legitimately go to Qt — the core does not own actions:

    54 state reads of raw Qt paths, across 33 distinct paths
    26 actions, across 24 paths
    21 distinct view.* paths consumed

Most of the 54 are single scalars — `vehicle.latitude`, `vehicle.armed`, `vehicles.activeVehicleAvailable`
— where a projection would add nothing. The ones that matter are where the head reads several raw
values and **combines** them, because that is a rule two heads would each have to write.

`SetupScreen` was the clearest: nine raw vehicle paths feeding `firmwareSummary` and
`setupBlockedReason`, the latter reproducing `SetupPage.qml` exactly including the rover clause that
looks like a local addition and is not. The note said it was for whoever moved it into the core.

**That happened on 2026-09-11.** `view.setup` now carries `openable` and `blockedReason` per
component, so the screen's `armed`, `flying` and `rover` watches and its per-component reads of
`vehicle.autopilotPlugin` are gone (aircast-android `59e323d`). The core session encoding it found
the part a second head would get wrong: when a component forbids setup both while armed and while
flying and both hold, the reason named must be *armed*, because telling an operator to land when
disarming is what is wanted is worse than saying nothing.

The same pass took the plan's last bulk read. `PlanBridge.rawItems()`, `FenceBridge`'s three
`getFields(path, "*")` calls, `VehiclePicker` and the import rules all read views now. What is left
reading Qt directly falls into three groups, each with a reason rather than a backlog entry:

- **Facts, not values.** `ParametersScreen` and the extra-video-source editor read a `Fact` — units,
  range, enum strings — to render an editor. The views serve resolved values, which is the wrong
  shape for a control that has to offer choices.
- **Firmware-dependent lists.** `patternNames()` reads `complexMissionItemNames`, which is what the
  connected firmware offers. `view.missionKinds` looks like the replacement and is a static catalogue
  of seven; adopting it would silently drop import targets a vehicle actually supports.
- **Deliberate on-demand reads.** A survey's grid angle and altitude are read when the control is
  tapped rather than on every poll, which is why they are not in a view.

The counts above are left as last measured rather than refreshed, because the sweep that produced
them did not record its method and a re-count with a different one is not comparable. Anyone
refreshing them should record the matcher alongside the number — three sweeps this session produced
false positives from path constants and path-building helpers that hide a `view.` prefix.

### The Plan-entry stall did not get worse

`view.plan` became a watched path on the Plan screen when Upload started consulting the core. Measured
after: 962-968 ms, against the 927-971 ms recorded before. No change, and the new path never appears
as a blocked call at all, because a watched path does not block its reader the way a `get` does.

### When a stale read is a defect, and when it is not

Three defects in one afternoon, all in code written that same afternoon, all the same shape: an
action decided from a **watched** value, which is polled and therefore up to a poll old. Worth
writing the rule down, because the obvious response — re-read everything at the moment of the tap —
is wrong and would add latency to the buttons that can least afford it.

**A stale read is a defect only when the stale answer permits something, and nothing downstream
catches it.** Both halves matter:

- **Upload** was a defect. `canSend` from a poll could still read true after the vehicle had begun
  flying the mission, and nothing verifies an upload against the vehicle's mission state afterwards —
  the plan simply goes up, over a mission in flight. Fixed by reading `view.plan` at the tap.
- **Link editing** was a defect. The row said disconnected, the link connected, and the write landed
  on a live configuration with nothing to notice. Fixed by re-reading the row at the save.
- **Guided flight commands are not**, and this is the useful case. `attemptCommand` waits for the
  vehicle to actually reach the commanded state and reports `"Arm was not confirmed by the
  aircraft."` when it does not. The autopilot runs its own arming checks, so a stale permit produces
  a refused command that the head detects and reports, not a silent wrong action.
- **Adding a serial link is not.** The port list and the taken names are both watched, but
  `createSerialConfiguration` refuses a duplicate name or an absent port itself. A stale read there
  produces a refusal.

So the question to ask of a watched gate is not "could this be stale" — it always could — but
**"if the stale answer says yes, what happens?"** If the answer is a refusal from the vehicle, the
core, or the C++, leave it alone.

### Continue Mission can never be offered, because the bridge's controller is a Plan one

Chased from an incidental observation while probing the flags: `plan.missionController.missionItemCount`
read back **0** on a plan that visibly had items. It is not a bug in the read. The header says so —
`///< True mission item command count (only valid in Fly View)` — and the bridge creates its own
`PlanMasterController` with `setFlyView(false)`.

`currentMissionIndex` is the same, and says it in code rather than a comment:

    int MissionController::currentMissionIndex(void) const
    {
        if (!_flyView) {
            return -1;
        }

`guided.rs` reads both. `has_more_mission()` is `current_mission_index < mission_item_count - 1`,
which on this controller is `-1 < -1` — **false, always**. So `Action::ContinueMission` is never
offered, and an operator who pauses a mission in flight cannot resume it from this head.

Not reproducible on this rig, which cannot fly, and the code is unambiguous enough that reproducing it
would only confirm what both halves already state.

Swept for the general case rather than assuming one: **`missionItemCount` is the only property in the
tree carrying that comment**, so this is two specific properties, not a category. QML does not hit it
because it reads the *fly view* controller — `globals.planMasterControllerFlyView` — while the bridge
has only the Plan one.

The obvious alternative is not reachable either: `MissionManager::currentIndex()` and
`Vehicle::missionManager()` are both plain getters, not `Q_PROPERTY`, so reflection cannot traverse to
them. A fix needs the bridge to own a fly-view controller as well, or those two exposed. Raised rather
than chosen.

**Chosen, and it was the first of the two** (`a4cc60614`). The bridge now keeps a second controller,
`planFly`, which is the arrangement QML already has rather than a new idea, and `guided.rs` reads
`missionItemCount` and `currentMissionIndex` from it. Measured on a real upload to a MockLink, with
both halves asserted in one test: after a two-item mission `planFly.missionController` counts it and
knows where the vehicle is, while `plan.missionController` answers 0 and -1 beside it. The second half
is what makes the first mean anything.

So `ContinueMission` can be offered. This head already routes it — and it routes it to
`vehicle.startMission`, which reads wrong until you check: QGC does the same thing,
`case actionStartMission: case actionContinueMission: _activeVehicle.startMission()`
(`GuidedActionsController.qml:623`). The distinction is which action the operator is offered and what
it is called, not which command goes to the vehicle. Still unverified in flight, because this rig
cannot fly.

### Adding an item is now one call, and the head no longer spells QGC's names

`mission.insert` selects the insertion point the way a click on the plan view does, asks, then refuses
or inserts, seeds the shape — an area for a survey, a launch position for a takeoff — and removes the
item again if the shape will not take. The read and the write are in the same call, so a head cannot
race a stale view past it.

Adopting it deleted 80 net lines from this head (`f88ee55`): its own gate, `appendTakeoff` with its
rollback, `appendLanding`, `insertSurvey` with its polygon corners, and three helpers left orphaned
behind them. The head sends `"survey"` and the core supplies `"Survey"` and `insertComplexMissionItem`,
so a rename in QGC is no longer a string in a head.

It was declined once, correctly: at that point the action inserted a survey with no polygon and a
takeoff with no launch coordinate, so adopting it would have been a regression rather than a
simplification. The seeding is what made it an improvement.

### One bulk read feeds six consumers, which changes what adopting a view costs

`view.missionItems` was offered as replacing a bridge call per item with one call for the list. This
head never had that shape. `PlanMapContent.refresh()` makes **one** read —
`getFields("plan.missionController.visualItems", "*")` — and derives six things from it: the mission
items, the item count, the plan's shape names, whether the route links to home, the surveys, and the
terrain profile.

`view.missionItems` covers the first four. Surveys need polygon vertices and the profile needs
`flightPathSegments`, neither of which it carries, so adopting it leaves `"*"` in place and the head
makes **two** reads where it made one — holding two descriptions of the same plan, which is the
two-sources problem that made the first `mission.insert` adoption a regression.

So the unit of adoption here is not a view, it is the **read**: `missionItems` plus whatever covers
surveys and the profile, landing together, after which `"*"` goes. Taking the first four and leaving
a TODO is the one option worth refusing — two plans that disagree show up as a line drawn across a
map rather than as an error.

Recorded rather than acted on, because the decision is about the shape and belongs with whoever owns
the views. Two smaller gaps found while mapping it, in case the answer is yes: `exitCoordinate`
landed (`3d0ad0fa6`), and `routeEndsAfter` still needs `isLandCommand` or an `endsRoute` composed the
way `flownLeg` is — `kind` cannot stand in, because RTL specifies no coordinate and reads as
`"command"` while Land specifies one and reads as `"waypoint"`.

### The Add Item gate is constant, because it rests on the three flags

`view.missionKinds` gained `enabled` and `disabledReason` per kind, with refusals written for an
operator to read — "The mission already takes off before this point." I wired the Add Item row to it,
tested it on the handset, and **reverted it**.

`insertable()` in `missionkinds.rs` reads `onlyInsertTakeoffValid`, `isInsertTakeoffValid`,
`isInsertLandValid` and `flyThroughCommandsAllowed`. The first three are the flags recorded earlier in
this document: assigned only inside `MissionController::setCurrentPlanViewSeqNum`, and only when the
seq changes. A head that never drives PlanView never calls it, so they hold member defaults forever —
`_onlyInsertTakeoffValid = true`, `_isInsertLandValid = false`.

Measured on the device with three items in the plan, one already a takeoff:

    Takeoff   added a second one       isInsertTakeoffValid defaults true  -> always allowed
    Land      refused, plan unchanged  isInsertLandValid defaults false    -> always refused
    Survey    refused, plan unchanged  onlyInsertTakeoffValid defaults true -> always refused

So the gate is **exactly inverted**: it would have made Land and Survey permanently unavailable while
leaving the one case it exists to prevent — a second takeoff — wide open.

**The whole table is obsolete, re-measured on the handset 2026-09-11 evening.** The core's `point_at()`
calls `setCurrentPlanViewSeqNum` on every insert, so the three flags are driven now rather than holding
their member defaults. Same device, same sim, one plan built up from empty:

    Takeoff, one already present, item 2 selected   REFUSED, "The mission already takes off
                                                    before this point." - the case the gate exists for
    Land, last item selected                        inserted; plan reads "4 items (takeoff, RTL)"
    Survey, middle item selected                    inserted with a real area; "5 items (takeoff, RTL)
                                                    · 52 scan pts · 5.28 km"

So all three are now correct. The reverted `view.missionKinds` wiring rests on exactly these flags, and
its original reason for being reverted is gone — but **it should stay reverted, for a different reason**.

The view is well built: `enabled` is null when nothing is selected, so a head can tell "no answer" from
"refused", and `atSequence` lets a head notice an answer that is about some other selection. The
problem is what a head would do with it. Greying the Land button tells an operator it is unavailable
and cannot tell them why — there is no room beside a row of small text buttons on a phone for a
sentence per button. Leaving it live costs one tap and produces "A landing goes after the takeoff and
after every place the vehicle flies through.", which is the sentence the gate exists to convey.

A refusal that explains beats a control that is merely absent, so the tap is the better interface here
and the gate would make it worse. Recorded as a decision rather than left as an open invitation; the
view's `enabled` remains the right thing for a head with room to show the reason, which this one does
not have.

Worth noting in passing, because it confirms something the core session reported: asking for a "land"
produced an **RTL**, which is what ArduCopter maps it to. What "land" gives you depends on the vehicle. The ungated buttons are
worse in theory and better in practice, so they stay until the input varies. Raised with the core;
`flyThroughCommandsAllowed` is fine, being genuinely computed.

**Two things the tests could not have told me.** The unit tests passed, because they feed the decoder
a constructed view and so prove the decode while proving nothing about whether the view says anything
true. And the first tap *looked* correct — a takeoff went in, which is what success looks like. Only
the item count going 2 to 3 showed it was the second one. A green suite and a plausible screen, and
the count was the only witness.

### The plan was reading the vehicle's translated command names

I reported this defect in the core's `endsRoute` this morning and did not look for it in my own code.
Two places in map-spike matched on a mission item's command *name*. That name is the core's
`commandName`, which is `friendlyName`, which is in QGC's `translateKeys` — so it is translated.

`waypointColour` matched `contains("takeoff")`, `contains("land")`, `contains("return")`. In any
locale but English every marker fell through to the default colour and takeoff, land, RTL and loiter
lost their distinct markers.

`takeoffMissing` looked for `"TAKEOFF"` and had the worse consequence: a German plan that already
takes off was told it needed one inserted first, and the insert went to the front of a plan that was
already correct. Its tests passed `"NAV_TAKEOFF"` — a string the core never serves; it serves
`"Takeoff"` — so the tests agreed with the code and neither of them agreed with the field.

Both now read `kind` and the command id, which are the same in every locale. Break-checks: blanking
`waypointColour` turns 4 tests red, pointing `takeoffMissing` at a kind that does not exist turns 3.

**One instance of this cannot be fixed in a head.** `Fact.valueIsOffTheEnumList` matches the
`tr("Unknown: %1")` label QGC synthesises for a value that is not in a parameter's enum list, and it
is what keeps a float with suggested values (`ACRO_RP_RATE_TC`) from becoming a dropdown with no way
to type a number. `Fact::enumIndex()` *appends* that synthesised entry to the shared `FactMetaData`
and returns its index, so after the first read the value is genuinely in the list and no data
distinguishes it — and the bridge cannot avoid triggering the append, because `enumOrValueString`,
which is how both heads render a readable value, calls `enumStringValue` calls `enumIndex`. The
macOS head carries the same `hasPrefix` check for the same reason. Fixing it means teaching
`FactMetaData` to remember which entries it synthesised; that is a FactSystem change, not a head one,
and it is not worth it for one screen in one locale. Recorded rather than attempted.

## Phase 6 — Shell · 2 weeks

Cheaper than macOS, because Qt is already off the main thread.

**The video-initialisation blocker is cleared** (`e68d606d9`, `df36a17`). It was recorded here as
"video initialisation must be rebuilt before the host can be deleted", which was true and vague; the
specific coupling was a QML `Timer` in `AndroidHost.qml` calling `initForItem(mainWindow)`, and
`init()` refusing a null `QQuickWindow`.

Tracing what the window was for showed all three uses are dead under native rendering. `_mainWidget`
finds the QtQuick video item, which `_rebindWidgets` never wants once `nativeRendering` is set;
`_initVideoReceiver` uses the window only for the thermal widget; and `scheduleRenderJob` defers
`startVideo()` until the scene graph's render thread is ready, which a native sink does not wait on.
So `init()` now tolerates a null window for a head that renders video itself, `initNative()` is the
entry point, and the head calls it after `setNativeRendering(true)` — an order that is load-bearing,
since a receiver bound before that flag looks for a QtQuick item that does not exist.

**Verified by deleting the QML trigger rather than leaving it redundant**, which makes the test the
same shape as the phase: with nothing in QML initialising video, the handset still creates the
receivers, binds nine native sinks and reaches `startVideo()` at 3.5 s, with no init refusal. Every
edit to `init()` is a null guard, so the desktop path is unchanged.

**What the host still carries — re-read on 2026-09-11, because all of it had changed.** This section
described a 172-line shim with 17 functions and a live defect dropping 140 call sites' messages. None
of that is still true, and leaving it standing would send the next person after a bug that is fixed.

`AndroidHost.qml` is now **9 lines**: a bare `Item { id: mainWindow }` and a comment saying why it
cannot go to zero — Qt 6.8.3 has no public way to start Qt embedded without a `QtQuickView`, since
`QtView`, `QtEmbeddedLoader` and `QtEmbeddedDelegate` are all package private and `QtActivityBase` is
the model where Qt owns the Activity. It defines **no functions at all**.

**The message defect is fixed, and not by adding QML.** `showAppMessage`, `showCriticalVehicleMessage`
and `showVehicleConfig` each call `QGCHostNotices::instance()->post(...)` **before** reaching for the
root object, so the invoke that used to drop the message is now belt-and-braces after delivery has
already happened. `showAppMessage` additionally skips its 200 ms retry queue when `_embeddedHost` is
set, with a comment naming the reason: an embedded host has no QML root and never will, so queueing
would grow a list nothing drains behind a timer that never stops re-arming. That is the bridge
channel this section said navigation and messages needed before the host could go.

So the four `_rootQmlObject()` call sites now read: two messages and one navigation that all post a
notice first and no longer depend on QML, plus `attemptWindowClose`, which is still missing and still
has no caller outside `QGCApplication` — it is desktop window-close.

**That was only half the path, and I recorded it as the whole thing earlier the same day.** The post
reached the head; the head then dropped it. `LaunchedEffect(notices)` in `MainActivity` was keyed on
`host`, and acknowledging a batch is a **write to `host`** — so the effect cancelled itself inside
`Qgc.invoke("host.acknowledgeThrough")`, before the `showSnackbar` loop after it, and the restart
found the batch already claimed. Deterministic, not a race. Navigation survived only because it is
applied before the suspend, which is exactly the headline case: **the app jumped the operator to Setup
and the message beside it was cancelled on its way to the screen.** Fixed in aircast-android
`3e118f8` by running the acknowledge and the snackbars in a `rememberCoroutineScope`.

Measured over matched 45 s windows on the handset: before, 18 samples and no snackbar; after, the
parameter-missing warning and `EKF variance` both render. Worth stating how close this came to being
missed — the first probe grepped for a guessed banner string, found nothing, and read as "the channel
is dead"; the second, after the fix, found nothing for the same reason and read as "the fix failed".
The title is `Aircast QGC Daily`. Two opposite wrong conclusions from one bad string.

**Verified on the handset rather than from source.** The rig's `apmvehicle.py` sends a severity-3
`EKF variance` status text every 60 s. The Fly view banner reads **`EKF variance · 216 messages from
the vehicle`** — worst severity first, count behind it. The operator gets the message. Note that
logcat is not the instrument here: of the sim's four status texts only the severity-4 one appeared,
as a `QtTextToSpeech` line, and the severity-3 one logged nothing at all on any tag. Reading silence
in logcat as "the message was dropped" would have reproduced exactly the wrong conclusion this
section originally recorded.

What remains before the host can be deleted is therefore not navigation or messages. It is the Qt
embedding API itself, which is the one thing here that no amount of head-side work removes.

**And the shim had not only been a shim.** It instantiated the whole QML `FlyView` and `PlanView`,
filling the parent, with `planView.map: flyView.mapControl` — invisible, because the head sets
`renderViews: false`, but constructed and bound. That was what pulled QtQuick, QtLocation, QtCharts,
QtMultimedia and QtPositioning into the AAR, and it was the substance behind "expect the 82 MB AAR to
roughly halve".

**Both are now gone** (`86359dffe`). Nothing native needed them: the bridge owns its own
`PlanMasterController`, created — as the comment at `QGCBridgeCore.cc:67` says — because native
frontends have no QML view to own one, so `plan.*` never went through `flyView.planController`. The
three `globals` properties that reached into `flyView` are read only by QGC QML loaded through
`toolLoader`, and `toolLoader` is never active here, because `showTool` is only ever called from
`MainWindow.qml` while this host's navigation functions emit `navigateRequest` instead.

Verified on the handset: the app starts, all five tabs render, video still binds its nine native
sinks, and there is no QML error or crash after touring every tab.

**The AAR did not shrink** — 81.2 MB before and after. Removing the usage does not unlink QtQuick,
QtLocation, QtCharts, QtMultimedia and QtPositioning; that needs the Android dependency set edited,
and this change is what makes that possible rather than what does it. Worth stating plainly, because
"expect the AAR to roughly halve" is the sort of claim that gets read as already banked.

**It is not, however, the Plan-entry stall.** The head sets `page` on every tab change, which drives
`flyViewActive`, which drives `planView.planActive` — so switching to Plan activates a second,
invisible plan view, which is a good story for the ~930 ms. Tested by not setting `page` at all, so
`planActive` never became true: **942-961 ms, against a 927-971 ms baseline.** Unchanged. That is a
fourth mechanism ruled out, after item count, fact serialisation, the video restart loop and generic
marshalling. The stall remains unexplained and is now bounded away from the QML views as well.

- Delete the `QtQuickView` host and `AndroidHost.qml`.

  **Attempted and reverted, which found the thing the survey missed.** With navigation and messages
  both on `host.notices`, video initialising without a scene graph, and the QML views gone, nothing
  *live* reached `mainWindow` any more — so the host looked ready to delete. Removing it crashed the
  app on launch:

      java.lang.UnsatisfiedLinkError: No implementation found for
      void org.mavlink.qgroundcontrol.QGCBridge.notifyFontScale(float)
      - is the library loaded, e.g. System.loadLibrary?

  `QtQuickView(this, QML_URI, QML_LIBRARY, ...)` is what **loads `libAircastQGC` and starts Qt**. The
  host is the boot path, not just a view, and every survey of what *reaches into* it was looking the
  wrong way down the dependency: nothing needed the QML, but everything needed the loader that
  happened to come with it.

  So this bullet is really the one below it — the embedded-host boot path has to exist before the
  host can go, not after. Reverted; the app is back to normal and verified.

  **And that boot path cannot be written, which changes this phase.** Qt 6.8.3 exposes exactly two
  shapes on Android: Qt owns the Activity (`QtActivityBase`), or you embed a **`QtQuickView`**, which
  is the only *public* embedded entry point. `QtView`, `QtEmbeddedLoader`, `QtEmbeddedDelegate` and
  `QtEmbeddedViewInterface` are all package private, so a head outside `org.qtproject.qt.android`
  cannot reach them. There is no supported way to start Qt embedded without QtQuick.

  **So "delete the QtQuickView host" and "QtQuick drops from the dependency set" cannot both happen**,
  and the plan asks for both. The achievable shape is the one now in the tree: `AndroidHost.qml` is
  **9 lines** — `import QtQuick; Item {}` — kept as the smallest thing the view can load, with the
  reason written in the file. QtQuick, QtQml and QtGui stay; everything they do not need can still go.
  Verified on the handset: all five tabs render, video binds its nine native sinks, no QML errors, no
  crashes.

  One fix from the attempt was kept, because it is correct independently: an embedded host no longer
  queues app messages for a QML root that will never appear (`QGCApplication.cc`). That branch is
  unreachable while `AndroidHost.qml` is loaded and becomes live the moment it is not, which is a
  200ms timer re-arming forever on a handset.
- `QGCApplication` drops from `QApplication` to `QCoreApplication`; the embedded-host boot path
  becomes the only path.
- QtQuick, QtQml, QtGui, QtLocation, QtMultimedia, QtCharts, QtWidgets and QtPositioning drop from
  the Android dependency set.

  **Measured, because "roughly halve" reads as banked and is only half the story.** The AAR is 81.2 MB
  compressed and 178.9 MB of native code across 143 `.so`:

      QGC's own libAircastQGC      79.8 MB   44.6%
      Qt modules + QML plugins     71.4 MB   39.9%
      media and crypto (gst, av)   20.4 MB   11.4%
      other                         7.2 MB    4.0%

  So dropping **every** Qt UI module removes 40%, not 50%. Halving needs the other half to come from
  `libAircastQGC` itself, by excluding QGC's own QML and UI C++ from the Android build — which is
  also what the module drop requires, since `Qt6::Charts` reaches `QGCApplication.h` and the Analyze
  chart controllers, and the list in `src/CMakeLists.txt` is unconditional and shared with the
  desktop build. It is two pieces of work, not a CMake edit.

  Individual subsystems, for sequencing: QuickControls styles 10.4 MB across 27 plugins, Widgets
  6.7 MB, Quick3D 3.9 MB, Location and Positioning 2.9 MB, Charts 2.8 MB, Multimedia 1.4 MB.

  **Two cuts landed and the lever is not where this section says it is** (`8b398139a`, `edf56face`).
  Excluding the QML that Android cannot load — MainWindow, FlightDisplay, FlightMap, PlanView,
  FirstRunPromptDialogs, and the ~200 files in `QGroundControl.Controls` — saves **0.33 MB**. Around
  215 QML files, because .qml sources are small next to compiled C++ and the modules stay for their
  `QML_ELEMENT` registrations.

  **The absolute figures in this section were measured on incremental builds and are not comparable.**
  Deleting `AircastQGC_autogen` and rebuilding puts the same tree at 81.76 MB where an incremental
  build reported 80.84 MB, so every number here that was not taken from a clean regeneration is
  roughly a megabyte low, this section's own 81.16 MB baseline included. Re-measured A/B with a full
  regeneration on both sides: **82.09 MB without the exclusions, 81.76 MB with them.** The delta held;
  the absolutes did not. Quote deltas from this section, not levels.

  **And dropping a Qt module from the link list does not undeploy it.** Tested directly, since the
  plan reads as though the two are the same thing: `Qt6::QuickControls2` was removed from the Android
  link and `QQuickStyle::setStyle` compiled out, which links clean — and the AAR went 80.83 -> 80.84 MB
  with **13 QuickControls style plugins still shipped**. androiddeployqt resolves what to package from
  Qt's own dependency graph, not from this target's link list, exactly as `-DQGC_VIEWER3D=OFF` left all
  four Quick3D libraries in place. The experiment was reverted rather than kept, because two `#ifdef`s
  in shared C++ for a measured 0.00 MB is complexity with nothing behind it.

  **Measured from the built artifact rather than carried forward:**

      libAircastQGC              79.3 MB   44%     <- one file
      Qt modules and plugins     67.6 MB   38%
      media and crypto           20.7 MB   11%
      other                      10.8 MB    6%

  The largest single item in the AAR is QGC's own library, and the largest Qt items are ones this head
  never uses: Gui 7.5, Quick 6.8, **Widgets 6.7**, Qml 5.3, **ShaderTools 3.8**, FluentWinUI3 style 2.6,
  QuickTemplates2 2.3, Imagine style 2.2, **Quick3D 2.1**, **Charts 2.0**. `libQt6Test` is in the release
  AAR too.

  So the sequencing in this section is the wrong way round. Excluding QGC's **UI C++** from the Android
  build is the lever for the 44%, and the Qt 38% needs the deployment scan changed — an explicit
  `QT_ANDROID_DEPLOYMENT_DEPENDENCIES` or equivalent — not a link-list edit. The QML exclusions above are
  a prerequisite for the first and worth keeping for build time, but they are not the saving.

  **And the scan cannot be narrowed either, which was the obvious next lever and is now ruled out.**
  `androiddeployqt` runs `qmlimportscanner` over `qml-root-path`, and the generated settings show that
  list as the **project root** plus every directory that still declares a QML module — so it reads
  `.qml` off disk and deploys their imports whether or not those files are compiled. That is the real
  reason both cuts above moved nothing.

  `QT_QML_ROOT_PATH` was pointed at a generated directory holding only `AndroidHost.qml`. It took
  effect — the minimal root is first in the list — and the AAR did not move, because Qt **prepends**
  to that list rather than replacing it: the source root and all eleven module directories were still
  there. Reverted.

  So three mechanisms are now measured and none of them is the lever: the link list does not drive
  deployment, feature switches do not (`-DQGC_VIEWER3D=OFF`, 0.08 MB), and the scan root cannot be
  narrowed. What remains for the Qt 38% is an explicit `QT_ANDROID_DEPLOYMENT_DEPENDENCIES` listing
  every library and plugin to ship, which bypasses the scan entirely — and that is a decision with a
  cost rather than a task: the list is maintained by hand, and getting it wrong fails at runtime on a
  handset rather than at build time.

  **"Expect the 82 MB AAR to roughly halve" should be read as unfunded until that decision is taken.**
  The 44% in `libAircastQGC` is still reachable by ordinary means and is where the next work goes.

  **And the 44% is mostly not code, which closes the last easy route.** Measured from the artifact:
  `libAircastQGC` is 42.7 MB of the 80.8 MB AAR — 53% on its own — and its sections are `.rodata`
  38.4 MB against `.text` 28.3 MB. The five largest symbols in the library are `qt_resource_data`
  blobs totalling ~31 MB. **Half the library is embedded resources, already compressed**, which is why
  the AAR zip cannot squeeze them further.

  Raw payloads behind those blobs: APM parameter metadata 38.5 MB across 45 files, `qgcresources`
  9.9 MB, `qgroundcontrol.qrc` 9.4 MB, `qgcimages` 5.2 MB over 223 images, PX4 1.3 MB.

  **The obvious cut was measured and is a bad trade.** Shipping only ArduPilot 4.4+ metadata on
  Android — dropping 25 of 35 firmware versions, 20.5 MB of source XML — moves the AAR
  **80.84 -> 79.93 MB. 0.91 MB.** In exchange, every vehicle on 4.3 or older loses parameter
  descriptions, enum decoding and bitmask names, which is what makes `ARMING_CHECK` read
  "Barometer, INS, RC Channels" instead of `82`, and what the Params search matches on. Reverted.

  One mechanism note for whoever tries this next: a generator expression in the `QGC_RESOURCES` list
  does not swap a `.qrc`. AUTORCC compiles both and the link takes both, so the first attempt measured
  0.03 MB and looked like the resources were incompressible rather than like a broken swap. An
  `if(ANDROID)` around a `set()` is what actually replaces it.

  **So the size target has no cheap remainder.** Seven mechanisms have now been measured and none of
  them is a build flag:

      link list (Qt6::QuickControls2 removed)      no change, 13 style plugins still shipped
      feature switch (-DQGC_VIEWER3D=OFF)          0.08 MB
      QML scan root (QT_QML_ROOT_PATH)             no change, Qt prepends rather than replaces
      QML module exclusion                         0.33 MB
      resource contents (drop pre-4.4 metadata)    0.91 MB, at the cost of older airframes
      qt_import_plugins EXCLUDE_BY_TYPE qmltooling settings changed, artifact unchanged
      QT_ANDROID_DEPLOYMENT_DEPENDENCIES           settings changed, artifact unchanged
      Gradle jniLibs excludes                      **breaks the app**

  **And the last one is worth knowing about before someone else tries it.** Excluding
  `libplugins_qmltooling_qmldbg_*.so` and `libQt6Test_*.so` from the APK is obviously safe reasoning —
  the debugger plugins load only with `-qmljsdebugger` and nothing links Qt Test — and it is not.
  Qt's loader reads a manifest of the libraries it bundles and loads them **by name**; one missing
  entry aborts initialisation before the JNI natives are registered, and the app dies on launch with

      java.lang.UnsatisfiedLinkError: No implementation found for
      void org.mavlink.qgroundcontrol.QGCBridge.notifyFontScale(float)

  which is the same signature the host-deletion attempt produced and reads as a linking fault rather
  than a packaging one. Reverted; the handset is back to a working build.

  **One reframing that matters more than any of the above: the AAR is not the shipping artifact.**
  Every figure in this section is an AAR, which is an intermediate library. The APK is what ships, and
  a clean `assembleDebug` is **116.6 MB** — measured A/B against itself, because an incremental APK
  read 157.3 MB and would have supported a 41 MB saving that does not exist. Of that, 90 MB is native
  libraries and 45 MB is `libAircastQGC` alone.

  Halving what ships means choosing what the product does without, not finding a build flag.

  **The `Viewer3D` decision is answered, and the answer is that it barely matters.** The project has a
  supported switch for it, so I tried the designed path before considering surgery: configuring
  `build-android` with `-DQGC_VIEWER3D=OFF` and rebuilding took the AAR from 81.16 MB to **81.08 MB**,
  and left all four Quick3D libraries in place. Deployment follows *linkage and the QML import scan*,
  not whether a feature is compiled — `Quick3D` is an `OPTIONAL_COMPONENTS` entry on the top-level
  `find_package(Qt6 ...)`, and androiddeployqt ships what it finds. So dropping Viewer3D saves 0.08 MB
  rather than the 3.9 MB its libraries occupy. The tree was configured back to `ON` afterwards, since
  an undocumented cache divergence would cost the next person more than 80 KB is worth.

  That sharpens where the remaining work is: **the size lives in what the Android build compiles and
  scans, not in which features are switched on.** Excluding QGC's QML modules for Android is the lever
  — it shrinks `libAircastQGC` and removes the imports the deployment scanner follows — and it is a
  change to the unconditional list in `src/CMakeLists.txt` that the desktop build shares.
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
3. ~~**The bridge has no tests.**~~ **It has had 121 since before this plan was written.** This risk
   was stale on the day it was recorded: `ff1327d5d` is titled "Add bridge unit tests and the Android
   migration plan" and added both in the same commit. `test/Bridge/` holds 43 `QGCBridgeCoreTest`
   cases over path resolution, fact reads and writes, unit conversion, accessor calls, projected
   reads, the plan roots and refusal reporting, plus 78 `QGCCoreCTest` cases; both are registered in
   `UnitTestList.cc` and neither appears in 58 recorded suite runs' flake history. Left in place
   rather than deleted, because "fix this before Phase 3" sent me looking for work that was already
   done, and the next person deserves to know it was never outstanding.
4. **Blocking calls from the Android main thread.** `runOnQtThread` uses a `BlockingQueuedConnection`
   when called off the Qt thread. A Qt thread that is itself waiting on the Android main thread would
   deadlock. Not observed, not prevented.

   Measured 2026-09-10: opening the Plan tab makes the Qt thread busy for about a second, and every
   bridge read issued in that window waits on the blocking connection. The same path read twice in
   one tab entry gives **864 ms at entry and 5 ms four seconds later** — so the reads are cheap and
   the wait is queueing, not work. Reads issued back to back during the window drain progressively
   (883, 619, 18, 4 ms). Ruled out by experiment: item count (an empty plan costs the same), full
   versus compact fact serialisation (`getFields(path, "*")` changed nothing), the video receiver's
   1 s restart loop (disabling video left the number unchanged), generic marshalling cost (another
   off-thread call in the same window measured 8 ms) and the `Watcher`, which binds to signals rather
   than re-reading. What occupies the thread for that second is still unidentified. No ANR — the
   calls are off the main thread — but whatever reads first comes back a second late.
5. **Watcher latency.** Watched paths are polled and diffed at 200 ms, not signal-connected. Fine for
   status, too slow for an attitude indicator. Phase 5 needs the notify-signal path.
6. **macOS divergence.** Every path Compose needs must also serve SwiftUI. Add to `QGCBridgeCore`,
   never to a head.

## Not covered

- **iOS, Linux, Windows.** They stay on Qt. This is an Android frontend, not a migration of the
  project.
- **De-Qt of the core.** Stripping QObject ends the upstream merge stream permanently. Don't.

### The leg gained its third fact, 2026-09-12

`altitudeChangeText` had been in the core's mission-item view unread. QGC
documents `altDifference` as "change in altitude from previous waypoint" —
the same from-previous framing as `distance` and `azimuth` — so it belongs on
the leg line under a selected item, not on the item itself. Selected waypoint
now reads `432 m · 56° · +10.0 m`, verified on the OnePlus 6 by raising one
waypoint with the `+10` button and watching that item's own incoming leg
change.

Two gates, not one. A level leg has a real distance and a zero change, and
`+0.0 m` under every waypoint of a flat mission is noise. And the first
attempt gated the whole line on distance, which was wrong in the other
direction: `MissionController::_recalcMissionFlightStatus` skips
`setDistance` when the previous item is a land command, and a waypoint
stacked over its predecessor has no ground distance either — both still carry
an `altDifference`, and suppressing the line threw away the only fact those
legs have. The first item is safe either way; QGC sets all four to zero there
under the comment "No values for first item".

The pattern that found the second bug is the one worth keeping: the feature
was already committed and verified on hardware before I asked what the gate
does when its input is zero for a reason other than the one I had in mind.

### The fallbacks were the last of the unit defects, 2026-09-12

The four unit bugs found earlier by grepping for `" m"` were each fixed by
reading the core's text — but each fix left the head's own version behind as
an `ifBlank` fallback. `format_measure`, `range_text` and `distance_text` are
unconditional and `detailText` is a plain `format!`, so none of them can
return blank: every fallback was unreachable while the core answers, and
reachable only when the head runs against a library older than itself. In
that case it prints a metres label on a feet build — a wrong reading with no
tell.

It is not hypothetical. That is precisely what made the `bandText` hunt take
three rounds: the terrain label looked healthy while the library was stale,
because the fallback caught the blank and said metres. Removing it makes a
skew show as a missing measurement instead of a confident wrong one.

`heightRange` collapsed from five branches to two — flat names one height,
everything else is the band. The fixtures went with the branches: a fixture
leaving `lowestText` and `bandText` empty described a core that does not
exist, which is how those tests came to assert a string the head had
assembled. The same rule as before, now with the deletion to match it — a
test asserting a measurement the head built is a test that the head is
answering a question it should be asking.

Swept the rest of both modules for the same shape. The `ifBlank` calls that
remain supply names and prose — "Camera 2" for an unnamed stream, a sentence
when a refusal carries no reason — and none of them invent a number with a
unit on it.
