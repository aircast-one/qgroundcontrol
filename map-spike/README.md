# map-spike

A native map for the Android frontend: MapLibre in Compose, driven entirely through
`QGCBridgeCore`, on tiles from QGroundControl's own cache.

Phase 4 of `NATIVE_ANDROID_REWRITE.md`. It began as a spike asking whether a native map could
replace the QML one. It does: `:app` hosts it as the Plan tab. The module name has outlived the
question it was named after, and renaming it would touch `:app`, so it stands for now.

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

**The main app has to start first.** Launching the spike alone never boots Qt, so the bridge
natives are unregistered and every call throws. The header says so when that happens.

Placing anything uses the vehicle's position, falling back to the map centre, so the spike can be
driven with no vehicle connected.

## What works

Mission items, geofence polygons, geofence circles, rally points and surveys can each be created,
moved and deleted. Surveys generate transects and their grid rotates. Waypoint altitude is
editable. A terrain profile plots planned altitude against distance. All verified on a device
against a live vehicle.

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

**Read a list once and parse it many times.** Mission items, surveys and the terrain profile all
come out of the same item list. Fetching it once per poll instead of once per consumer halved the
poll, from a median of 66 ms to 32 ms on an 84 point survey. Each call serialises the whole
subtree to JSON and blocks the Qt thread while it does, so the cost is in the trip, not the
parsing.

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

**Loading from the vehicle throws work away.** It overwrites whatever is drawn and there is no
undo, so QGC asks first when the plan has unsent changes. `plan.dirty` says when that is, and Load
here asks the same question rather than being the one place that discards a survey silently.

**`clearAllInteractive` does not clear anything stored.** It ends an interactive edit. A button
built on it reports success and changes nothing. `deletePolygon` is the real path.

**Writes cannot take an object.** `invoke` resolves an `@path` argument to a QObject, but `set`
does not. So `activeVehicle`, which is a writable property taking a `Vehicle*`, cannot be written,
and there is no vehicle picker here. Teaching `writePath` the same `@path` form would fix it and
would serve the macOS frontend equally.

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

## Tiles

`QgcTileCache` reads QGroundControl's SQLite cache directly and serves tiles through an OkHttp
interceptor on a private host, so the map works offline on imagery already downloaded. Tiles are
keyed by a 29 character hash: a 10 digit provider hash then x, y and z zero padded to 8, 8 and 3.

QGroundControl writes that database while the map reads it, and a read-only connection cannot roll
back a journal it finds mid-write. The exception lands on OkHttp's thread and takes the process
down, so a failed tile read yields no tile instead.

Falls back to OpenStreetMap when the cache is empty. That style is for development only.

## Tests

```
./gradlew :map-spike:testDebugUnitTest
```

Everything testable off-device is pure Kotlin: JSON parsing, tile hashing, ring geometry, track
handling, profile building. Anything touching MapLibre or the bridge is verified on hardware
instead, which is why each commit says what was checked on the phone.
