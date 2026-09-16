# What this rig can and cannot make

The macOS head's "cannot make" list lived only in a standing prompt, where nothing could
check it. It decayed in both directions: I once took **a vehicle that goes away** off it
without making one (it is not makeable — see below), and I kept five entries on it after
`android/tools/apmvehicle.py` grew the knobs that produce them.

**A `cannot make` entry is a claim about a moving file.** `apmvehicle.py` is the Android
session's and gains knobs without telling anyone. Before trusting an entry, re-derive it:

    grep -oE 'environ\.get\("[A-Z_]+"' android/tools/apmvehicle.py | sort -u

## Candidates with a named mechanism — knob found, NOT yet witnessed

A knob existing is not the state existing: the input still has to reach the rule. Each of
these is a line in the fake that produces the MAVLink or parameter condition, read this
cycle, and none has been run yet.

| Case, previously listed as impossible | Knob | What it does |
|---|---|---|
| An unhealthy sensor | `SENSOR_FAULT=1` | `sys_status_send(present, enabled, health=0)` — sensors present and enabled with health zero, which is the shape a head must not read as absent |
| A setup verdict that is not good | `ACCEL_UNCAL=1` | `INS_ACCOFFS_{X,Y,Z} = 0.0`, an uncalibrated accelerometer |
| A setup verdict that is not good | `NO_COMPASS_FIT=1` | drops `COMPASS_CAL_FIT` entirely |
| A dead / unmapped RC channel | `NO_RCMAP=1` | drops `RCMAP_ROLL/PITCH/THROTTLE/YAW` |
| Two links | `SECOND_PORT` | a second UDP port on the same host, so QGC carries one vehicle on two links |
| A parameter whose enum value is off the list | `OFFLIST_ENUM=1` | `FRAME_CLASS = 99.0` |
| No battery at all | `NO_BATTERY=1` | drops the battery parameters |

## Measured, and genuinely not makeable here

- **A vehicle that goes away with a window open.** Killing the fake produces `contactLost`,
  never removal: 240 s after the kill `connected` was still true. `VehicleLinkManager::_linkDisconnected`
  (`:244`) is what empties `_rgLinkInfo` and fires `allLinksRemoved` into
  `MultiVehicleManager::_deleteVehiclePhase1` (`:146`) — a **link** event, not a heartbeat
  timeout, and a UDP link does not disconnect because a sender stopped. The only route is
  disconnecting a link, which this session must not do.
- **A tracked camera rectangle reaching the head.** `CAM_TRACKING=1` does make the *vehicle*
  report one — the fake sends `CAMERA_TRACKING_IMAGE_STATUS` with an ACTIVE rectangle every
  fifth tick — but `VehicleCameraControl.cc:1672` clears `_trackingImageRect` unless the
  ground station's own `trackingEnabled` is set, and arming that sends a tracking command.
  So the rect is null for a camera actively reporting a box, and the knob alone cannot fix
  it. **The reason is the ground station's flag, not the rig**, which is worth knowing before
  anyone spends another cycle on the fake.
- **ADS-B contacts.** `ADSB_CONTACTS=n` fills the feed, but the panel reads `Traffic off`
  until the ADS-B feature toggle is on, and that is a settings write this session must not make.

## Still listed as impossible, no knob found this pass

A receiver reporting channels with nothing live; a keep-out or firmware fence; the populated
Lights/Camera case; an empty log list; a read-only settings fact; a PX4 vehicle; a submarine;
a live sensor with nothing in range; an imperial profile whose conversions do not cancel; a
waypoint-first plan; a posted host notice; a populated plan; a backwards-clock log; a log with
a system that never heartbeat; a successful geotag run.
