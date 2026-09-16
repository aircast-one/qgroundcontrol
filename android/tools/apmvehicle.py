import math
import os
import socket
import struct
import sys

import time

NO_FENCE = os.environ.get("NO_FENCE") == "1"

# Sectors the proximity ring reports, as "index:centimetres" pairs. The default is the
# single reading this fake has always sent; a head drawing an arc needs several bearings
# and a near/far mix to show that placement and scaling are right.
OBSTACLE_RING = [
    tuple(int(part) for part in pair.split(":"))
    for pair in os.environ.get("OBSTACLE_RING", "18:320").split(",")
    if pair
]
OBSTACLE_SECONDS = float(os.environ.get("OBSTACLE_SECONDS", "45"))

# A camera that advertises tracking. Off by default so the rig keeps reporting what a
# plain camera reports; a head drawing tracking controls needs a camera that claims them.
CAM_TRACKING = os.environ.get("CAM_TRACKING") == "1"

# The rectangle a tracking camera reports, normalised 0..1 as top-left/bottom-right.
TRACK_RECT = [float(part) for part in os.environ.get("TRACK_RECT", "0.35,0.30,0.65,0.70").split(",")]

# Whether the camera also reports an active track. Off gives a camera that CAN track and is
# not tracking, which is the state a head offers to start one in.
CAM_TRACK_STATUS = os.environ.get("CAM_TRACK_STATUS", "1") != "0"
# QGC discards the tracked rectangle unless its OWN trackingEnabled is set, and that
# only turns on when the vehicle acknowledges the start. Without these three acks the
# camera reports tracking, QGC decodes it, and throws the box away.
TRACKING_COMMANDS = (2004, 2005, 2010)

# A SiK radio reports its own link with RADIO_STATUS. Every field is zero before the
# first one arrives, which is why the core serves the whole telemetry object as null
# until then - so without this knob there is no way to see the reading at all.
RADIO_LINK = os.environ.get("RADIO_LINK")

# A camera with a storage card, so a head can draw a Format row. Off by default: without it
# the camera reports no storage and Format is correctly hidden, which is the only arm the
# contract recording has ever witnessed.
CAM_STORAGE = os.environ.get("CAM_STORAGE") == "1"

from pymavlink.dialects.v20 import ardupilotmega as apm
from pymavlink.dialects.v20 import common as mavlink
from pymavlink.generator.mavcrc import x25crc

TARGET = (sys.argv[1], int(os.environ.get("RIG_UDP_PORT", "14550")))

INT_PARAMS = frozenset(
    ["ARMING_CHECK", "FS_OPTIONS", "SIMPLE", "SUPER_SIMPLE", "FS_THR_ENABLE",
     "FLTMODE_CH", "INITIAL_MODE", "FRAME_CLASS", "FRAME_TYPE", "GRIP_ENABLE",
     "BATT_MONITOR", "RTL_ALT", "FENCE_ENABLE", "FENCE_TYPE",
     "BATT_FS_LOW_ACT", "BATT_FS_CRT_ACT", "BATT_CAPACITY", "BATT_VOLT_PIN",
     "BATT_CURR_PIN", "BATT2_MONITOR", "BATT2_FS_LOW_ACT", "BATT2_FS_CRT_ACT",
     "BATT2_CAPACITY", "FENCE_ACTION", "FS_GCS_ENABLE", "FRAME",
     "COMPASS_DEV_ID", "COMPASS_DEV_ID2", "COMPASS_DEV_ID3",
     "COMPASS_USE", "COMPASS_USE2", "COMPASS_USE3", "COMPASS_LEARN",
     "BATT2_VOLT_PIN", "BATT2_CURR_PIN"]
    + ["RC%d_TRIM" % ch for ch in range(1, 9)]
    + ["FLTMODE%d" % slot for slot in range(1, 7)]
    + ["RCMAP_ROLL", "RCMAP_PITCH", "RCMAP_THROTTLE", "RCMAP_YAW"]
    + ["RC%d_MIN" % ch for ch in range(1, 9)]
    + ["RC%d_MAX" % ch for ch in range(1, 9)]
    + ["RC%d_REVERSED" % ch for ch in range(1, 9)]
)

MAGCAL_MASK = 0b011
ACCELCAL_POSITIONS = [1, 2, 3, 4, 5, 6]
CAMERA_FEEDBACK_EVERY = int(os.environ.get("CAMERA_FEEDBACK_EVERY", "0"))
ADSB_CONTACTS = int(os.environ.get("ADSB_CONTACTS", "0"))
ORBIT_RADIUS_M = float(os.environ.get("ORBIT", "0"))
CAM_INTERVAL = os.environ.get("CAM_INTERVAL") == "1"
ADSB_SQUAWK = int(os.environ.get("ADSB_SQUAWK", "1200"))
ADSB_SIMULATED = int(os.environ.get("ADSB_SIMULATED", "0"))
VIBRATION_AXES = os.environ.get("VIBRATION_AXES", "xyz")
GROUND_ALTITUDE = 0.5
CLIMB_RATE = 2.0
DEFAULT_TAKEOFF_ALTITUDE = 10.0
FLYING_ALTITUDE = (lambda metres: float(metres) if metres else None)(os.environ.get("FLYING"))
ALTITUDE_DRIFT_M = float(os.environ.get("ALTITUDE_DRIFT", "3.0"))
COPTER_MODE_RTL = 6
COPTER_MODE_LAND = 9
DESCENDING_MODES = frozenset([COPTER_MODE_RTL, COPTER_MODE_LAND])

FIRMWARE = os.environ.get("FIRMWARE", "4.5.7")
FIRMWARE_VERSION = (
    lambda parts: (parts[0] << 24) | (parts[1] << 16) | (parts[2] << 8) | 255
)([int(part) for part in FIRMWARE.split(".")])
SYSID = int(sys.argv[2]) if len(sys.argv) > 2 else 1


def addressed_here(message) -> bool:
    target = getattr(message, "target_system", 0)
    return target in (0, SYSID)

GPS_SENSOR = 32
LOG_SIZES = [4096, 10240]
LOG_BASE_UTC = 1757280000
STATUS_TEXTS = [
    (6, b"AircastSim ready"),
    (4, b"GPS glitch cleared"),
    (3, b"EKF variance"),
    (6, b"Batt & temp < 40C"),
]
CENTRE_SHIFT = (SYSID - 1) * 0.01
CENTRE_LAT = float(os.environ.get("SIM_LAT", "41.7151"))
CENTRE_LON = float(os.environ.get("SIM_LON", "44.8271"))
RADIUS_DEG = float(os.environ.get("ORBIT_RADIUS_DEG", "0.004"))
VEHICLE_TYPES = {
    "copter": mavlink.MAV_TYPE_QUADROTOR,
    "plane": mavlink.MAV_TYPE_FIXED_WING,
    "vtol": mavlink.MAV_TYPE_VTOL_QUADROTOR,
}
VEHICLE_TYPE = VEHICLE_TYPES[os.environ.get("VEHICLE", "copter")]
GROUND_SPEED = 8.0
AIRSPEED = 7.5
METRES_PER_DEGREE = 111320.0
ORBIT_SECONDS = 2 * math.pi * RADIUS_DEG * METRES_PER_DEGREE / GROUND_SPEED


class Sender:
    """Writes every frame to each target, so one vehicle is heard on several links.

    SECOND_PORT adds a second udp port on the same host. QGC treats each port it
    hears the same system on as its own link, which is the only way this rig can
    show a vehicle carried by more than one radio. SECOND_PORT_SECONDS then goes
    quiet on that port, which is a vehicle that has lost one of its two radios
    and is still flying on the other.

    QUIET_AFTER goes quiet on every port at once while the socket stays open, so
    the link is up and carrying nothing. That is the failure worth testing: a
    closed connection makes QGC drop the link outright, and the reading vanishes
    instead of degrading.
    """

    def __init__(self, sock, target):
        self.sock = sock
        second = os.environ.get("SECOND_PORT")
        self.started = time.time()
        self.primary = target
        self.second = (target[0], int(second)) if second else None
        self.quiet_after = float(os.environ.get("SECOND_PORT_SECONDS", "1e9"))
        self.silent_after = float(os.environ.get("QUIET_AFTER", "1e9"))

    def targets(self):
        if time.time() - self.started > self.silent_after:
            return []
        if self.second is None or time.time() - self.started > self.quiet_after:
            return [self.primary]
        return [self.primary, self.second]

    def write(self, data):
        for target in self.targets():
            self.sock.sendto(data, target)


STICKS_FILE = os.environ.get("STICKS_FILE", "/tmp/aircast-sticks")
INTERVALS = {}


def read_sticks():
    """Eight PWM values the rig wants the transmitter to be sending, or None.

    A zero in any slot leaves that channel to whatever it was already doing, so a
    test can pin one stick without freezing the rest. A negative value reports the
    channel as zero, which is what a receiver sends for a channel carrying no
    signal - the case that separates "eight channels" from "eight channels
    carrying a signal".
    """
    try:
        with open(STICKS_FILE) as handle:
            values = [int(part) for part in handle.read().split()]
    except (OSError, ValueError):
        return None
    return (values + [0] * 8)[:8]



ORBIT_STATUS_ID = 360
ORBIT_STATUS_CRC = 11

CONTROL_STATUS_ID = 512
CONTROL_STATUS_CRC = 184
CONTROL_STATUS_SYSTEM_MANAGER = 1
CONTROL_STATUS_TAKEOVER_ALLOWED = 2
IN_CONTROL = (lambda held: int(held) if held else None)(os.environ.get("IN_CONTROL"))
TAKEOVER_ALLOWED = os.environ.get("TAKEOVER_ALLOWED", "1") not in ("0", "")


def control_status_frame(sequence, sysid, holder):
    flags = CONTROL_STATUS_SYSTEM_MANAGER | (CONTROL_STATUS_TAKEOVER_ALLOWED if TAKEOVER_ALLOWED else 0)
    payload = struct.pack("<BB", holder & 0xFF, flags)
    header = struct.pack("<BBBBBB", len(payload), 0, 0, sequence & 0xFF, sysid,
                         mavlink.MAV_COMP_ID_AUTOPILOT1)
    body = header + struct.pack("<I", CONTROL_STATUS_ID)[:3] + payload
    checksum = x25crc(body + bytes([CONTROL_STATUS_CRC])).crc
    return b"\xfd" + body + struct.pack("<H", checksum)


def orbit_status_frame(sequence, sysid, radius_m, latitude, longitude, altitude_m):
    payload = struct.pack("<Qfiifb", int(time.time() * 1e6), radius_m,
                          int(latitude * 1e7), int(longitude * 1e7), altitude_m, 0)
    header = struct.pack("<BBBBBB", len(payload), 0, 0, sequence & 0xFF, sysid,
                         mavlink.MAV_COMP_ID_AUTOPILOT1)
    body = header + struct.pack("<I", ORBIT_STATUS_ID)[:3] + payload
    checksum = x25crc(body + bytes([ORBIT_STATUS_CRC])).crc
    return b"\xfd" + body + struct.pack("<H", checksum)


def main():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("0.0.0.0", 0))
    sock.settimeout(0.05)

    link = mavlink.MAVLink(Sender(sock, TARGET), srcSystem=SYSID,
                           srcComponent=mavlink.MAV_COMP_ID_AUTOPILOT1)
    magcal = apm.MAVLink(Sender(sock, TARGET), srcSystem=SYSID,
                         srcComponent=mavlink.MAV_COMP_ID_AUTOPILOT1)
    magcal_started = [None]
    accelcal = [None]
    accelcal_sent = [None]
    parser = mavlink.MAVLink(None, srcSystem=255, srcComponent=0)
    sent_status = [0]
    cameras = {
        mavlink.MAV_COMP_ID_CAMERA: b"SimCam",
        mavlink.MAV_COMP_ID_CAMERA2: b"SimCam Thermal",
    }
    links = {
        comp: mavlink.MAVLink(Sender(sock, TARGET), srcSystem=SYSID, srcComponent=comp)
        for comp in cameras
    }
    camera = links[mavlink.MAV_COMP_ID_CAMERA]

    def send_camera_information(comp=mavlink.MAV_COMP_ID_CAMERA):
        links[comp].camera_information_send(
            int((time.time() - boot) * 1000),
            list(b"Aircast".ljust(32, b"\0")),
            list(cameras[comp].ljust(32, b"\0")),
            (1 << 24),          # firmware_version
            4.5, 6.2, 4.6,      # focal length, sensor size
            1920, 1080, 0,
            mavlink.CAMERA_CAP_FLAGS_CAPTURE_IMAGE
            | mavlink.CAMERA_CAP_FLAGS_CAPTURE_VIDEO
            | mavlink.CAMERA_CAP_FLAGS_HAS_MODES
            | (mavlink.CAMERA_CAP_FLAGS_HAS_TRACKING_POINT if CAM_TRACKING else 0)
            | (mavlink.CAMERA_CAP_FLAGS_HAS_TRACKING_RECTANGLE if CAM_TRACKING else 0),
            0, b"")
        print("CAMERA_INFORMATION sent for %d" % comp, flush=True)

    cam_mode = [0]          # 0 photo, 1 video
    cam_recording = [False]

    def send_camera_settings():
        for cam_link in links.values():
            cam_link.camera_settings_send(
                int((time.time() - boot) * 1000), cam_mode[0], 0.0, 0.0)

    boot = time.time()
    armed = False
    mode = 0
    tick = 0

    altitude = FLYING_ALTITUDE or 0.0
    target_altitude = altitude
    landing = False
    armed = armed or FLYING_ALTITUDE is not None

    params = {}
    for i in range(200):
        params["SIM_VALUE_%03d" % i] = float(i)
    # QGC reads all three before it will report a circular geofence radius:
    # GeoFenceController.cc requires FENCE_ENABLE true and bit 1 of FENCE_TYPE.
    for name, value in (("FENCE_RADIUS", float(os.environ.get("FENCE_RADIUS", "0"))),
                        ("FENCE_ENABLE", 1.0 if os.environ.get("FENCE_RADIUS") else 0.0),
                        ("FENCE_TYPE", 2.0),
                        ("RTL_ALT", 1500.0), ("WPNAV_SPEED", 500.0), ("FS_THR_ENABLE", 1.0),
                        ("ARMING_CHECK", float(os.environ.get("ARMING_CHECK", "82"))),
                        ("FS_OPTIONS", 4.0),
                        ("FRAME_CLASS", 1.0), ("FRAME_TYPE", 1.0),
                        ("FLTMODE_CH", 5.0), ("SIMPLE", 5.0), ("SUPER_SIMPLE", 0.0),
                        ("INITIAL_MODE", 0.0), ("GRIP_ENABLE", 1.0)):
        params[name] = value
    accel_offset = 0.0 if os.environ.get("ACCEL_UNCAL") == "1" else 0.01
    for axis in ("X", "Y", "Z"):
        params["INS_ACCOFFS_%s" % axis] = accel_offset
    if os.environ.get("NO_RCMAP") != "1":
        for name, value in (("RCMAP_ROLL", 1.0), ("RCMAP_PITCH", 2.0),
                            ("RCMAP_THROTTLE", 3.0), ("RCMAP_YAW", 4.0)):
            params[name] = value
    if os.environ.get("NO_BATTERY") != "1":
        for name, value in (("BATT_MONITOR", 4.0), ("BATT_CAPACITY", 5000.0),
                            ("BATT_LOW_VOLT", 10.5), ("BATT_CRT_VOLT", 9.9),
                            ("BATT_LOW_MAH", 500.0), ("BATT_CRT_MAH", 200.0),
                            ("BATT_FS_LOW_ACT", 2.0), ("BATT_FS_CRT_ACT", 1.0),
                            ("BATT_VOLT_MULT", 10.1), ("BATT_AMP_PERVLT", 17.0),
                            ("BATT_VOLT_PIN", 2.0), ("BATT_CURR_PIN", 3.0),
                            ("BATT2_MONITOR", 0.0), ("BATT2_CAPACITY", 0.0),
                            ("BATT2_LOW_VOLT", 0.0), ("BATT2_CRT_VOLT", 0.0),
                            ("BATT2_LOW_MAH", 0.0), ("BATT2_CRT_MAH", 0.0),
                            ("BATT2_FS_LOW_ACT", 0.0), ("BATT2_FS_CRT_ACT", 0.0),
                            ("BATT_AMP_OFFSET", 0.0), ("BATT_ARM_VOLT", 0.0),
                            ("BATT2_VOLT_PIN", -1.0), ("BATT2_CURR_PIN", -1.0),
                            ("BATT2_VOLT_MULT", 10.1), ("BATT2_AMP_PERVLT", 17.0),
                            ("BATT2_AMP_OFFSET", 0.0), ("BATT2_ARM_VOLT", 0.0)):
            params[name] = value
    for name, value in (("FS_THR_VALUE", 975.0), ("FS_GCS_ENABLE", 0.0),
                        ("FENCE_ACTION", 1.0), ("FENCE_ALT_MAX", 100.0),
                        ("FENCE_MARGIN", 2.0), ("RTL_ALT_FINAL", 0.0),
                        ("RTL_LOIT_TIME", 5000.0), ("LAND_SPEED", 50.0),
                        ("FRAME", 1.0), ("COMPASS_DEV_ID", 97539.0),
                        ("COMPASS_DEV_ID2", 131874.0), ("COMPASS_DEV_ID3", 0.0),
                        ("COMPASS_OFS_X", 12.0), ("COMPASS_OFS_Y", -7.0),
                        ("COMPASS_OFS_Z", 33.0), ("COMPASS_OFS2_X", 9.0),
                        ("COMPASS_OFS2_Y", -4.0), ("COMPASS_OFS2_Z", 28.0),
                        ("COMPASS_USE", 1.0), ("COMPASS_USE2", 1.0),
                        ("COMPASS_USE3", 0.0), ("COMPASS_LEARN", 0.0)):
        params[name] = value
    for name, value in (("ATC_INPUT_TC", 0.15), ("ATC_ANG_RLL_P", 4.5),
                        ("ATC_ANG_PIT_P", 4.5), ("ATC_ANG_YAW_P", 4.5),
                        ("ATC_RAT_RLL_P", 0.135), ("ATC_RAT_RLL_I", 0.135),
                        ("ATC_RAT_RLL_D", 0.0036), ("ATC_RAT_PIT_P", 0.135),
                        ("ATC_RAT_PIT_I", 0.135), ("ATC_RAT_PIT_D", 0.0036),
                        ("ATC_RAT_YAW_P", 0.18), ("ATC_RAT_YAW_I", 0.018),
                        ("PSC_ACCZ_P", 0.5), ("PSC_ACCZ_I", 1.0),
                        ("MOT_SPIN_ARM", 0.10), ("MOT_SPIN_MIN", 0.15),
                        ("MOT_THST_HOVER", 0.35)):
        params[name] = value
    for channel in range(1, 9):
        params["RC%d_TRIM" % channel] = 1500.0
        params["RC%d_MIN" % channel] = 1100.0
        params["RC%d_MAX" % channel] = 1900.0
        params["RC%d_REVERSED" % channel] = 1.0 if channel == 4 else 0.0
    for slot in range(1, 7):
        params["FLTMODE%d" % slot] = float(slot)
    if os.environ.get("NO_COMPASS_FIT") != "1":
        params["COMPASS_CAL_FIT"] = 16.0
    if os.environ.get("TRAILING_PARAM") == "1":
        params["ZZ_TRAILER"] = 1.0
    if os.environ.get("OFFLIST_ENUM") == "1":
        params["FRAME_CLASS"] = 99.0
    names = sorted(params)

    def send_param(name):
        link.param_value_send(name.encode()[:16], params[name],
                              mavlink.MAV_PARAM_TYPE_INT32 if name in INT_PARAMS
                              else mavlink.MAV_PARAM_TYPE_REAL32,
                              len(names), names.index(name))

    stored = {}
    incoming = {}
    expected = 0
    uploading = 0

    while True:
        elapsed = time.time() - boot
        if tick % 10 == 0:
            print("CAM HEARTBEAT", flush=True)
            send_camera_settings()
            for comp, cam_link in links.items():
                cam_link.camera_capture_status_send(
                    int((time.time() - boot) * 1000),
                    3 if CAM_INTERVAL else 0,
                    1 if cam_recording[0] else 0, 5.0, 0, 0, 0)
                cam_link.heartbeat_send(mavlink.MAV_TYPE_CAMERA,
                                        mavlink.MAV_AUTOPILOT_INVALID, 0, 0,
                                        mavlink.MAV_STATE_ACTIVE)
                if CAM_STORAGE:
                    cam_link.storage_information_send(
                        int((time.time() - boot) * 1000),
                        1, 1, mavlink.STORAGE_STATUS_READY,
                        61440.0, 20480.0, 40960.0, 90.0, 60.0)
        if CAM_TRACKING and CAM_TRACK_STATUS and tick % 5 == 0:
            for cam_link in links.values():
                try:
                    cam_link.camera_tracking_image_status_send(
                        mavlink.CAMERA_TRACKING_STATUS_FLAGS_ACTIVE,
                        mavlink.CAMERA_TRACKING_MODE_RECTANGLE,
                        mavlink.CAMERA_TRACKING_TARGET_DATA_IN_STATUS,
                        float('nan'), float('nan'), float('nan'),
                        TRACK_RECT[0], TRACK_RECT[1], TRACK_RECT[2], TRACK_RECT[3])
                except Exception as exc:
                    print('TRACKING FAILED %r' % (exc,), flush=True)

        if tick % 5 == 0 and elapsed < OBSTACLE_SECONDS:
            ring = [65535] * 72
            for sector, centimetres in OBSTACLE_RING:
                ring[sector] = centimetres
            try:
                link.obstacle_distance_send(
                    int(elapsed * 1e6), 0, ring, 5, 20, 4000, 5.0, 0.0, 12)
                print("OBSTACLE sent", flush=True)
            except Exception as exc:
                print("OBSTACLE FAILED %r" % (exc,), flush=True)
        angle = (elapsed / ORBIT_SECONDS) * 2 * math.pi
        lat = CENTRE_LAT + CENTRE_SHIFT + RADIUS_DEG * math.cos(angle)
        lon = CENTRE_LON + RADIUS_DEG * math.sin(angle)
        heading = (math.degrees(angle) + 90.0) % 360.0
        now_ms = int(elapsed * 1000)

        base_mode = mavlink.MAV_MODE_FLAG_CUSTOM_MODE_ENABLED
        if armed:
            base_mode |= mavlink.MAV_MODE_FLAG_SAFETY_ARMED

        link.heartbeat_send(VEHICLE_TYPE,
                            mavlink.MAV_AUTOPILOT_ARDUPILOTMEGA,
                            base_mode, mode, mavlink.MAV_STATE_ACTIVE)
        nofix = os.environ.get("NOFIX") == "1" and elapsed < float(os.environ.get("NOFIX_SECONDS", "1e9"))
        health = 0 if os.environ.get("SENSOR_FAULT") == "1" else GPS_SENSOR
        link.sys_status_send(GPS_SENSOR, GPS_SENSOR, health, 250, 12100, 3200, 78, 0, 0, 0, 0, 0, 0)
        status_at = float(os.environ.get("STATUS_AT", "45"))
        status_every = float(os.environ.get("STATUS_EVERY", "60"))
        if elapsed >= status_at + sent_status[0] * status_every:
            sent_status[0] += 1
            for severity, text in STATUS_TEXTS:
                link.statustext_send(severity, text.ljust(50, b"\0"))
                print("STATUSTEXT sev=%d %s" % (severity, text.decode()), flush=True)
        link.battery_status_send(
            0, mavlink.MAV_BATTERY_FUNCTION_ALL, mavlink.MAV_BATTERY_TYPE_LIPO,
            2500, [3700, 3690, 3710] + [65535] * 7, 3200, 1800, -1,
            int(os.environ.get("BATT_PCT", "25")), 0,
            int(os.environ.get("BATT_STATE", "0")))
        sticks = [
            int(1500 + 380 * math.sin(elapsed * 0.6 + channel * 1.3))
            for channel in range(8)
        ] if os.environ.get("STILL_STICKS") != "1" else [1500, 1500, 1100, 1500, 1000, 1000, 1000, 1000]
        held = read_sticks()
        if held:
            sticks = [0 if h < 0 else (h or moving) for h, moving in zip(held, sticks)]
        link.rc_channels_send(
            now_ms, 8, *sticks,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            int(os.environ.get("RC_RSSI", "80")))
        vibe = [v if a in VIBRATION_AXES else float("nan") for a, v in zip("xyz", (15.0, 45.0, 75.0))]
        link.vibration_send(int(elapsed * 1e6), *vibe, 0, 3, 12)
        if FLYING_ALTITUDE is not None and not landing:
            target_altitude = FLYING_ALTITUDE + ALTITUDE_DRIFT_M * math.sin(elapsed / 12.0)
        altitude += max(-CLIMB_RATE, min(CLIMB_RATE, target_altitude - altitude))
        if landing and altitude <= GROUND_ALTITUDE:
            landing = False
            armed = False
        airborne = altitude > GROUND_ALTITUDE
        link.extended_sys_state_send(
            mavlink.MAV_VTOL_STATE_UNDEFINED,
            mavlink.MAV_LANDED_STATE_LANDING if landing and airborne
            else mavlink.MAV_LANDED_STATE_IN_AIR if airborne
            else mavlink.MAV_LANDED_STATE_ON_GROUND)
        if not nofix:
            link.global_position_int_send(now_ms, int(lat * 1e7), int(lon * 1e7),
                                          120000, int(altitude * 1000), 300, 0, 0,
                                          int(heading * 100))
        if not nofix and tick % 10 == 0:
            link.home_position_send(int((CENTRE_LAT + CENTRE_SHIFT) * 1e7), int(CENTRE_LON * 1e7), 120000,
                                    0.0, 0.0, 0.0, [1.0, 0.0, 0.0, 0.0], 0.0, 0.0, 0.0,
                                    now_ms * 1000)
        link.gps_raw_int_send(now_ms * 1000, 0 if nofix else 3, int(lat * 1e7), int(lon * 1e7),
                              120000, 120, 120, 350, int(heading * 100) % 36000, 11)
        link.vfr_hud_send(AIRSPEED, GROUND_SPEED, int(heading), 55, altitude, 1.2)
        link.attitude_send(now_ms, 0.02, -0.01, math.radians(heading), 0.0, 0.0, 0.0)

        if tick % 25 == 0:
            print("pos %.5f %.5f hdg %.0f" % (lat, lon, heading), flush=True)

        deadline = time.time() + 0.2
        while time.time() < deadline:
            try:
                data, _ = sock.recvfrom(2048)
            except socket.timeout:
                continue
            for message in parser.parse_buffer(data) or []:
                if not addressed_here(message):
                    continue
                kind = message.get_type()
                if kind == "COMMAND_LONG":
                    cmd = message.command
                    if cmd in (mavlink.MAV_CMD_REQUEST_CAMERA_INFORMATION,
                               mavlink.MAV_CMD_REQUEST_MESSAGE):
                        wanted = int(message.param1)
                        if (cmd == mavlink.MAV_CMD_REQUEST_CAMERA_INFORMATION
                                or wanted == mavlink.MAVLINK_MSG_ID_CAMERA_INFORMATION):
                            asked = message.target_component
                            if asked in links:
                                links[asked].command_ack_send(cmd, mavlink.MAV_RESULT_ACCEPTED)
                                send_camera_information(asked)
                            continue
                    if cmd in (mavlink.MAV_CMD_IMAGE_START_CAPTURE,
                               mavlink.MAV_CMD_VIDEO_START_CAPTURE,
                               mavlink.MAV_CMD_VIDEO_STOP_CAPTURE,
                               mavlink.MAV_CMD_SET_CAMERA_MODE,
                               mavlink.MAV_CMD_REQUEST_CAMERA_SETTINGS):
                        camera.command_ack_send(cmd, mavlink.MAV_RESULT_ACCEPTED)
                        if cmd == mavlink.MAV_CMD_SET_CAMERA_MODE:
                            cam_mode[0] = int(message.param2)
                            print("CAMERA MODE -> %d" % cam_mode[0], flush=True)
                        if cmd == mavlink.MAV_CMD_VIDEO_START_CAPTURE:
                            cam_recording[0] = True
                        if cmd == mavlink.MAV_CMD_VIDEO_STOP_CAPTURE:
                            cam_recording[0] = False
                        if cmd == mavlink.MAV_CMD_IMAGE_START_CAPTURE:
                            print("CAMERA PHOTO TAKEN", flush=True)
                        send_camera_settings()
                        print("CAMERA CMD %d accepted" % cmd, flush=True)
                        continue
                if kind == "RC_CHANNELS_OVERRIDE":
                    active = [(i, getattr(message, "chan%d_raw" % i))
                              for i in range(1, 19)
                              if getattr(message, "chan%d_raw" % i, 0) not in (0, 65535)]
                    print("RC_OVERRIDE %s" % active, flush=True)
                    continue
                if kind == "FILE_TRANSFER_PROTOCOL":
                    request = bytes(message.payload)
                    reply = bytearray(251)
                    seq = int.from_bytes(request[0:2], "little") + 1
                    reply[0:2] = seq.to_bytes(2, "little")
                    reply[2] = request[2]
                    reply[3] = 129            # NAK
                    reply[4] = 1              # one byte of data
                    reply[5] = request[3]     # the opcode being refused
                    reply[12] = 10            # FileNotFound
                    link.file_transfer_protocol_send(0, 255, 0, list(reply))
                    print("FTP NAK for opcode %d" % request[3], flush=True)
                elif kind == "PARAM_REQUEST_LIST":
                    print("PARAM_REQUEST_LIST -> %d params" % len(names), flush=True)
                    for name in names:
                        send_param(name)
                elif kind == "PARAM_REQUEST_READ":
                    wanted = message.param_id.strip("\x00") if isinstance(message.param_id, str) \
                        else message.param_id.decode().strip("\x00")
                    if wanted in params:
                        send_param(wanted)
                    elif 0 <= message.param_index < len(names):
                        send_param(names[message.param_index])
                elif kind == "PARAM_SET":
                    setting = message.param_id.strip("\x00") if isinstance(message.param_id, str) \
                        else message.param_id.decode().strip("\x00")
                    if setting in params:
                        params[setting] = message.param_value
                        print("PARAM_SET %s = %g" % (setting, message.param_value), flush=True)
                        send_param(setting)
                    else:
                        print("PARAM_SET %s (unknown) ignored" % setting, flush=True)
                elif kind == "COMMAND_LONG" and message.command == mavlink.MAV_CMD_COMPONENT_ARM_DISARM:
                    armed = message.param1 > 0.5
                    print("ARM %s (param1=%.1f force=%.1f) from %d/%d" % (
                        "armed" if armed else "DISARMED", message.param1,
                        getattr(message, "param2", 0.0),
                        message.get_srcSystem(), message.get_srcComponent()), flush=True)
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif kind == "MISSION_COUNT":
                    uploading = getattr(message, "mission_type", 0)
                    expected = message.count
                    incoming = {}
                    print("UPLOAD start type=%d count=%d" % (uploading, expected), flush=True)
                    if expected == 0:
                        stored[uploading] = []
                        link.mission_ack_send(255, 0, mavlink.MAV_MISSION_ACCEPTED,
                                              mission_type=uploading)
                    else:
                        link.mission_request_int_send(255, 0, 0, mission_type=uploading)
                elif kind == "MISSION_ITEM_INT":
                    incoming[message.seq] = message
                    if len(incoming) < expected:
                        link.mission_request_int_send(255, 0, len(incoming),
                                                      mission_type=uploading)
                    else:
                        stored[uploading] = [incoming[i] for i in sorted(incoming)]
                        link.mission_ack_send(255, 0, mavlink.MAV_MISSION_ACCEPTED,
                                              mission_type=uploading)
                        print("UPLOAD done type=%d items=%d" % (
                            uploading, len(stored[uploading])), flush=True)
                        for item in stored[uploading]:
                            print("  seq=%d cmd=%d lat=%.7f lon=%.7f alt=%.1f" % (
                                item.seq, item.command, item.x / 1e7, item.y / 1e7, item.z), flush=True)
                elif kind == "MISSION_REQUEST_LIST":
                    kind_of = getattr(message, "mission_type", 0)
                    count = len(stored.get(kind_of, []))
                    print("DOWNLOAD start type=%d count=%d" % (kind_of, count), flush=True)
                    link.mission_count_send(255, 0, count, mission_type=kind_of)
                elif kind in ("MISSION_REQUEST_INT", "MISSION_REQUEST"):
                    kind_of = getattr(message, "mission_type", 0)
                    held = stored.get(kind_of, [])
                    if 0 <= message.seq < len(held):
                        item = held[message.seq]
                        link.mission_item_int_send(
                            255, 0, item.seq, item.frame, item.command,
                            item.current, item.autocontinue,
                            item.param1, item.param2, item.param3, item.param4,
                            item.x, item.y, item.z, kind_of)
                elif kind == "MISSION_ACK":
                    print("DOWNLOAD done", flush=True)
                elif kind == "LOG_REQUEST_LIST":
                    print("LOG_REQUEST_LIST %d..%d" % (message.start, message.end), flush=True)
                    for i, size in enumerate(LOG_SIZES):
                        log_id = i + 1
                        if not (message.start <= log_id <= message.end):
                            continue
                        link.log_entry_send(log_id, len(LOG_SIZES), len(LOG_SIZES),
                                            LOG_BASE_UTC + i * 3600, size)
                elif kind == "LOG_REQUEST_DATA":
                    remaining = LOG_SIZES[message.id - 1] - message.ofs
                    count = max(0, min(90, message.count, remaining))
                    print("LOG_REQUEST_DATA id=%d ofs=%d count=%d -> %d" % (
                        message.id, message.ofs, message.count, count), flush=True)
                    payload = bytes((message.ofs + n) % 251 for n in range(count))
                    link.log_data_send(message.id, message.ofs, count,
                                       list(payload) + [0] * (90 - count))
                elif kind == "LOG_REQUEST_END":
                    print("LOG_REQUEST_END", flush=True)
                elif kind == "LOG_ERASE":
                    print("LOG_ERASE", flush=True)
                elif kind == "SET_POSITION_TARGET_LOCAL_NED":
                    print("ALT CHANGE frame=%d z=%.2f type_mask=0x%X" % (
                        message.coordinate_frame, message.z, message.type_mask), flush=True)
                elif kind == "COMMAND_LONG" and message.command == mavlink.MAV_CMD_NAV_TAKEOFF:
                    print("TAKEOFF alt=%.1f" % message.param7, flush=True)
                    armed = True
                    target_altitude = message.param7 or DEFAULT_TAKEOFF_ALTITUDE
                    landing = False
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif (kind == "COMMAND_LONG"
                        and message.command == mavlink.MAV_CMD_SET_MESSAGE_INTERVAL):
                    asked_id = int(message.param1)
                    INTERVALS[asked_id] = int(message.param2)
                    print("INTERVAL msg=%d us=%d" % (asked_id, INTERVALS[asked_id]), flush=True)
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif (kind == "COMMAND_LONG" and message.command == mavlink.MAV_CMD_REQUEST_MESSAGE
                        and int(message.param1) == mavlink.MAVLINK_MSG_ID_MESSAGE_INTERVAL):
                    asked_id = int(message.param2)
                    link.message_interval_send(asked_id, INTERVALS.get(asked_id, 0))
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                    print("MESSAGE_INTERVAL msg=%d us=%d" % (asked_id, INTERVALS.get(asked_id, 0)), flush=True)
                elif (kind == "COMMAND_LONG" and message.command == mavlink.MAV_CMD_REQUEST_MESSAGE
                        and int(message.param1) == mavlink.MAVLINK_MSG_ID_AUTOPILOT_VERSION):
                    link.autopilot_version_send(
                        mavlink.MAV_PROTOCOL_CAPABILITY_MISSION_FLOAT
                        | mavlink.MAV_PROTOCOL_CAPABILITY_PARAM_FLOAT
                        | mavlink.MAV_PROTOCOL_CAPABILITY_COMMAND_INT
                        | (0 if NO_FENCE else mavlink.MAV_PROTOCOL_CAPABILITY_MISSION_FENCE)
                        | (0 if NO_FENCE else mavlink.MAV_PROTOCOL_CAPABILITY_MISSION_RALLY),
                        FIRMWARE_VERSION, 0, 0, 0,
                        [0] * 8, [0] * 8, [0] * 8, 0, 0, 0, [0] * 18)
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                    print("AUTOPILOT_VERSION sent %s" % FIRMWARE, flush=True)
                elif kind == "COMMAND_LONG" and message.command == mavlink.MAV_CMD_NAV_LAND:
                    print("LAND requested from %.1f m" % altitude, flush=True)
                    landing = True
                    target_altitude = 0.0
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif kind == "COMMAND_LONG" and message.command == mavlink.MAV_CMD_DO_SET_MODE:
                    mode = int(message.param2)
                    print("MODE -> %d" % mode, flush=True)
                    if mode in DESCENDING_MODES:
                        print("LAND requested from %.1f m" % altitude, flush=True)
                        landing = True
                        target_altitude = 0.0
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif (kind == "COMMAND_LONG"
                        and message.command == mavlink.MAV_CMD_PREFLIGHT_CALIBRATION
                        and accelcal[0] is not None
                        and not any(getattr(message, "param%d" % n, 0.0)
                                    for n in range(1, 8))):
                    magcal.command_long_send(255, 0, apm.MAV_CMD_ACCELCAL_VEHICLE_POS, 0,
                                             apm.ACCELCAL_VEHICLE_POS_FAILED, 0, 0, 0, 0, 0, 0)
                    accelcal[0] = None
                    accelcal_sent[0] = None
                    print("ACCEL CAL abandoned", flush=True)
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif (kind == "COMMAND_LONG"
                        and message.command == mavlink.MAV_CMD_PREFLIGHT_CALIBRATION
                        and message.param5 == 1):
                    accelcal[0] = 0
                    print("ACCEL CAL started", flush=True)
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif kind == "COMMAND_ACK" and accelcal[0] is not None:
                    accelcal[0] += 1
                elif kind == "COMMAND_LONG" and message.command == apm.MAV_CMD_DO_START_MAG_CAL:
                    magcal_started[0] = time.time()
                    print("MAG CAL started", flush=True)
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif kind == "COMMAND_LONG" and message.command == apm.MAV_CMD_DO_CANCEL_MAG_CAL:
                    magcal_started[0] = None
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif kind == "COMMAND_LONG" and message.command in TRACKING_COMMANDS:
                    print("CMD %d %s" % (message.command, " ".join(
                        "p%d=%.2f" % (n, getattr(message, "param%d" % n, 0.0))
                        for n in range(1, 8))), flush=True)
                    # The ack has to come from the component the command was addressed to.
                    # Sent from the autopilot it is never matched, and QGC tells the operator
                    # the vehicle did not respond while the camera is visibly tracking.
                    links.get(message.target_component, link).command_ack_send(
                        message.command, mavlink.MAV_RESULT_ACCEPTED)
                elif kind == "COMMAND_LONG":
                    print("CMD %d %s" % (message.command, " ".join(
                        "p%d=%.2f" % (n, getattr(message, "param%d" % n, 0.0))
                        for n in range(1, 8))), flush=True)
                    link.command_ack_send(message.command, mavlink.MAV_RESULT_ACCEPTED)

        for contact in range(ADSB_CONTACTS):
            bearing = 2 * math.pi * contact / max(1, ADSB_CONTACTS) + elapsed * 0.05
            spread = 0.01 + 0.01 * contact
            magcal.adsb_vehicle_send(
                0xABCDE0 + contact,
                int((lat + spread * math.cos(bearing)) * 1e7),
                int((lon + spread * math.sin(bearing)) * 1e7),
                apm.ADSB_ALTITUDE_TYPE_GEOMETRIC,
                int((300 + 150 * contact) * 1000),
                int(((math.degrees(bearing) + 180.0) % 360.0) * 100),
                int(60 * 100), int(2 * 100),
                ("BAW%03d" % contact).encode().ljust(9, b"\0"),
                apm.ADSB_EMITTER_TYPE_LIGHT,
                1,
                apm.ADSB_FLAGS_VALID_COORDS | apm.ADSB_FLAGS_VALID_ALTITUDE
                | apm.ADSB_FLAGS_VALID_HEADING | apm.ADSB_FLAGS_VALID_VELOCITY
                | apm.ADSB_FLAGS_VALID_CALLSIGN | apm.ADSB_FLAGS_VALID_SQUAWK
                | (apm.ADSB_FLAGS_SIMULATED if contact < ADSB_SIMULATED else 0),
                ADSB_SQUAWK if contact == 0 else 1200)

        if ORBIT_RADIUS_M:
            link.file.write(orbit_status_frame(tick, SYSID, ORBIT_RADIUS_M,
                                               CENTRE_LAT + CENTRE_SHIFT, CENTRE_LON, altitude))

        if RADIO_LINK is not None and tick % 5 == 0:
            local = int(RADIO_LINK)
            link.radio_status_send(abs(local), abs(local) - 3, 100, 40, 38, tick % 7, 0)

        if IN_CONTROL is not None and tick % 5 == 0:
            link.file.write(control_status_frame(tick, SYSID, IN_CONTROL))

        if CAMERA_FEEDBACK_EVERY and tick % CAMERA_FEEDBACK_EVERY == 0:
            magcal.camera_feedback_send(
                int(time.time() * 1e6), SYSID, 0, tick // CAMERA_FEEDBACK_EVERY,
                int(lat * 1e7), int(lon * 1e7), altitude + GROUND_ALTITUDE,
                altitude, 0.0, 0.0, heading, 0.0, 0)
            print("CAMERA FEEDBACK %d" % (tick // CAMERA_FEEDBACK_EVERY), flush=True)

        if accelcal[0] is not None and accelcal_sent[0] != accelcal[0]:
            accelcal_sent[0] = accelcal[0]
            position = (ACCELCAL_POSITIONS[accelcal[0]]
                        if accelcal[0] < len(ACCELCAL_POSITIONS)
                        else apm.ACCELCAL_VEHICLE_POS_SUCCESS)
            magcal.command_long_send(255, 0, apm.MAV_CMD_ACCELCAL_VEHICLE_POS, 0,
                                     position, 0, 0, 0, 0, 0, 0)
            print("ACCEL CAL position %d" % position, flush=True)
            if position == apm.ACCELCAL_VEHICLE_POS_SUCCESS:
                accelcal[0] = None
                accelcal_sent[0] = None

        if magcal_started[0] is not None:
            spent = time.time() - magcal_started[0]
            percent = min(100, int(spent * 10))
            for compass in (0, 1):
                magcal.mag_cal_progress_send(
                    compass, MAGCAL_MASK, apm.MAG_CAL_RUNNING_STEP_TWO, 1,
                    percent, [255] * 10, 0.0, 0.0, 0.0)
            if percent >= 100:
                for compass in (0, 1):
                    magcal.mag_cal_report_send(
                        compass, MAGCAL_MASK, apm.MAG_CAL_SUCCESS, 1, 2.5,
                        10.0, -8.0, 4.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0)
                print("MAG CAL reported success", flush=True)
                magcal_started[0] = None

        tick += 1


if __name__ == "__main__":
    main()
