# map-spike

A native map for the Android frontend: MapLibre in Compose, driven entirely through
`QGCBridgeCore`, on tiles from QGroundControl's own cache.

Phase 4 of `NATIVE_ANDROID_REWRITE.md`. It began as a spike asking whether a native map could
replace the QML one. It does: `:app` hosts it as the Plan tab. The module name has outlived the
question it was named after, and renaming it would touch `:app`, so it stands for now.

## Where things live

`PlanMapContent.kt` holds `MapSpikeScreen`, the composable both routes render. `PlanMap.kt` holds
`PlanMapScreen`, the entry point the app calls. `MapSpikeActivity.kt` holds only the harness
activity. That split exists because the shared screen used to live in the activity file, and a
reader who greps for a symbol, finds it in `MapSpikeActivity.kt` and concludes it is harness-only is
reasoning correctly from a filename that was lying.

## Hosting it in the app

`:app` already depends on this module. Drop the map into the app's own navigation with

```kotlin
PlanMapScreen(Modifier.weight(1f))
```

It fills the modifier it is given rather than sizing itself, so a host can put it in a `Column` under
its own header. Colours come from the app's `MaterialTheme`; the module wraps none of its own.

It sets itself up on first use and survives being entered again, which matters because MapLibre has
to be initialised before the tile source replaces the HTTP client and that client may only be
replaced once. Only a style backed by QGC's tiles is remembered, so entering the tab before QGC has
opened its cache does not pin the OpenStreetMap fallback for the life of the process.

The map paints no vehicle name, mode or telemetry, on the assumption the host shell already does.
What it does show is what nothing else can: the plan, what it costs, and what is selected. The
activity below is the standalone harness and calls the same composable.

## Running it standalone

The map ships inside the app's Plan tab. The activity is a harness for working on the module
without going through the app's navigation, and both routes render the same composable, so
whatever the harness shows is what the tab shows:

```
adb shell am start -n one.aircast.android/.MainActivity          # boots Qt
adb shell am start -n one.aircast.android/one.aircast.mapspike.MapSpikeActivity
```

`am start -S` force-stops the whole package, so using it on the spike activity kills Qt seconds
after the main app booted it. Use it on `MainActivity` if you want a clean restart, never on the
spike. The symptom is "Bridge not running" and `No implementation found for QGCBridge.get` in
logcat, which reads like a broken bridge rather than a test harness shooting Qt.

**Do not name a cause the bridge cannot see.** A failed read says a read failed. It does not say
why, and the map used to answer it with "start the main app first" — true of the harness, and in
the app's Plan tab an instruction to do the thing already done. It says "Waiting for
QGroundControl", which is the observation. The harness hint lives here instead, where it is true.

**The main app has to start first.** Launching the spike alone never boots Qt, so the bridge
natives are unregistered and every call throws. The header says so when that happens.

Placing anything uses the vehicle's position, falling back to the map centre, so the spike can be
driven with no vehicle connected.

## What works

Mission items, geofence polygons, geofence circles, rally points and surveys can each be created,
moved and deleted. Surveys generate transects and their grid rotates. Waypoint altitude is
editable. A terrain profile plots planned altitude and ground height against distance. All
verified on a device against a live vehicle.

The whole of that in one gesture: dragging a survey's corner handle - the same handle built for
fence polygons - reshapes the area, regenerates the grid through the bridge, redraws the
transects, and moves the profile with it. 5.90 km became 4.16 km in the status line, in the
chart, and in the ground curve underneath it. A second survey carries on from where the first
ended: 10.80 km in the profile against 10.80 km in the status line, with the ground running
under both.

## Not done, and why

Takeoff and RTL are counted but not drawn. Neither has a coordinate of its own - a multirotor
takeoff goes straight up from the launch point and an RTL returns to it - so the panel can say
"3 items" over a map showing one survey. QGC solves this with a mission item list, which is a
bigger thing than a map and is not here. The count is right and the map is right; what is missing
is the third view that reconciles them.

The `flightPathSegments` read that draws the ground is not cached. One survey costs 20 ms per poll
against 0.1 ms without it, which is 3% of a 700 ms poll off the main thread. Nine surveys is where
that stops being free - see **Terrain** for the number and for the trap waiting in any cache keyed
on the plan alone.

Terrain cannot be tested at the default site at all. The tile service has no coverage for Tbilisi
and answers HTTP 500; `fakevehicle.py` takes `SIM_LAT`/`SIM_LON` so the vehicle can be flown
somewhere with tiles.

A fix in `QGCBridgeCore` does not reach the phone until `AircastQGC.aar` is rebuilt. Both Android
modules build against the prebuilt AAR, so "verified on the handset" covers the app and not the C++
under it unless the AAR was rebuilt in between. The vehicle picker sat disabled for hours for
exactly this reason.

## What the bridge taught us

These cost the most time. They are not obvious from the C++ headers.

**Settings hide in the fact list.** A circle's radius, a survey's grid angle and an item's
altitude are not fields on the object. They are Facts in its `facts` array, found by name. Reading
`radius` off the circle returns nothing and the circle looks like it failed to create. `factValue`
in `PlanBridge.kt` is the one lookup for all of them.

**Child objects are listed, not nested.** A survey's area polygon appears only as a name in
`children`. Reading the item gives you no polygon at all; it needs its own read at
`…visualItems.N.surveyAreaPolygon`.

**The bridge owns the plan controller, not the QML view.** `rootObject("plan")` creates a
`PlanMasterController` on first use and calls `start()` on it, precisely so a native frontend does
not need a QML view to own one. So `plan.*` here addresses the bridge's controller, and the QML plan
view's controller is a different object entirely. Nothing about this depends on `PlanView.qml`, and
the Phase 6 host teardown does not take it away.

**A plan item can exist without being on the map.** Inserting a takeoff puts a real item in the
plan, but on a copter it specifies no coordinate of its own, so nothing is drawn and the item
count does not move. Counting drawn markers to decide whether a bridge call worked reports a
working call as a failure. The raw element count is the one that answers that question.

**Mission Start is in the terrain profile too, and it sets the floor.** It carries its own planned
altitude, usually 0, so a survey flown at one height still spans 0 to that height rather than being
flat. A genuinely flat profile needs every item at the same altitude, which is rare, so the flat
case is worth handling but is not the common survey case.

**Mission Start is drawn like a waypoint but is not one.** The settings item takes a coordinate
once the plan has something in it, so it appears as a numbered dot at the planned launch point.
It is coloured apart from the waypoints for that reason.

**A bad index is a segfault, not an exception.** `insertSimpleMissionItem` with an index past the
end walks off the list and takes the process down, tombstone and all. The bridge validates types,
never ranges. The insert index addresses the whole visual item list, which opens with a settings
item and can hold items carrying no coordinate, so counting only what is drawn gives the wrong
number.

**This module is one client of several, and says so.** `QGCBridge` keeps a path set and a listener
per client and watches the union, so this module registers as `"map"` and arming its watches no
longer disarms the app's other screens. It did once: `watch` and `setEventListener` used to be
single-owner, and taking either froze the telemetry on the screen a pilot flies from. Anything
calling the single-argument forms lands in a shared `"default"` bucket, so always pass the client.

**Let the bridge off the hook.** A watch registered before Qt has its natives in place throws. If
the path stays in the watched set nothing ever retries it, and the screen reports a dead bridge at
one that came up a second later. Watches are released when the map goes away too, because the
watcher polls and diffs every path at 200 ms and stops only when told to watch nothing.

**Draw what the values say, not what movement says.** The vehicle marker used to be redrawn only
when the position changed, behind an early return, so a vehicle rotating on the spot never updated
its arrow. The effect is keyed on everything it draws.

**Losing heartbeats is not losing the vehicle, and it catches you twice.** QGC keeps the `Vehicle`
after the link goes quiet, so nothing that keys on the vehicle disappearing fires. That is why
clearing the marker on an unusable position never triggers, and it is also why the plan survives:
`PlanMasterController::_activeVehicleChanged` clears a clean plan with `removeAll()` when the active
vehicle becomes null, but stopping a vehicle does not make it null. Measured: an uploaded plan of
two items and 88 transects was still there eighty seconds after the vehicle stopped sending.

**What that handler does, from source and not reproduced.** In the plan view, on an actual vehicle
change: a dirty plan emits `promptForPlanUsageOnVehicleChange` and waits for a decision, which QML
answers with a dialog and a native frontend answers with nothing; a clean plan is cleared if the
vehicle went away, or replaced by the new vehicle's plan if one took over. So a native Plan tab that
ever sees a real vehicle change will either keep a plan silently associated with a different
vehicle, or lose a clean one. Neither is reachable by stopping a vehicle.

**A lost link does not make the position unusable.** QGC keeps the `Vehicle` and its last known
coordinate after heartbeats stop, so `isPlottable` stays true and the marker keeps drawing a live
looking aircraft at a place it may have left. Clearing on an unusable position is right but never
fires for this; `vehicle.vehicleLinkManager.communicationLost` is what actually answers it, and the
marker goes hollow rather than vanishing, because the last known position is still worth seeing.

**A tab does not own the process, an activity nearly did.** Three things here assumed otherwise and
all three were real bugs once the map became a tab: the poll ran on forever, the MapView was never
destroyed, and the watches were never released. The reverse case is the flown trail, which is
accumulated over time and cannot be re-read from the bridge, so it is the one piece of state that
has to outlive the screen rather than die with it. Anything else added here should be sorted into
one of those two piles deliberately.

**Poll only while something is watching.** Every poll is a blocking trip into the Qt thread. As its
own activity the screen stopped polling when it went away; hosted as a tab it stays composed while
the app is in the background, so the loop is gated on the lifecycle. Measured on the phone: ten
polls in seven seconds in the foreground, none at all backgrounded, ten again on resume.

**A drag writes on release, not per touch.** Writing a move per touch event puts overlapping
blocking calls on the bridge whose completion order is not guaranteed. Intermediate positions are
dropped on an interval and the release always writes the real one, so the final position is exact
whatever the throttle skipped. Verified: a drag lands on its drop point.

**The nearest marker wins, not the first.** `queryRenderedFeatures` returns everything the box
touches in source order, so `firstOrNull` picks by list position. A launch point and the first
waypoint routinely overlap, and grabbing the pair moved whichever was drawn first. Distance decides
now, for the point layers; being inside a fill is not a matter of degree so fills still take the
first.

**`adb input swipe` cannot drive a drag; `input draganddrop` can.** A swipe starts moving
immediately, which a drag handler is right to read as something else, and it produces no drag, no
pan and no selection change at once — three outcomes that look like a broken handler and are the
verb. `input swipe x y x y 900` is a long press.

**Never call the bridge from the main thread.** Calls block on the Qt thread. Touch handlers and
button callbacks all go through a background dispatcher.

**The poll cost is linear in surveys, and the headroom is real but finite.** A poll is four blocking
calls plus one per survey — the whole plan, three fence reads, and `surveyAreaPolygon` each — plus
parsing the plan JSON. Measured on the handset against the 700 ms interval, medians of five:

| surveys | transects | poll |
|---|---|---|
| 0 | 0 | ~5 ms |
| 1 | 88 | ~55 ms |
| 3 | 268 | ~62 ms |
| 6 | 536 | ~94 ms |
| 9 | 804 | ~164 ms |

Roughly 18 ms per survey, so a realistic plan of a few surveys costs well under a fifth of the
interval, and it would take somewhere around thirty five surveys to saturate the poll. Variance is
wide — 53 to 130 ms at six surveys — so a single reading means little and only medians are worth
quoting.

**Read a list once and parse it many times.** Mission items, surveys and the terrain profile all
come out of the same item list. Fetching it once per poll instead of once per consumer halved the
poll, from a median of 66 ms to 32 ms on an 84 point survey. Each call serialises the whole
subtree to JSON and blocks the Qt thread while it does, so the cost is in the trip, not the
parsing.

**An upload can be reported as asked, never as done.** `sendToVehicle` returns void and nothing
reachable through the bridge says the aircraft accepted the plan. `plan.dirty` looks like that
signal and is not: measured against a vehicle that never acknowledges, it went false anyway and the
map said "Uploaded to vehicle". The wording is "Upload sent to vehicle", which is the part actually
known.

**Most of the editing calls cannot report failure at all.** Nine of the ten methods this module
invokes return void — `addInclusionPolygon`, `addInclusionCircle`, `addPoint`, `adjustVertex`,
`appendVertex`, `deletePolygon`, `deleteCircle`, `removePoint`, `removeVisualItem`. For those `ok`
is true whenever the method was found and its arguments converted, so a call that bounds-checks a
stale index and quietly does nothing looks exactly like one that worked, and the panel says so.
Only `insertComplexMissionItem` returns anything, a pointer.

That is survivable here for one reason worth keeping in mind if this code is reused elsewhere: the
map re-reads the plan every poll and draws it, so the screen shows what actually happened within a
few hundred milliseconds even when the message does not. The state being visible is the effect
check. A headless caller of these same paths would have no such luxury and would need to confirm
each one by reading back.

**`ok` means the method ran, not that it did anything.** The bridge reports success when it found
and invoked a `Q_INVOKABLE`, and a method that decides internally to do nothing still returns `ok`.
`sendToVehicle` with no vehicle, or while a sync is already running, logs a warning and returns, so
a map that trusted `ok` would report an upload that never happened. `plan.offline` and
`plan.syncInProgress` are the properties that actually answer it. Any void call worth trusting
needs a property to confirm it landed.

**A selection outlives the plan it was made against.** It names an index, and loading from the
vehicle replaces the plan wholesale, so the altitude and delete controls would have acted on
whatever now sat at that index. Deletes clear it themselves; every other way the plan can change
underneath is caught by validating the selection against the plan each poll.

**Plan files report success in the invoke result.** `saveToFile`, `saveToCurrent` and
`loadFromFile` return bool, so the `result` field of the invoke is the answer in both directions —
read that, not `ok`, which only says the method ran. This changed in qgroundcontrol `25e313267`;
before it they returned void and `plan.currentPlanFile` was the only way to tell a success from a
failure. An AAR built earlier still has the old behaviour. Measured against the AAR that carries it:
a writable save gave `result=true` with the path recorded, an unwritable one `result=false` with it
empty, and a missing file and a non-plan file both gave `result=false`. `ok` was true in all five,
so it still only says the method ran.

`currentPlanFile` is still worth reading after a save, for the path it actually wrote rather than to
find out whether it wrote: a filename with no dot gains `.plan`, so `/…/probe-plan` lands at
`/…/probe-plan.plan`. Anything copying a saved plan back out has to use what it reports rather than
the path it asked for.

A failed load is not destructive. With a survey in the plan, loading a missing file and loading a
file that exists but is not a plan both left the item count untouched, so opening the wrong document
costs nothing but the attempt.

**What the plan file affordances have to do, taken from `PlanView.qml` rather than invented.** Save
and Save As are both enabled on `containsItems && !syncInProgress`. Save reuses the existing file
when there is one — `currentPlanFile != ""` means call `saveToCurrent()`, otherwise it behaves as
Save As and asks for a destination. That branch is the whole reason a one-slot design is avoidable:
`currentPlanFile` is what makes Save mean "again, where I said before". `dirty` says whether there
is anything to save; `containsItems` says whether there is a plan at all, and they are not the same
question. Export KML is a separate action, not a third choice in a Save dialog, because it writes a
format nothing here can reopen.

**A clear cannot live in here.** The host owns the opened document, and `removeAll` does not touch
it — nor QGC's `currentPlanFile` unless the controller is offline. So a clear invoked from inside
the map empties the plan while the shell still names the user's file and keeps Save enabled, and the
next Save writes a blank plan over their mission and reports success. `PlanMapScreen` takes
`onClear` and shows the button only when the host supplies one, because the dependency runs app to
map and nothing in here can reach the document reference.

**Clear is local, where QGC's is not.** `PlanView.qml`'s Clear Mission calls `removeAllFromVehicle`
and wipes the aircraft as well as the controller, behind a modal confirmation and coloured red.
`Clear` here calls `removeAll`, which empties the controller only, because starting a plan over is
the local intent and Upload already exists for anyone who wants the empty plan flown. A deliberate
divergence: it makes the destructive half opt-in rather than the default, and it keeps the action
inside what a two-tap confirm can honestly carry.

**Loading from the vehicle throws work away.** It overwrites whatever is drawn and there is no
undo, so QGC asks first when the plan has unsent changes. `plan.dirty` says when that is, and Load
here asks the same question rather than being the one place that discards a survey silently.

**`clearAllInteractive` does not clear anything stored.** It ends an interactive edit. A button
built on it reports success and changes nothing. `deletePolygon` is the real path.

**Writes can take an object now.** `writePath` resolves an `@path` value to a QObject and
type-checks it against the property, as of qgroundcontrol `78040316a`. That is what makes the
vehicle picker possible: `vehicles.activeVehicle` is a writable `Vehicle*`, and the write names an
entry in the manager's own list rather than a copy of it. The picker only appears with more than one
vehicle connected, because with one there is nothing to choose.

## What MapLibre taught us

**A failing glyph font kills every layer on the source.** Waypoint circles that need no glyphs
never drew because a sibling text layer's font 404'd. The only evidence is in MapLibre's native
log, not in Java. Match the font stack to what the style's glyph endpoint actually serves.

**The circle layer is sized in screen pixels.** A geofence circle has to become a ring on the
ground, or it keeps its size as the map zooms. See `circleRing`.

**Android's edge gestures eat drags.** A drag beginning near the screen edge becomes a back
gesture, and the touch handler never sees the release that re-enables the map's own gestures, so
panning stays dead. Starting a touch on empty map restores them.

**The logo and attribution are not decoration.** MapLibre puts them bottom left, under any control
panel placed there, and showing them is a licence condition. Their margins have to follow the
panel's measured height, which means reading it after layout: read once while the map loads and the
height is still zero.

**Drag by a handle, never by a fill.** A fence wide enough to cover the screen would otherwise
swallow every pan. Circles get a centre handle for this reason.

## Two vehicles

Everything here follows the active vehicle, and with more than one connected that changes on its
own. The header names the vehicle and says how many are connected, because things placed "at the
vehicle" landing at the other one reads exactly like a broken feature. This cost several rounds
before the header existed.

The picker offers whichever vehicles are not active, and tapping one writes an @path to
`vehicles.activeVehicle` - the property takes a `Vehicle*`, so the value names the entry in the
manager's own list rather than a copy of it.

That write was refused for most of its life, and the reason is worth keeping. `resolve` used to
follow the last path segment into the object the property already held, so the write arrived with
an empty property name and died in a branch that returned a bare `{"ok":false}` with no reason at
all. It therefore worked exactly while `activeVehicle` read null - the state where there is nothing
to switch to - and failed the moment a vehicle connected. Fixed on the bridge side; the AAR has to
be rebuilt before a fix there reaches the phone.

Verify it by the read, never by the write. "Vehicle 2 is active" is only `set` returning ok. The
evidence is that the picker then offers Vehicle 1, which comes from `vehicle.id` on the other side.

## Tiles

`QgcTileCache` reads QGroundControl's SQLite cache directly and serves tiles through an OkHttp
interceptor on a private host, so the map works offline on imagery already downloaded. Tiles are
keyed by a 29 character hash: a 10 digit provider hash then x, y and z zero padded to 8, 8 and 3.

QGroundControl writes that database while the map reads it, and a read-only connection cannot roll
back a journal it finds mid-write. The exception lands on OkHttp's thread and takes the process
down, so a failed tile read yields no tile instead.

Falls back to OpenStreetMap when the cache is empty. That style is for development only.

## Seen in the app's Plan tab

Verified in place, not only in the harness: the app's header and status strip sit above, then its
`Open / Save / Save as` row, then this map with its chip, scale bar, controls and profile, then the
nav. No duplicated vehicle readout and no collision between the chip and the row.

Editing here reaches the host. A long press adds a waypoint through the bridge and `plan.dirty`
follows, which the tab shows by changing its plan status from "New plan" to "Unsaved plan" and
enabling Save. That status is a label, not a command — the tab has no New Plan action, and Open is
the only thing there that replaces a plan wholesale.

## Altitude

A selected waypoint gets a typed field as well as the ten metre steps. Steps alone cannot reach a
particular height without a run of taps, and reaching a particular height is the usual reason to
touch an altitude. The field refuses what does not parse and anything below the ground rather than
sending it, and keeps a half-typed number — "4" on the way to "47" is not a reason to throw the edit
away.

**The map does not draw every item a plan can hold.** Corridor scans, structure scans and landing
patterns are not rendered, and a plan opened from a file is where they arrive. They still count in
the distance and still upload. A runtime warning for this was tried and removed: three attempts at
the predicate each fired on ordinary plans, because "complex", "specifies a coordinate" and "was
drawn" do not mean on real serialised elements what they appear to mean. A warning that fires on
every plan is worse than the gap it describes, so this is documentation until someone can state the
condition against real data rather than against an assumption about it.

**Every control on the selection row must follow the selection.** Rotate did not: it acted on
`surveys.first()` whatever was picked, so with two surveys it turned the wrong grid, and it appeared
whenever any survey existed rather than when one was selected. Easy to miss because with a single
survey the two are the same thing.

**A survey's height is not an altitude fact.** It is `cameraCalc.distanceToSurface` — the camera's
distance to the surface, which is what the survey editor labels Altitude. `cameraCalc` is a child
object, so it needs its own read, and that read happens when a survey is selected rather than every
poll, since the poll already costs one call per survey.

## The map centre is a real coordinate before it is a meaningful one

With no vehicle the map opens on MapLibre's default camera: a 10000 km world
view sitting in the Atlantic. placeAt falls back to that centre, and near 0,0
isPlottable refuses it, so every creator answers "did not work". That is correct
and it is reported - the message appears immediately and clears after
FAILURE_MESSAGE_MS, which is 2500, so a screenshot taken later shows a button
that looks ignored.

The dangerous half is one pan away. Move that world view slightly and its centre
becomes a perfectly plottable coordinate off Greenland, so the creators stop
refusing and start placing items there instead - a mission line running from the
plan to the middle of the North Atlantic. isPlottable is doing its job; a
coordinate can be valid and still mean nothing.

So the map now fits the plan once, while there is no vehicle position. QGC's
Plan view does the same. It fires only while there is nothing to follow, so it
never argues with a camera the pilot is flying, and it is what keeps the world
view from being used as an anchor in the first place.

Reproducing it needs the app's Plan tab, File, Import boundary, site.kml,
Survey - no vehicle, and touch nothing else. Before: "1 item, 56 survey pts,
4.92 km" at 20000 km, a black screen with one dot. After: the same plan at
200 m with its transects and handles.

## ok is not the same as done, again

`insertTakeoffItem` throws away the coordinate it is handed - the parameter is
commented out in `MissionController` - so the item arrives at 0,0 with
`specifiesCoordinate` false and `readyForSaveMessage` "Set its location". The
invoke returns `{"ok":true}` with a whole item in `result`, because the method
was found and did run. Nothing was wrong with the call. The item simply had no
location, so it was not drawn and not counted, and the panel read "Empty plan"
over a plan that had a takeoff in it.

Writing `launchCoordinate` on the returned item is what places it, and it sets
the planned home the takeoff is measured from at the same time. `insertLandItem`
passes its coordinate through and needs no such help.

The bridge does return what an invoke returned - `result` carries the object, or
`{"kind":"null"}` for a null pointer. Reading only `ok` throws that away.

## Terrain

Ground height comes from a tile service, and it does not cover our test site.
`https://terrain-ce.suite.auterion.com/api/v1/carpet?points=41.713,44.830,...`
answers HTTP 500 `{"error":"tile not found"}` for Tbilisi and returns real
carpets for Switzerland and San Francisco. So at 41.71,44.83 every
`amslTerrainHeights` is `[]` and every `terrainAltitude` is NaN, and "ground
height unknown" is the honest answer rather than a bug worth chasing. Nothing
terrain-shaped can be tested here without moving the vehicle: `fakevehicle.py`
takes `SIM_LAT`/`SIM_LON` for that.

A survey is one visual item holding a whole flight. Reading its coordinate
gives a point, and the profile charted a 6.43 km mission as the 0.25 km hop out
to it. `complexDistance` is the distance the item covers on its own and
`exitCoordinate` is where the next item is measured from.

Item 0 of `visualItems` is the mission settings item - QGC finds it by that
position too, `_visualItems->value<MissionSettingsItem*>(0)`. It is the planned
home, not a leg that gets flown, and `MissionController` leaves it out of
`missionTotalDistance` under `lastFlyThroughVI != _settingsItem`. Counting it
put a different distance in the profile than in the status line above it.

The two altitude sources disagree, and each is right about one thing. Every
segment of a 5.9 km survey reported `coord1AMSLAlt` as 50 - the height above
launch - while the item's `amslEntryAlt` read 1099 against 1049 of terrain.
The item is resolved to AMSL; the segments are not. But the segments carry a
run of `amslTerrainHeights` each, which is the only place the real ground curve
exists. Ground from the segments, planned altitude from the item.

Reading one survey's `flightPathSegments` is 31 KB of JSON across 67 segments,
and it is fetched only for items that have a `complexDistance`. Measured on the
phone, one survey: 20 ms per poll with segments against 0.1 ms without, and an
empty plan is unaffected at 104 us. That is 3% of the 700 ms poll for one
survey and it sits on Dispatchers.Default, so nothing was cached for it. The
number to watch is the count: the poll already cost 164 ms at nine surveys
before segments existed, and nine more segment reads would put it near half the
interval. Cache it then, and remember that terrain arrives after the plan stops
changing - a cache keyed on the plan alone would freeze an empty profile.

## Weight

Upload is a filled button and everything else is text. It is the action that finishes the job — the
mission reaching the aircraft — and it used to be one of twelve controls of identical weight, styled
exactly like Fit, which only moves the camera. A screen with every command and no primary action
makes the pilot find the important one unaided on every pass.

Dividers separate the three kinds: vehicle sync, item creation, view control. They were adjacent
with nothing to say where one ended.

## Naming

The vehicle sync buttons are `Download` and `Upload`, which is what QGC calls them — "Download from
Vehicle" and "Upload" in `PlanView.qml`. They used to be `Load` and `Send`. That mattered once the
app's Plan tab grew an `Open / Save / Save as` row above the map: `Load` beside `Open` reads as two
ways to do one thing, when one moves a plan to and from the aircraft and the other to and from a
file. Taking QGC's vocabulary rather than inventing keeps the two pairs distinct without anyone
having to learn a local dialect.

## Scripting the controls

The control panel is anchored to the bottom and grows with what the plan contains, so every button
moves when the plan changes. With an empty plan `Survey` sits at y=2027 on a 1080x2280 screen; add
one survey and the terrain profile and selection row appear beneath it and the same button is at
y=1906. A script that reuses a coordinate after changing the plan taps empty space and reports
nothing wrong, which is how a measurement of six surveys turned out to be six taps into a gap and
one survey. Screenshot between steps and locate the button each time.

`input swipe` cannot drive a drag; `input draganddrop` can, and `input swipe x y x y 900` is a long
press.

## Tests

```
./gradlew :map-spike:testDebugUnitTest
```

Everything testable off-device is pure Kotlin: JSON parsing, tile hashing, ring geometry, track
handling, profile building. Anything touching MapLibre or the bridge is verified on hardware
instead, which is why each commit says what was checked on the phone.
