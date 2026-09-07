# map-spike

A native map for the Android frontend: MapLibre in Compose, driven entirely through
`QGCBridgeCore`, on tiles from QGroundControl's own cache.

Phase 4 of `NATIVE_ANDROID_REWRITE.md`. This module exists to answer whether a native map can
replace the QML one, and the answer is yes.

## Hosting it in the app

`:app` already depends on this module. Drop the map into the app's own navigation with

```kotlin
PlanMapScreen()
```

It sets itself up on first use and survives being entered again, which matters because MapLibre has
to be initialised before the tile source replaces the HTTP client and that client may only be
replaced once. The activity below is the standalone harness and now calls the same composable.

## Running it standalone

The spike is an Activity of its own, deliberately outside the app's navigation so it cannot
disturb the shipping tabs:

```
adb shell am start -n one.aircast.android/.MainActivity          # boots Qt
adb shell am start -n one.aircast.android/one.aircast.mapspike.MapSpikeActivity
```

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

**A plan item can exist without being on the map.** Inserting a takeoff puts a real item in the
plan, but on a copter it specifies no coordinate of its own, so nothing is drawn and the item
count does not move. Counting drawn markers to decide whether a bridge call worked reports a
working call as a failure. The raw element count is the one that answers that question.

**Mission Start is drawn like a waypoint but is not one.** The settings item takes a coordinate
once the plan has something in it, so it appears as a numbered dot at the planned launch point.
It is coloured apart from the waypoints for that reason.

**A bad index is a segfault, not an exception.** `insertSimpleMissionItem` with an index past the
end walks off the list and takes the process down, tombstone and all. The bridge validates types,
never ranges. The insert index addresses the whole visual item list, which opens with a settings
item and can hold items carrying no coordinate, so counting only what is drawn gives the wrong
number.

**Never call the bridge from the main thread.** Calls block on the Qt thread. Touch handlers and
button callbacks all go through a background dispatcher.

**Read a list once and parse it many times.** Mission items, surveys and the terrain profile all
come out of the same item list. Fetching it once per poll instead of once per consumer halved the
poll, from a median of 66 ms to 32 ms on an 84 point survey. Each call serialises the whole
subtree to JSON and blocks the Qt thread while it does, so the cost is in the trip, not the
parsing.

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
