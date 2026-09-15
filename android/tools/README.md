# Rig

The scripts that drive a handset against a simulated vehicle. They were written in a session
scratchpad, which does not survive the session; they live here so the next one does not rebuild
them.

Everything assumes `adb` on the PATH. Ask `handset-ip.sh` for the address rather than remembering one: DHCP moved it mid-session once, and a stale address looks exactly like a broken app — "No vehicle" and nothing on the wire. Paths inside the scripts
still point at the scratchpad they were written in — fix those before use, or run them from a
copy there.

| script | what it does |
|---|---|
| `handset-ip.sh` | The handset's Wi-Fi address, from `adb`. |
| `ui.sh` | `front`, `tap`, `swipe`, `text`, `key`, `shot`. Every input checks `topResumedActivity` first and refuses rather than guess: `am start` returns before the window is up, and a tap sent into that gap lands in whatever the user had open. |
| `apmvehicle.py` | An ArduPilot-shaped MAVLink vehicle: heartbeats, GPS, battery, RC, vibration, two camera components, log download, and a print for every command it receives. Its environment knobs are the point of it — see the table below. |
| `device-lock.sh` | `take`/`drop` around `/tmp/aircast-device.lock`, and stops the handset dozing while held. |
| `regress.sh` | Drives Fly, the Actions sheet, Vibration, Log Download and Settings, captures each, and fails a capture under 100 000 bytes because a sleeping screen photographs as a small black rectangle. |
| `detrig.sh` | `up`/`down` for the detection overlay: an SSE feed on 8099 and a TCP video stream on 8100, both through `adb reverse`, with the device ini pointed at `127.0.0.1`. |
| `detfeed.py` | The SSE detection feed `detrig.sh` serves — two boxes, one of them moving. |
| `shots.sh` | Screenshots of the tabs that **refuse to save a picture of the wrong screen**. `ui.sh pick` exits 1 when it cannot find a node and scripts routinely silence that, and a confirm dialog left open swallows later taps while every command still succeeds - so this checks the exit status and that the screen actually changed, stepping off the target tab first so "unchanged" can only mean the tap reached nothing. |
| `udptcp.py` | Relays `apmvehicle.py`'s UDP to TCP 5771 so the emulator can reach it: `adb reverse` carries TCP only, so without this the emulator has no vehicle at all. Bidirectional, so parameter and mission requests still get answers. |
| `videofeed.sh` | A decodable h.264/MPEG-TS stream on 5600 for the emulator, with the reverse forward. Set Settings > Video to **TCP-MPEG2** and the url to `127.0.0.1:5600`. The UDP source cannot be fed from the host at all, which is why video looked impossible here. Without it `video.decoding` stays false and the Fly view's detection overlay never composes — the feature reads as dead rather than unconfigured. |
| `whatsunder.py` | What flight control, if any, sits under a point. `ui.sh tap` refuses rather than guess, and refuses when this cannot answer — it failed open for six hours on 2026-09-11 because its own pattern would not parse. |
| `whatsunder_test.py` | `python3 tools/whatsunder_test.py`. No device needed. Pins both quote styles, the planning-screen exemptions, that unreadable input exits non-zero, and that `PLAN_ITEMS_HEADING` in the Kotlin still matches the literal the guard looks for. |
| — | **A path with a space works now, and took three separate fixes.** The bridge did not decode its path parameter (fixed in `e99ebb6e0`); `probe.sh get` interpolated the path raw into the URL where `set` had always encoded it; and `curl --data-urlencode` writes a space as `+`, which `QUrl::FullyDecoded` leaves as a literal plus — a log under "Aircast QGC Daily" came back as "Aircast+QGC+Daily" and read as unreadable. `get` now percent-encodes with python so a space arrives as `%20`. Verified against a real telemetry log: `readable: true`, 1079824 bytes, path echoed back intact. |
| `rccal.py` | drives a whole RC calibration through the head: reads `view.radio`, writes the sticks the step asks for, taps Next where the state machine waits for it. Twelve steps, about two minutes. The Radio component stops asking for setup when it finishes. |
| — | **The emulator can now run the whole rig, vehicle and video included.** `ANDROID_SERIAL=emulator-5554` works with `installDebug`, `probe.sh`, `ui.sh` and `adb reverse`. It used to be limited to what needed no vehicle, because `apmvehicle.py` speaks UDP and `adb reverse` carries TCP only — `udptcp.py` closes that, and `videofeed.sh` gives it a decodable stream. It is also where the packet-radio startup crash was found. Its profile is imperial, which makes it the cheapest way to see a unit conversion that a metric handset hides. |
| `sbs1feed.py` | An SBS-1 traffic server on 30003 for the ADSB receiver: `adb reverse tcp:30003 tcp:30099` (the device keeps the conventional SBS port, the host listens on 30099 so the feed does not take 30003 from `ADSBTest`, which binds it), switch on Settings > ADSB Traffic, and contacts appear. `SBS_CONTACTS`, `SBS_ALERT=1` for an aircraft squawking an alert. It serves every client in its own thread, because QGC's C++ ADSB manager takes a connection too and a single-connection server leaves the core's reader attached to a socket that never gets a byte - which reads exactly like a broken parser. |
| — | **Corrupting a setting to test a refusal.** QSettings lives at `./files/settings/Aircast/Aircast QGC Daily.ini` under `run-as one.aircast.android`, and the app must be force-stopped first or it rewrites the file on exit. Back the whole file up and restore it the same way (`adb push` to `/data/local/tmp`, then `run-as ... sh -c 'cat ... > file'`) - a `sed -i` through `run-as` breaks on the quoting in a JSON value. This is how the unreadable-extraVideoSources refusal was verified. |
| `contract-check.py` | `python3 tools/contract-check.py`. No device needed. Reads `test/Bridge/fixtures/view-shapes.json` and compares it against what this head decodes: fields served and never read, fields recorded empty, acceptances that have gone stale, and **bare `optBoolean` reads of fields the core can serve as null** - `optBoolean` returns `false` for a JSON null, and false is the reassuring answer every time (*contact is fine*, *checked and not ready*, *the reading is current*). It refuses rather than reporting a clean head if the contract parse finds no `bool|null` fields or the source walk finds no `.kt` reads, because an empty result and a clean result are indistinguishable otherwise. |
| `unread-fields.py` | `python3 tools/unread-fields.py`. No device needed. Finds fields this head DECODES from a view and then never dereferences - the mirror of `served-unread.py`, which finds fields the core serves and the head never names. It runs clean: every hit has been drawn, deleted, or accepted with a written reason, so a new one is a new finding rather than a number to compare against. It self-checks against a known answer first and refuses to run if that answer is wrong, because the naive version of this check reported every property in the head as dead. |
| `settings-groups.py` | `python3 tools/settings-groups.py`. No device needed. Diffs the settings groups the core serves against the ones `SETTINGS_GROUPS` lists, and exits non-zero for any group that is neither drawn nor exempt with a written reason. The list was eleven groups behind the core on 2026-09-14 and nothing said so. |
| `watchprobe.py` | `on`/`off` around timing instrumentation in `Watcher::_poll`. It asserts the poll body is in the shape it expects, so it fails loudly when the bridge changes rather than patching the wrong thing. |

## What `apmvehicle.py` can pretend to be

Every knob here exists because a screen rendered one way and nothing could
falsify it. The ones marked *found something* are in the commit log for
2026-09-13.

| knob | the state it produces | |
|---|---|---|
| `VEHICLE=copter\|plane\|vtol` | the airframe in every heartbeat | *found something* |
| `SECOND_PORT=14551` | the same vehicle heard on two links, which is a packet radio beside an LTE modem | *found something* |
| `STICKS_FILE` | eight pwm values to hold. Zero leaves a channel to its own sweep, **negative reports it as carrying no signal** | *found something* |
| `SENSOR_FAULT=1` | a sensor present and enabled but unhealthy, which is failing rather than absent | confirmed a claim |
| `NOFIX=1`, `NOFIX_SECONDS` | no GPS fix, optionally for a while and then a fix | |
| `RC_RSSI`, `BATT_PCT`, `BATT_STATE` | signal and battery readings | |
| `FIRMWARE` | the version string in `AUTOPILOT_VERSION` | |
| `NO_FENCE=1` | a vehicle without fence and rally capability bits | |
| `OFFLIST_ENUM=1` | `FRAME_CLASS` at 99, a value outside its own enum list, so `Fact` appends its synthetic "Unknown: 99" entry | *the case three heads got wrong* |
| `NO_RCMAP=1`, `ACCEL_UNCAL=1`, `ARMING_CHECK`, `TRAILING_PARAM` | parameter-shaped edge cases | |
| `SIM_LAT`, `SIM_LON`, `FENCE_RADIUS` | where it flies and how far it may go | *see terrain below* |
| `STILL_STICKS=1` | sticks that do not sweep, for a screenshot that does not change under you | |
| `STATUS_AT`, `STATUS_EVERY` | when STATUSTEXT messages arrive | |
| `ORBIT_RADIUS_DEG` | how wide a circle the vehicle flies, default 0.004 (about 445 m). Small values put the vehicle and the operator on screen together, which is the only way to see both markers at once | |
| `NO_COMPASS_FIT=1` | drops `COMPASS_CAL_FIT`, the parameter whose absence crashed the app when a compass calibration started | *found something* |
| `NO_BATTERY=1` | drops the whole `BATT_*` set, which is the only way back to the Power page's empty state now that the rig serves them | |
| `CAM_INTERVAL=1` | the camera reports `CAMERA_CAPTURE_STATUS` with image_status 3, an interval capture in progress. This is the only way to reach `canStopPhoto`, and QGC withdraws `canPhoto` while it runs - so before the Stop control existed the shutter simply vanished and nothing could end the capture | *found something* |
| `ORBIT=<metres>` | the vehicle reports an active orbit of that radius, negative for anticlockwise. The message is hand-framed: `ORBIT_EXECUTION_STATUS` is id 360 and this pymavlink (2.4.49) does not carry it, so the rig packs `<Qfiifb` with crc_extra 11 from the generated header and QGC's own parser is what proves the framing | |
| `ADSB_CONTACTS`, `ADSB_SQUAWK` | aircraft relayed as `ADSB_VEHICLE`. These only reach the core when the core owns the link (`coreLinks`), which is off by default, so `sbs1feed.py` is the way in until that flips | |

**Terrain needs somewhere the service covers.** The default position is Tbilisi
and `terrain-ce.suite.auterion.com` has no tile for it, so every plan reads
"ground height unknown" and the terrain profile, `altitudeAmsl` and the
Calculated Above Terrain mode all render their empty case. `SIM_LAT=47.3769
SIM_LON=8.5417` puts it over Zurich, where the carpet endpoint answers 200 with
real elevations. Two defects on 2026-09-13 were only visible with terrain
present. Note the coordinates endpoint returns 501 everywhere tested, so only
the carpet-fed surfaces come alive.

The rule they came from: before auditing a screen, ask which of its inputs this
rig has only ever produced one value for, and make it produce a second. Four of
the five added on 2026-09-13 were under ten lines.


The rule these were built to serve: an empty screen is not evidence that a screen works. Every
one of them reports what the vehicle received or what the screen actually showed, not that a
command was sent.

## apmvehicle.py — what the fake actually does

It is a **fake vehicle**, not a probe: it sends MAVLink at a target. Anything
odd on a head's telemetry has to be checked against this file before it is
called a defect.

- **It orbits whether or not it is armed.** Position, heading and distance to
  home all advance from elapsed time with no reference to the armed flag, so a
  disarmed vehicle drifts across the map at a steady heading. That is the fake,
  not the head. It cost a chunk of a session on 2026-09-12 before the hardcoded
  speed gave it away.
- **Speed now matches the orbit.** `GROUND_SPEED` drives `ORBIT_SECONDS`, so
  what `VFR_HUD` reports is what the position is doing. It used to send a
  hardcoded `8.1` while the circle implied 62.2 m/s — a 445 m radius every 45
  seconds, which no quadrotor flies. Change `GROUND_SPEED` and the period
  follows.
- **A running instance keeps the old behaviour** until it is restarted, and the
  sim is shared with the other sessions, so restart it deliberately rather than
  as a side effect.

Facts the code no longer states, which are still true:

- A **second MAVLink component** is announced so QGC's camera manager has
  something to discover.
- The **obstacle ring stops after 45 s**, deliberately, so a reader can watch
  the display go stale. It is 72 sectors of 5 degrees with one obstacle to the
  right — index 18, 90 degrees — at 3.20 m, everything else out of range.
- **`STATUS_TEXTS` are sent well after start and then periodically** (`STATUS_AT`
  45 s, `STATUS_EVERY` 60 s). A ground station takes about half a minute to
  bring its link up and anything the vehicle says before that is said to nobody.
  This once cost an afternoon spent deciding the message banner was broken.
- **ArduPilot makes QGC try the parameter download over MAVFTP first.** A NAK of
  `FileNotFound` is what makes it fall back to `PARAM_REQUEST_LIST`; ignoring
  the request just makes it retry.

### Fences and rally points

The fake stores missions per `mission_type` and advertises
`MAV_PROTOCOL_CAPABILITY_MISSION_FENCE` and `_MISSION_RALLY`. Before
2026-09-12 it did neither: it acked any non-zero type and kept nothing, and
without the capability bits QGC correctly declined to send them at all. Either
alone is enough to make a fence round trip look like a head defect.

**QGC caches `capabilityBits` from the `AUTOPILOT_VERSION` it receives when the
vehicle connects.** Restarting the sim is not enough to pick up a capability
change — force-stop the app too, or it will keep the old answer and go on not
sending.

### Panning the map with `input swipe` can drag a waypoint

A swipe that starts on a marker **drags that marker** rather than panning, and
nothing on screen says so — the plan silently changes shape. Caught on
2026-09-12 when a 212-item plan went from 9.87 km to 10.93 km across three pan
gestures; the distance in the summary chip was the only tell.

Start a pan on empty map, and read the summary chip before and after: if the
distance moved, the plan moved. Reload the file before trusting any measurement
taken after a pan.

### VEHICLE=plane / vtol

`apmvehicle.py` heartbeats as a quadrotor by default. `VEHICLE=plane` or
`VEHICLE=vtol` changes the type, which is the only way to exercise the paths
QGC gates on airframe — the landing pattern above all, since
`MissionController::insertLandItem` builds a `FixedWingLandingComplexItem` for a
plane, a `VTOLLandingComplexItem` for a VTOL, and a plain RTL for anything else.

**Two things cache and will mislead you.** QGC takes the vehicle type from the
heartbeat at connect, so force-stop the app after changing it or it keeps
reporting the old airframe. And the *plan* carries its own `vehicleType`: a plan
file saved as a multirotor makes `insertLandItem` add an RTL even when a plane
is connected, because the plan controller follows the plan rather than the link.

### A scripted sequence can run against the file picker

`ui.sh` refuses taps unless the app is in front, but raw `adb shell input`
does not. On 2026-09-12 a Save-as picker was left open and an entire
load-add-upload sequence ran against `documentsui` instead — every step
"succeeded", the sim saw no upload, and the screen showed no plan.

Check `dumpsys activity activities | grep topResumedActivity` before a
sequence that mixes `ui.sh` with raw input, and dismiss a picker deliberately
rather than assuming one BACK did it.
