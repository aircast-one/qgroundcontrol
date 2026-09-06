#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = ["mcp<2"]
# ///
"""MCP server for building, running and driving AircastQGC.

App-control tools talk to the localhost debug API that AircastQGC exposes.
On desktop, run_app launches it with QGC_DEBUG_API_PORT set. On Android,
run_app_android launches it, enables the debug API via the aircast-qgc:// deep
link, and adb-forwards the port to localhost so the same tools apply.
"""

import json
import os
import pathlib
from urllib.parse import quote
import socket
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Literal

from mcp.server.fastmcp import FastMCP
from mcp.server.fastmcp import Image

REPO = Path(__file__).resolve().parents[2]
RELEASE_BUILD = REPO / "build-aircast"
TEST_BUILD = REPO / "build-test"
RELEASE_APP = RELEASE_BUILD / "Release/AircastQGC.app/Contents/MacOS/AircastQGC"
TEST_APP = TEST_BUILD / "Debug/AircastQGC.app/Contents/MacOS/AircastQGC"
APP = RELEASE_APP if RELEASE_APP.exists() else TEST_APP
LOG_FILE = Path("/tmp/aircast-qgc-mcp.log")
API_PORT = 8777
TMP_DIR = os.environ.get("TMPDIR", "/tmp")
ANDROID_PKG = "org.mavlink.qgroundcontrol"

mcp = FastMCP("aircast-qgc")


def _api(path: str) -> dict:
    request = urllib.request.Request(
        f"http://127.0.0.1:{API_PORT}{path}",
        headers={"X-QGC-Debug-Api": "1"},
    )
    try:
        with urllib.request.urlopen(request, timeout=5) as resp:
            return json.load(resp)
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode(errors="replace")
        raise RuntimeError(f"debug api returned {exc.code}: {detail}") from exc
    except OSError as exc:
        raise RuntimeError(f"debug api unreachable ({exc}); start the app with run_app (desktop) or run_app_android") from exc


@mcp.tool()
def app_status() -> dict:
    """Video state of the running app: active camera, per-camera name/status, decoding/streaming/recording."""
    return _api("/status")


@mcp.tool()
def switch_camera(index: int | None = None) -> dict:
    """Switch the active camera (to `index`, or cycle to the next camera when omitted). Returns the new status."""
    return _api(f"/switch?index={index}" if index is not None else "/switch")


@mcp.tool()
def grab_image() -> dict:
    """Save a snapshot of the active camera stream; returns the image file path."""
    return _api("/grab")


@mcp.tool()
def screenshot() -> Image:
    """Screenshot of the running app's main window, returned as an image."""
    result = _api("/screenshot")
    if "imageFile" not in result:
        raise RuntimeError(f"screenshot failed: {result}")
    return Image(path=result["imageFile"])


@mcp.tool()
def vehicle_status() -> dict:
    """Telemetry of the connected vehicle: armed, flight mode, position, speed, GPS, battery."""
    return _api("/vehicle")


@mcp.tool()
def record(on: bool) -> dict:
    """Start or stop video recording of all active streams. Returns the new status."""
    return _api(f"/record?on={1 if on else 0}")


@mcp.tool()
def wait_until(path: str, equals: str, timeout_s: int = 30, endpoint: str = "/status",
               object_name: str = "") -> dict:
    """Poll until a value stringifies to `equals` (case-insensitive), or fail after timeout_s.

    By default polls an endpoint ("/status" or "/vehicle") for a dotted path such as
    "cameras.1.receiving", or "connected" with endpoint="/vehicle".

    With object_name set, polls a QML property on that item instead and `path` is the property
    name - so waiting for a panel to finish opening is wait_until("_morph", "1",
    object_name="indicatorDrawer") rather than a sleep long enough to cover the worst case."""
    deadline = time.monotonic() + timeout_s
    poll = 0.05 if object_name else 1
    while True:
        if object_name:
            status = _api(f"/ui/prop?name={quote(object_name)}&property={quote(path)}")
            value = status.get("values", {}).get(path)
            if str(value).lower() == equals.lower():
                return status
            if time.monotonic() > deadline:
                raise RuntimeError(f"timeout: {object_name}.{path} is {value!r}, wanted {equals!r}")
            time.sleep(poll)
            continue
        status = _api(endpoint)
        value = status
        try:
            for key in path.split("."):
                value = value[int(key)] if isinstance(value, list) else value[key]
        except (KeyError, IndexError, ValueError) as exc:
            raise RuntimeError(f"bad path {path!r}: {exc}") from exc
        if str(value).lower() == equals.lower():
            return status
        if time.monotonic() > deadline:
            raise RuntimeError(f"timeout: {path} is {value!r}, wanted {equals!r}")
        time.sleep(poll)


@mcp.tool()
def ui_tree(filter: str = "", all: bool = False, visible_only: bool = False) -> dict:
    """List QML items in the running app (objectName, class, scene geometry, visibility).
    `filter` substring-matches objectName, or class name when all=True.

    all=True also returns items with no objectName, reported by class. Use it when a click
    lands on nothing and you need to find what is actually there - anything the author did
    not think to name is invisible to the default listing, which is exactly when you need it."""
    query = f"/ui/tree?name={quote(filter)}"
    if all:
        query += "&all=1"
    if visible_only:
        query += "&visible=1"
    return _api(query)


@mcp.tool()
def ui_prop(object_name: str, properties: list[str]) -> dict:
    """Read one or more QML properties from an item, as a single sample.

    Returns {"t": <app timestamp>, "values": {...}}. Reading several properties in one call
    matters whenever they change together: fetched over separate calls they drift apart by
    however long each round trip took, which on a fast animation reads as two properties
    disagreeing when they do not."""
    if not properties:
        raise ValueError("at least one property required")
    return _api(f"/ui/prop?name={quote(object_name)}&property={quote(','.join(properties), safe=',')}")


@mcp.tool()
def ui_set_prop(object_name: str, property: str, value: str) -> dict:
    """Write a QML property, coerced to the type the property already holds ("true", "2.5", text).

    Use it to put the UI into a state that would otherwise take a scripted sequence of gestures
    to reach. Fails rather than guessing if the value will not convert or the property is
    read-only."""
    return _api(f"/ui/setprop?name={quote(object_name)}&property={quote(property)}&value={quote(value)}")


@mcp.tool()
def ui_at(x: float, y: float) -> dict:
    """List every QML item under a scene point, outermost first, with geometry and whether each
    is enabled and accepts mouse buttons.

    This is the tool for "the click did nothing": it says whether anything is there at all,
    whether it is enabled, and which item would receive the press - which a screenshot cannot
    tell you and a failed click looks identical to a broken control without."""
    return _api(f"/ui/at?x={x}&y={y}")


@mcp.tool()
def ui_watch(object_name: str, properties: list[str], frames: int = 60,
             interval_ms: int = 8, timeout_s: int = 15) -> dict:
    """Stream property samples while the UI changes, and return them as a list.

    Each sample carries the app's own timestamp and the frame number, and all the properties
    are read at the same instant. Use it for anything that moves - animations, settling
    layouts, values converging - where polling would sample each property a round trip apart
    and report motion that is really measurement error."""
    if not properties:
        raise ValueError("at least one property required")
    request = (f"GET /ui/watch?name={quote(object_name)}&property={quote(','.join(properties), safe=',')}"
               f"&frames={frames}&interval={interval_ms} HTTP/1.1\r\n"
               f"Host: localhost\r\nX-QGC-Debug-Api: 1\r\n\r\n")
    try:
        connection = socket.create_connection(("127.0.0.1", API_PORT), timeout=5)
    except OSError as exc:
        raise RuntimeError(f"debug api unreachable ({exc}); start the app with run_app") from exc

    samples: list[dict] = []
    with connection:
        connection.sendall(request.encode())
        connection.settimeout(0.5)
        buffered = b""
        head_done = False
        deadline = time.monotonic() + timeout_s
        while time.monotonic() < deadline:
            try:
                chunk = connection.recv(65536)
            except socket.timeout:
                continue
            except OSError:
                break
            if not chunk:
                break
            buffered += chunk
            if not head_done and b"\r\n\r\n" in buffered:
                head, _, buffered = buffered.partition(b"\r\n\r\n")
                if b" 200 " not in head.split(b"\r\n")[0]:
                    raise RuntimeError(head.decode(errors="replace"))
                head_done = True
            while b"\n" in buffered:
                line, _, buffered = buffered.partition(b"\n")
                if line.strip():
                    samples.append(json.loads(line))
    return {"count": len(samples), "samples": samples}


@mcp.tool()
def click(object_name: str = "", x: float | None = None, y: float | None = None,
          button: Literal["left", "right"] = "left") -> dict:
    """Click a QML item by objectName (its visible instance, at its center), or raw scene
    coordinates when x/y are given instead. Right-click opens context actions such as the
    telemetry bar's edit mode."""
    if object_name:
        return _api(f"/ui/click?name={quote(object_name)}&button={quote(button)}")
    if x is None or y is None:
        raise ValueError("object_name or x/y required")
    return _api(f"/ui/click?x={x}&y={y}&button={quote(button)}")


@mcp.tool()
def double_click(object_name: str = "", x: float | None = None, y: float | None = None) -> dict:
    """Double-click a QML item by objectName or scene coordinates (e.g. toggles video fullscreen)."""
    if object_name:
        return _api(f"/ui/doubleclick?name={object_name}")
    if x is None or y is None:
        raise ValueError("object_name or x/y required")
    return _api(f"/ui/doubleclick?x={x}&y={y}")


@mcp.tool()
def hover(object_name: str = "", x: float | None = None, y: float | None = None) -> dict:
    """Move the mouse over an item or point without clicking — reveals hover-gated UI
    (resize handles, PIP buttons) so it can be inspected or clicked next."""
    if object_name:
        return _api(f"/ui/hover?name={object_name}")
    if x is None or y is None:
        raise ValueError("object_name or x/y required")
    return _api(f"/ui/hover?x={x}&y={y}")


@mcp.tool()
def drag(from_object: str = "", from_x: float | None = None, from_y: float | None = None,
         to_x: float = 0, to_y: float = 0, steps: int = 10) -> dict:
    """Press-move-release drag from an item's center (or from_x/from_y) to scene
    coordinates to_x/to_y. Drives the draggable PIP, tiles and panels."""
    if from_object:
        src = f"name={from_object}"
    elif from_x is not None and from_y is not None:
        src = f"x={from_x}&y={from_y}"
    else:
        raise ValueError("from_object or from_x/from_y required")
    return _api(f"/ui/drag?{src}&toX={to_x}&toY={to_y}&steps={steps}")


@mcp.tool()
def type_text(text: str) -> dict:
    """Type text into whatever has keyboard focus (click a field first)."""
    return _api(f"/ui/type?text={quote(text)}")


@mcp.tool()
def key(combo: str) -> dict:
    """Press a key or shortcut by name: Enter, Esc, Tab, Backspace, Ctrl+A, ..."""
    return _api(f"/ui/key?key={quote(combo)}")


@mcp.tool()
def connect_vehicle(host: str = "sitl.aircast.one", port: int = 5760, name: str = "") -> dict:
    """Create (or reuse) a TCP MAVLink link to a vehicle/SITL and connect it. Hostnames are
    resolved. Follow with wait_until connected=true on the /vehicle endpoint."""
    address = socket.gethostbyname(host)
    suffix = f"&name={quote(name)}" if name else ""
    return _api(f"/links/connect?host={quote(address)}&port={port}{suffix}")


@mcp.tool()
def list_links() -> dict:
    """Configured comm links and their connection state."""
    return _api("/links")


@mcp.tool()
def disconnect_vehicle(name: str = "") -> dict:
    """Disconnect a comm link by name, or all links when name is empty."""
    return _api(f"/links/disconnect?name={quote(name)}" if name else "/links/disconnect")


@mcp.tool()
def get_params(filter: str = "", limit: int = 100) -> dict:
    """List autopilot parameters (name -> value) matching a substring filter.
    Requires a connected vehicle with parameters loaded."""
    return _api(f"/vehicle/params?filter={quote(filter)}&limit={limit}")


@mcp.tool()
def get_param(name: str) -> dict:
    """One autopilot parameter with metadata: value, units, min/max, description."""
    return _api(f"/vehicle/params?name={quote(name)}")


@mcp.tool()
def set_param(name: str, value: str) -> dict:
    """Write an autopilot parameter (sent to the vehicle immediately). Returns the new value.
    Tune carefully: changes take effect on the flight controller."""
    return _api(f"/vehicle/params/set?name={quote(name)}&value={quote(value)}")


@mcp.tool()
def params_save(file: str = "") -> dict:
    """Save all vehicle parameters to a QGC .params file (default: temp path). Use as a
    golden preset for fleet provisioning or as a backup before tuning."""
    return _api(f"/vehicle/params/save?file={quote(file)}" if file else "/vehicle/params/save")


@mcp.tool()
def params_load(file: str) -> dict:
    """Load a .params file and send every differing parameter to the vehicle. This is the
    fleet-provisioning path: flash a known-good preset onto a new airframe."""
    return _api(f"/vehicle/params/load?file={quote(file)}")


@mcp.tool()
def params_diff(file: str) -> dict:
    """Diff the vehicle's current parameters against a saved .params preset. Returns
    changed (name: preset vs current), missing, and extra parameter names."""
    preset = {}
    for line in pathlib.Path(file).read_text(errors="replace").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) >= 4:
            preset[parts[2]] = parts[3]
    result = _api("/vehicle/params?limit=10000")
    if result.get("truncated"):
        raise RuntimeError(f"vehicle reports {result['matched']} params, above the 10000 fetch limit; diff would be incomplete")
    current = result["params"]
    changed = {}
    for name, value in preset.items():
        if name in current:
            try:
                same = abs(float(value) - float(current[name])) < 1e-6
            except ValueError:
                same = str(value) == str(current[name])
            if not same:
                changed[name] = {"preset": value, "current": current[name]}
    missing = sorted(set(preset) - set(current))
    extra = sorted(set(current) - set(preset))
    return {"changed": changed, "missingOnVehicle": missing, "notInPreset": extra,
            "presetCount": len(preset), "vehicleCount": len(current)}


SAFETY_CHECKS = [
    ("ARMING_CHECK", lambda v: float(v) != 0, "arming checks must not be disabled (0)"),
    ("FS_THR_ENABLE", lambda v: float(v) != 0, "RC failsafe must be enabled"),
    ("BATT_FS_LOW_ACT", lambda v: float(v) != 0, "battery low failsafe action must be set"),
    ("BATT_LOW_VOLT", lambda v: float(v) > 0, "battery low voltage threshold must be set"),
    ("RTL_ALT", lambda v: 500 <= float(v) <= 10000, "RTL altitude should be 5-100m (cm units)"),
    ("FS_GCS_ENABLE", lambda v: True, "GCS failsafe (informational)"),
    ("FENCE_ENABLE", lambda v: True, "geofence (informational)"),
]


@mcp.tool()
def safety_audit() -> dict:
    """Read-only failsafe/safety audit of the connected vehicle's parameters (ArduPilot).
    Reports pass/fail per check plus the raw values, for pre-flight sign-off."""
    results = []
    for name, check, rule in SAFETY_CHECKS:
        entry = {"param": name, "rule": rule}
        try:
            value = _api(f"/vehicle/params?name={quote(name)}")["value"]
            entry["value"] = value
            entry["ok"] = bool(check(value))
        except Exception as exc:
            entry["ok"] = None
            entry["note"] = f"not readable: {exc}"
        results.append(entry)
    failed = [r["param"] for r in results if r["ok"] is False]
    return {"verdict": "FAIL" if failed else "PASS", "failed": failed, "checks": results}


@mcp.tool()
def calibrate(type: str, stop: bool = False) -> dict:
    """Start (or stop) a sensor calibration: accel, mag, gyro, level, radio, esc.
    Accel/mag need the operator to physically move the airframe — follow the prompts
    from vehicle_messages()."""
    if stop:
        return _api("/vehicle/calibrate?stop=1")
    return _api(f"/vehicle/calibrate?type={quote(type)}")


@mcp.tool()
def vehicle_messages() -> dict:
    """Recent STATUSTEXT messages from the vehicle (calibration prompts, arming denials,
    failsafe warnings)."""
    return _api("/vehicle/messages")


@mcp.tool()
def rc_channels() -> dict:
    """Live RC channel PWM values — verify transmitter sticks and switches."""
    return _api("/vehicle/rc")


@mcp.tool()
def motor_test(motor: int, percent: int = 10, timeout_s: int = 2) -> dict:
    """Spin one motor (1-based) at percent throttle for timeout_s seconds, for
    order/direction bench checks. PROPS OFF. Requires the app to be launched with
    QGC_DEBUG_API_ALLOW_ACTUATORS=1, otherwise refused."""
    return _api(f"/vehicle/motortest?motor={motor}&percent={percent}&timeout={timeout_s}")


@mcp.tool()
def mission_upload(file: str) -> dict:
    """Load a .plan file and send the mission to the vehicle."""
    return _api(f"/mission/upload?file={quote(file)}")


@mcp.tool()
def mission_download(file: str) -> dict:
    """Download the mission from the vehicle and save it as a .plan file (async: poll for
    the file's existence)."""
    return _api(f"/mission/download?file={quote(file)}")


@mcp.tool()
def discover_device_cameras(host: str) -> dict:
    """Query an aircastd device for its configured camera paths (via /api/stream/config)
    and the stream URLs QGC would use for each."""
    with urllib.request.urlopen(f"http://{host}/api/stream/config", timeout=10) as resp:
        config = json.load(resp)
    cameras = []
    for path, entry in (config.get("paths") or {}).items():
        if entry and entry.get("source"):
            cameras.append({
                "path": path,
                "deviceSource": entry["source"],
                "rtsp": f"rtsp://{host}:8554/{path}",
                "whep": f"http://{host}:8889/{path}/whep",
            })
    return {"host": host, "cameras": cameras}


@mcp.tool()
def configure_cameras(host: str, paths: list[str] | None = None, names: list[str] | None = None) -> dict:
    """Configure QGC's cameras from an aircast device: camera 1 = RTSP, extras = WHEP.
    With no `paths`, discovers the device's cameras via its /api/stream/config."""
    if not paths:
        paths = [c["path"] for c in discover_device_cameras(host)["cameras"]]
    if not paths:
        raise ValueError(f"no cameras found on {host}")
    names = names or [f"{p} ({host})" for p in paths]
    result = {}
    result["primary"] = _api(f"/video/setting?fact=rtspUrl&value={quote(f'rtsp://{host}:8554/{paths[0]}')}")
    _api(f"/video/setting?fact=videoSource&value={quote('RTSP Video Stream')}")
    _api(f"/video/setting?fact=primaryCameraName&value={quote(names[0])}")
    extras = [
        {"name": names[i] if i < len(names) else paths[i],
         "source": "WebRTC (WHEP) Video Stream",
         "url": f"http://{host}:8889/{paths[i]}/whep"}
        for i in range(1, len(paths))
    ]
    result["extras"] = _api(f"/video/setting?fact=extraVideoSources&value={quote(json.dumps(extras))}")
    return result


@mcp.tool()
def set_logging(rules: str) -> dict:
    """Change logging rules at runtime without restarting, e.g.
    "qgc.videomanager.videoreceiver.gstreamer.gstvideoreceiver.debug=true".
    Separate multiple rules with ';'. Output lands in the run_app log file."""
    return _api(f"/logging?rules={quote(rules)}")


@mcp.tool()
def get_video_settings() -> dict:
    """All video settings facts and their current values."""
    return _api("/video/setting")


@mcp.tool()
def set_video_setting(fact: str, value: str) -> dict:
    """Set a video settings fact by name (e.g. multiViewEnabled=true, rtspUrl=rtsp://...,
    extraVideoSources=<json array>). Applies live — streams reconfigure immediately."""
    return _api(f"/video/setting?fact={fact}&value={quote(value)}")


@mcp.tool()
def build(target: str = "release") -> str:
    """Build the app. target: "release" (AircastQGC) or "test" (Debug unit-test build)."""
    cwd = RELEASE_BUILD if target == "release" else TEST_BUILD
    args = ["ninja"] + (["AircastQGC"] if target == "release" else [])
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True, timeout=1800)
    if result.returncode == 0:
        return "build ok"
    output = result.stdout + result.stderr
    errors = "\n".join(line for line in output.splitlines() if "error" in line.lower() or "FAILED" in line)
    return f"build failed:\n{errors or output[-2000:]}"


@mcp.tool()
def native_windows() -> list[dict]:
    """AppKit windows of the running macOS app: title, CGWindowID, frame, and whether the
    window hosts the embedded Qt scene. Use this instead of guessing screen coordinates."""
    return _api("/native/windows").get("windows", [])


@mcp.tool()
def native_screenshot(window: str = "") -> Image:
    """Screenshot one AppKit window by title, captured by window id so it works even when the
    window is behind others. Omit `window` to take the one hosting Qt. For the Qt scene's own
    render surface use `screenshot` instead."""
    windows = _api("/native/windows").get("windows", [])
    visible = [w for w in windows if w.get("visible")]
    if window:
        matches = [w for w in visible if w.get("title") == window]
    else:
        matches = [w for w in visible if w.get("hostsQt")] or visible
    if not matches:
        titles = ", ".join(repr(w.get("title", "")) for w in visible) or "none"
        raise RuntimeError(f"no visible window {window!r}; visible windows: {titles}")

    target = Path(TMP_DIR) / f"qgc-native-{matches[0]['number']}.png"
    subprocess.run(
        ["screencapture", "-x", "-o", "-l", str(matches[0]["number"]), str(target)],
        check=True,
        capture_output=True,
    )
    if not target.exists():
        raise RuntimeError("screencapture produced no file; is Screen Recording permitted?")
    return Image(path=str(target))


@mcp.tool()
def native_probe(id: str = "", action: str = "", **args: str) -> dict:
    """Drive the native macOS UI by identity rather than by synthesised clicks.

    This is the only thing that works when the screen is locked: no window can become
    key, so clicks land nowhere (and look like they passed). Screenshots keep working.

    Omit `id` to list probes and see whether the screen is locked. Give `id` alone to
    read that probe's state. Give `id` and `action` plus keyword arguments to drive it,
    e.g. native_probe("settings", "select", page="Connections") or
    native_probe("links", "connect", index="4")."""
    query = "&".join(
        f"{quote(str(k))}={quote(str(v))}"
        for k, v in (("id", id), ("action", action), *args.items())
        if v != ""
    )
    return _api(f"/native/probe?{query}" if query else "/native/probe")


@mcp.tool()
def native_click(window: str, x: float, y: float) -> dict:
    """Click an AppKit window at content-view coordinates with a top-left origin.

    Refuses when the screen is locked, because a window that cannot become key cannot
    receive the click. Use native_probe there instead."""
    return _api(f"/native/click?window={quote(window)}&x={x}&y={y}")


@mcp.tool()
def native_menu() -> list[dict]:
    """The app's menu bar as a tree of titles, key equivalents and enabled state."""
    return _api("/native/menu").get("menu", [])


@mcp.tool()
def native_menu_invoke(path: str) -> dict:
    """Fire a menu item by slash-separated title path, e.g. "Window/Native Telemetry"."""
    return _api(f"/native/menu/invoke?path={quote(path)}")


@mcp.tool()
def bridge_stats() -> dict:
    """Round-trip statistics for the Swift-to-Qt reflection bridge: call count, last, worst
    and mean milliseconds."""
    return _api("/native/bridge")


@mcp.tool()
def run_app() -> str:
    """(Re)start the release app with the debug API and video-manager logging enabled."""
    subprocess.run(["pkill", "-9", "-x", "AircastQGC"], capture_output=True)
    env = os.environ | {
        "QGC_DEBUG_API_PORT": str(API_PORT),
        "QT_LOGGING_RULES": "qgc.videomanager.videomanager.debug=true",
    }
    with open(LOG_FILE, "w") as log:
        subprocess.Popen([str(APP)], env=env, stdout=log, stderr=log, start_new_session=True)
    return f"started; debug api on 127.0.0.1:{API_PORT}, log at {LOG_FILE}"


@mcp.tool()
def stop_app() -> str:
    """Stop the running app."""
    result = subprocess.run(["pkill", "-9", "-x", "AircastQGC"], capture_output=True)
    return "stopped" if result.returncode == 0 else "was not running"


def _adb(serial: str) -> list[str]:
    return ["adb", "-s", serial] if serial else ["adb"]


@mcp.tool()
def run_app_android(serial: str = "") -> str:
    """Launch AircastQGC on an adb-connected Android device, enable its debug API via deep
    link, and forward it to localhost so every other tool works against the device.

    serial: adb device serial (e.g. 192.168.0.112:5555); empty uses the only device."""
    adb = _adb(serial)
    deeplink = f"aircast-qgc://debug?debug={API_PORT}"
    launch = subprocess.run(
        [*adb, "shell", f"am start -a android.intent.action.VIEW -d '{deeplink}'"],
        capture_output=True, text=True)
    if launch.returncode != 0:
        return f"launch failed: {launch.stderr or launch.stdout}"

    forward = subprocess.run(
        [*adb, "forward", f"tcp:{API_PORT}", f"tcp:{API_PORT}"], capture_output=True, text=True)
    if forward.returncode != 0:
        return f"adb forward failed: {forward.stderr or forward.stdout}"

    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        try:
            _api("/status")
            return f"started on {serial or 'device'}; debug api forwarded to 127.0.0.1:{API_PORT}"
        except RuntimeError:
            time.sleep(1)
    return "app launched and forwarded, but debug api did not come up within 30s"


@mcp.tool()
def stop_app_android(serial: str = "") -> str:
    """Stop AircastQGC on the Android device and remove the port forward."""
    adb = _adb(serial)
    subprocess.run([*adb, "shell", "am", "force-stop", ANDROID_PKG], capture_output=True)
    subprocess.run([*adb, "forward", "--remove", f"tcp:{API_PORT}"], capture_output=True)
    return "stopped"


@mcp.tool()
def logs(pattern: str = "", lines: int = 50) -> str:
    """Tail the app log started by run_app, optionally filtered by a substring."""
    if not LOG_FILE.exists():
        return "no log file; start the app with run_app first"
    with open(LOG_FILE, "rb") as f:
        f.seek(max(0, LOG_FILE.stat().st_size - 512 * 1024))
        content = f.read().decode(errors="replace").splitlines()
    if pattern:
        content = [line for line in content if pattern in line]
    return "\n".join(content[-lines:]) or "no matching lines"


@mcp.tool()
def run_test(name: str = "") -> str:
    """Run a QGC unit test suite by name (e.g. PipViewTest, DragToPositionTest, VideoManagerTest); empty runs all suites."""
    args = [str(TEST_APP), "--allow-multiple", f"--unittest:{name}" if name else "--unittest"]
    result = subprocess.run(args, capture_output=True, text=True, timeout=1800)
    interesting = [line for line in result.stdout.splitlines()
                   if any(key in line for key in ("PASS", "FAIL", "Totals", "TESTS"))]
    return "\n".join(interesting) or result.stdout[-2000:]


if __name__ == "__main__":
    mcp.run()
