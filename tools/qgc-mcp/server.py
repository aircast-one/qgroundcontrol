#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = ["mcp<2"]
# ///

import json
import os
import pathlib
import plistlib
import queue
import signal
import socket
import subprocess
import threading
import time
import urllib.error
import urllib.request
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Literal, cast
from urllib.parse import parse_qsl, quote, urlencode

from mcp.server.fastmcp import FastMCP, Image

REPO = Path(__file__).resolve().parents[2]
RELEASE_BUILD = REPO / "build-aircast"
TEST_BUILD = REPO / "build-test"
RELEASE_APP = RELEASE_BUILD / "Release/AircastQGC.app/Contents/MacOS/AircastQGC"
TEST_APP = TEST_BUILD / "Debug/AircastQGC.app/Contents/MacOS/AircastQGC"
APP = RELEASE_APP if RELEASE_APP.exists() else TEST_APP
LOG_FILE = Path("/tmp/aircast-qgc-mcp.log")
API_PORT = int(os.environ.get("QGC_DEBUG_API_PORT", "8777"))
TMP_DIR = os.environ.get("TMPDIR", "/tmp")
ANDROID_PKG = "one.aircast.app.core"
DEVICE_API_PORT = 8777
CORE = REPO / "groundstation"
COREHOST = CORE / "target/debug/corehost"
ACTUATOR_GUARD = "QGC_DEBUG_API_ALLOW_ACTUATORS"
MACOS_PROJECT = REPO / "macos"
MACOS_APP = REPO / "build-macos/dd/Build/Products/Debug/AircastCore.app/Contents/MacOS/AircastCore"
IOS_PROJECT = REPO / "ios"
IOS_BUILD = REPO / "build-ios/mcp"
IOS_SIMULATOR = "Aircast MCP"
WEB_PORT = 8800
WEB_ORIGIN = f"http://localhost:{WEB_PORT}"
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
RELAY_CALLS = {"/bridge/get": "get", "/bridge/set": "set", "/bridge/invoke": "invoke"}
MOCK_KEYS = {"arducopter": "apmCopter", "px4": "px4"}
RUNNING: dict = {}

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
        raise RuntimeError(
            f"debug api unreachable ({exc}); start the app with run_app (desktop) or run_app_android"
        ) from exc


def _free_port() -> int:
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def _query(pairs: dict) -> str:
    return urlencode(pairs, quote_via=quote)


def _endpoint_url(endpoint: str) -> str:
    return endpoint if endpoint.startswith("/") else f"/bridge/get?path={quote(endpoint)}"


@mcp.tool()
def read(path: str) -> dict:
    """Read anything the QGC core serves - the same API every app (macOS, iOS, Android, corehost)
    is built on. Works on whichever app the tools are attached to.

    `view.*` paths are typed, ready-made state for one concern; list them with views():
      read("view.flyState")       connected, armed, flying, mode, readyToFly, state
      read("view.guidedActions")  which flight actions are offered now, and why the rest are not
      read("view.battery"), read("view.preflight"), read("view.messages"), read("view.links")
    Views with arguments are called like view.coreParameter(1,RTL_ALT_M).

    Other paths are the raw objects and facts behind them:
      read("vehicle.flightMode"), read("settings.appSettings"),
      read("vehicle.parameterManager.getParameter(-1,RTL_ALT_M)")"""
    return _api(f"/bridge/get?path={quote(path)}")


@mcp.tool()
def write(path: str, value: str | float | bool) -> dict:
    """Write a fact or setting through the QGC core. Strings are JSON (true, 15, "Guided");
    bare text is taken as a string. Answers {ok, reason}.
      write("vehicle.flightMode", "Guided")
      write("vehicle.armed", True)
      write("vehicle.parameterManager.getParameter(-1,RTL_ALT_M)", 20)
      write("settings.appSettings.audioMuted", True)

    Refused while a real vehicle is connected unless the app was started with
    QGC_DEBUG_API_ALLOW_ACTUATORS=1 (run_corehost(actuators=True)) - SITL only, props off.
    A mock_vehicle() never needs it."""
    raw = value if isinstance(value, str) else json.dumps(value)
    return _api(f"/bridge/set?{_query({'path': path, 'value': raw})}")


@mcp.tool()
def invoke(path: str, args: list | None = None) -> dict:
    """Run an action through the QGC core. Answers {ok, reason, refusal, result}; a refused
    action says why in plain words (not armed, too far, another action still running - in that
    case wait_until the state settles and retry).
      vehicle.guidedModeTakeoff [metres]        vehicle.guidedModeLand []
      vehicle.guidedModeRTL [smart]             vehicle.pauseVehicle []
      vehicle.guidedModeChangeAltitude [delta metres, pause]
      vehicle.guidedModeGotoLocation [{"latitude": 47.39, "longitude": 8.54}]
      vehicle.startMission []                   vehicle.emergencyStop []
      vehicle.parameterManager.parameterNames [-1]
    Arm and change mode with write(). Same write guard as write()."""
    return _api(
        f"/bridge/invoke?{_query({'path': path, 'args': json.dumps(args or [], separators=(',', ':'))})}"
    )


@mcp.tool()
def views(filter: str = "") -> list[dict]:
    """Every view the QGC core serves, with the argument shape of those that take arguments.
    `filter` substring-matches the path (case-insensitive). Read one with read()."""
    catalog = _api("/bridge/get?path=view.dependencies")
    shapes = {mode["path"]: mode["arguments"] for mode in catalog.get("argumentModes", [])}
    return [
        {
            "path": view["path"],
            **({"arguments": shapes[view["path"]]} if view["path"] in shapes else {}),
        }
        for view in catalog.get("views", [])
        if filter.lower() in view["path"].lower()
    ]


@mcp.tool()
def attach(port: int) -> dict:
    """Point every tool at the debug API on another local port: an iOS simulator (8799), an
    adb-forwarded phone, a second app started with QGC_DEBUG_API_PORT. Answers its view.flyState."""
    global API_PORT
    API_PORT = port
    return _api("/bridge/get?path=view.flyState")


@mcp.tool()
def mock_vehicle(
    autopilot: Literal["arducopter", "px4"] = "arducopter", options: dict[str, str] | None = None
) -> dict:
    """Connect a simulated vehicle inside the app (corehost and debug builds) - no SITL, and
    write/invoke are allowed against it. Calling it again while one is connected changes its
    conditions instead, through `options`:
      prearm="<reason>" ("" clears)   gps="lost"|"ok"      motor="failed"|"ok"
      wind="<m/s>,<from degrees>"     endurance="<minutes>" home="<lat>,<lon>"
      silent="1"|"0" (telemetry stops: contact lost)
    On start, options also take camera="1", gimbal="1", video="h264"|"rtsp"|..., add="1" for a
    second vehicle. On the web app only the plain start is available."""
    return _api(f"/links/mocklink?{_query({'autopilot': autopilot, **(options or {})})}")


def _env(actuators: bool, **extra: str) -> dict:
    kept = {k: v for k, v in os.environ.items() if k != ACTUATOR_GUARD}
    return kept | ({ACTUATOR_GUARD: "1"} if actuators else {}) | extra


def _stop(name: str) -> bool:
    process = RUNNING.pop(name, None)
    if process is None or process.poll() is not None:
        return False
    os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
    return True


def _start(name: str, argv: list[str], env: dict, log: Path, port: int, timeout_s: int = 60) -> str:
    global API_PORT
    _stop(name)
    log.parent.mkdir(parents=True, exist_ok=True)
    with open(log, "w") as out:
        RUNNING[name] = subprocess.Popen(
            argv, env=env, stdout=out, stderr=out, start_new_session=True
        )
    RUNNING["log"] = log
    API_PORT = port
    if not _wait_for_api(RUNNING[name], timeout_s):
        return f"{name} did not answer on 127.0.0.1:{port}; log at {log}"
    return f"{name} running, debug api on 127.0.0.1:{port}, tools attached; log at {log}"


def _failed(result: subprocess.CompletedProcess) -> str:
    output = result.stdout + result.stderr
    errors = [line for line in output.splitlines() if "error:" in line or "FAILED" in line]
    return "\n".join(errors[-20:]) or output[-3000:]


def _wait_for_api(process: subprocess.Popen | None = None, timeout_s: int = 60) -> bool:
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline and (process is None or process.poll() is None):
        try:
            _api("/bridge/get?path=view.flyState")
            return True
        except RuntimeError:
            time.sleep(0.5)
    return False


@mcp.tool()
def run_corehost(actuators: bool = False) -> str:
    """Start the QGC core with no UI at all and attach every tool to it - the fastest way for
    an agent to fly. Builds it first (incremental), runs it on a free port with throwaway
    settings, so the operator's own settings and apps are untouched. Next, connect a vehicle:
    mock_vehicle() for a simulated one, connect_vehicle() for a SITL.

    actuators=True lets write/invoke act on a real (non-mock) vehicle: SITL only, props off."""
    stop_corehost()
    built = subprocess.run(
        ["cargo", "build", "--bin", "corehost", "--features", "native-host"],
        cwd=CORE,
        capture_output=True,
        text=True,
        timeout=1800,
    )
    if built.returncode != 0:
        return f"build failed:\n{built.stderr[-3000:]}"
    port = _free_port()
    home = Path(TMP_DIR) / f"qgc-corehost-{port}"
    argv = [
        str(COREHOST),
        "--port",
        str(port),
        "--settings",
        str(home / "settings.ini"),
        "--app-name",
        f"corehost-{port}",
        "--map-cache",
        str(home / "maps"),
    ]
    return _start("corehost", argv, _env(actuators), home / "corehost.log", port)


@mcp.tool()
def stop_corehost() -> str:
    """Stop the core started by run_corehost."""
    return "stopped" if _stop("corehost") else "was not running"


@mcp.tool()
def run_app_macos(actuators: bool = False) -> str:
    """Build and (re)start the Qt-free macOS app (Swift over the Rust core, Xcode target
    AircastCore) on a free debug port and attach every tool to it. It keeps its own settings
    ("Aircast QGC MCP"), so the operator's links and preferences are untouched. Needs
    macos/Local.xcconfig, which one `just xcode` writes.

    actuators=True lets write/invoke act on a real (non-mock) vehicle: SITL only, props off."""
    if not (MACOS_PROJECT / "Local.xcconfig").exists():
        return "macos/Local.xcconfig is missing; run `just xcode` once"
    generated = subprocess.run(
        ["xcodegen", "generate", "--quiet"],
        cwd=MACOS_PROJECT,
        env=os.environ | {"QGC_ROOT": str(REPO)},
        capture_output=True,
        text=True,
    )
    if generated.returncode != 0:
        return f"xcodegen failed:\n{_failed(generated)}"
    built = subprocess.run(
        [
            "xcodebuild",
            "-project",
            "AircastQGC.xcodeproj",
            "-scheme",
            "AircastCore",
            "-configuration",
            "Debug",
            "-derivedDataPath",
            str(REPO / "build-macos/dd"),
            "build",
        ],
        cwd=MACOS_PROJECT,
        capture_output=True,
        text=True,
        timeout=3600,
    )
    if built.returncode != 0:
        return f"build failed:\n{_failed(built)}"
    port = _free_port()
    argv = [str(MACOS_APP), "--app-name", "Aircast QGC MCP", "--debug-build"]
    log = Path(TMP_DIR) / "qgc-macos.log"
    return _start("macos", argv, _env(actuators, QGC_DEBUG_API_PORT=str(port)), log, port)


@mcp.tool()
def stop_app_macos() -> str:
    """Stop the macOS app started by run_app_macos."""
    return "stopped" if _stop("macos") else "was not running"


def _runtime_version(runtime: str) -> tuple[int, ...]:
    return tuple(int(part) for part in runtime.rsplit("iOS-", 1)[-1].split("-") if part.isdigit())


def _simulator() -> str:
    listed = json.loads(
        subprocess.run(
            ["xcrun", "simctl", "list", "devices", "available", "-j"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )["devices"]
    mine = [d["udid"] for devices in listed.values() for d in devices if d["name"] == IOS_SIMULATOR]
    if mine:
        return mine[0]
    runtime = max(
        (r for r, devices in listed.items() if ".iOS-" in r and devices),
        key=_runtime_version,
    )
    phone = next(d for d in listed[runtime] if d["name"].startswith("iPhone"))
    created = subprocess.run(
        [
            "xcrun",
            "simctl",
            "create",
            IOS_SIMULATOR,
            phone["deviceTypeIdentifier"],
            runtime,
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    return created.stdout.strip()


@mcp.tool()
def run_app_ios(actuators: bool = False) -> str:
    """Build the iOS app for a simulator of its own ("Aircast MCP", made on first use so other
    sessions' simulators and their debug ports are never touched), install it, (re)start it
    with the debug API on a free port and attach every tool to it. `open -a Simulator` shows it.

    actuators=True lets write/invoke act on a real (non-mock) vehicle: SITL only, props off."""
    steps = [
        ([str(IOS_PROJECT / "fetch-deps.sh")], REPO),
        (["xcodegen", "generate", "--quiet"], IOS_PROJECT),
    ]
    failed = next(
        (
            r
            for r in (subprocess.run(a, cwd=c, capture_output=True, text=True) for a, c in steps)
            if r.returncode
        ),
        None,
    )
    if failed is not None:
        return f"preparing the project failed:\n{_failed(failed)}"
    udid = _simulator()
    subprocess.run(["xcrun", "simctl", "bootstatus", udid, "-b"], capture_output=True, text=True)
    built = subprocess.run(
        [
            "xcodebuild",
            "-project",
            "Aircast.xcodeproj",
            "-scheme",
            "Aircast",
            "-configuration",
            "Debug",
            "-destination",
            f"id={udid}",
            "-derivedDataPath",
            str(IOS_BUILD),
            "build",
        ],
        cwd=IOS_PROJECT,
        capture_output=True,
        text=True,
        timeout=3600,
    )
    if built.returncode != 0:
        return f"build failed:\n{_failed(built)}"
    app = IOS_BUILD / "Build/Products/Debug-iphonesimulator/Aircast.app"
    bundle = plistlib.loads((app / "Info.plist").read_bytes())["CFBundleIdentifier"]
    installed = subprocess.run(
        ["xcrun", "simctl", "install", udid, str(app)], capture_output=True, text=True
    )
    if installed.returncode != 0:
        return f"install failed: {installed.stderr.strip()}"
    port = _free_port()
    log = Path(TMP_DIR) / "qgc-ios.log"
    child = {
        f"SIMCTL_CHILD_{k}": v
        for k, v in _env(actuators, QGC_DEBUG_API_PORT=str(port)).items()
        if k in (ACTUATOR_GUARD, "QGC_DEBUG_API_PORT")
    }
    launched = subprocess.run(
        [
            "xcrun",
            "simctl",
            "launch",
            "--terminate-running-process",
            f"--stdout={log}",
            f"--stderr={log}",
            udid,
            bundle,
        ],
        env=os.environ | child,
        capture_output=True,
        text=True,
    )
    if launched.returncode != 0:
        return f"launch failed: {launched.stderr.strip()}"
    RUNNING["ios"] = (udid, bundle)
    RUNNING["log"] = log
    return _attach_and_wait(port, f"iOS app on simulator {udid}", log)


def _attach_and_wait(port: int, what: str, log: Path) -> str:
    global API_PORT
    API_PORT = port
    if not _wait_for_api(timeout_s=60):
        return f"{what} did not answer on 127.0.0.1:{port}; log at {log}"
    return f"{what} running, debug api on 127.0.0.1:{port}, tools attached; log at {log}"


@mcp.tool()
def stop_app_ios() -> str:
    """Stop the iOS app started by run_app_ios; the simulator stays booted."""
    started = RUNNING.pop("ios", None)
    if started is None:
        return "was not running"
    subprocess.run(["xcrun", "simctl", "terminate", *started], capture_output=True)
    return "stopped"


def _literal(raw: str):
    if raw == "":
        return None
    try:
        return json.loads(raw)
    except ValueError:
        return raw


class _Relay(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, actuators: bool):
        super().__init__(("127.0.0.1", 0), _RelayHandler)
        self.actuators = actuators
        self.requests: queue.Queue = queue.Queue()
        self.waiting: dict = {}

    @property
    def port(self) -> int:
        return self.server_address[1]

    def ask(self, call: str, path: str, payload: str, timeout_s: float = 15) -> str | None:
        ident = uuid.uuid4().hex
        waiter = {"done": threading.Event(), "body": None}
        self.waiting[ident] = waiter
        self.requests.put({"id": ident, "call": call, "path": path, "payload": payload})
        answered = waiter["done"].wait(timeout_s)
        self.waiting.pop(ident, None)
        return waiter["body"] if answered else None

    def answered(self, ident: str, body: str) -> None:
        waiter = self.waiting.get(ident)
        if waiter is not None:
            waiter["body"] = body
            waiter["done"].set()

    def next_request(self, timeout_s: float = 20) -> dict | None:
        deadline = time.monotonic() + timeout_s
        while time.monotonic() < deadline:
            try:
                request = self.requests.get(timeout=max(0.0, deadline - time.monotonic()))
            except queue.Empty:
                return None
            if request["id"] in self.waiting:
                return request
        return None

    def real_vehicle_heard(self) -> bool:
        links = json.loads(self.ask("get", "links.linkConfigurations", "") or "{}")
        return any(
            link.get("heardVehicle") and link.get("class") != "MockConfiguration"
            for link in links.get("elements", [])
        )

    def handle(self, route: str, pairs: dict) -> tuple[int, dict]:
        if route == "/links/mocklink" and pairs.get("autopilot", "").lower() in MOCK_KEYS:
            route, pairs = (
                "/bridge/invoke",
                {
                    "path": "links.startMockLink",
                    "args": json.dumps([MOCK_KEYS[pairs["autopilot"].lower()]]),
                },
            )
        call = RELAY_CALLS.get(route)
        if call is None:
            return 404, {
                "error": f"{route} is not served by the web app; it answers {', '.join(RELAY_CALLS)}"
            }
        path = pairs.get("path", "")
        if not path:
            return 400, {"error": "path is required, e.g. path=settings.appSettings"}
        if call == "set" and "value" not in pairs:
            return 400, {"error": "value is required"}
        if call != "get" and not self.actuators and self.real_vehicle_heard():
            return 400, {
                "error": "bridge writes disabled while a real vehicle is connected; "
                "start it with run_web(actuators=True) (props off!)"
            }
        payload = (
            json.dumps({"value": _literal(pairs["value"])})
            if call == "set"
            else pairs.get("args") or "[]"
        )
        answer = self.ask(call, path, payload)
        if answer is None:
            return 503, {
                "error": "the web app did not answer; is its page still open?",
                "retry": True,
            }
        try:
            return 200, json.loads(answer)
        except ValueError:
            return 200, {"raw": answer}


class _RelayHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass

    @property
    def relay(self) -> _Relay:
        return cast("_Relay", self.server)

    def _send(self, status: int, body: dict | None) -> None:
        text = b"" if body is None else json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Access-Control-Allow-Origin", WEB_ORIGIN)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(text)))
        self.end_headers()
        self.wfile.write(text)

    def do_GET(self):
        route, _, query = self.path.partition("?")
        if route == "/poll":
            if self.headers.get("Origin") != WEB_ORIGIN:
                return self._send(403, {"error": "only the web app's page may poll"})
            request = self.relay.next_request()
            return self._send(204, None) if request is None else self._send(200, request)
        if self.headers.get("X-QGC-Debug-Api") is None:
            return self._send(403, {"error": "missing X-QGC-Debug-Api header"})
        return self._send(*self.relay.handle(route, dict(parse_qsl(query, keep_blank_values=True))))

    def do_POST(self):
        if self.path != "/reply" or self.headers.get("Origin") != WEB_ORIGIN:
            return self._send(403, {"error": "only the web app's page may reply"})
        reply = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))) or "{}")
        self.relay.answered(str(reply.get("id", "")), str(reply.get("body", "")))
        return self._send(204, None)


def _listening(port: int) -> bool:
    try:
        socket.create_connection(("localhost", port), timeout=1).close()
        return True
    except OSError:
        return False


@mcp.tool()
def run_web(headless: bool = True, actuators: bool = False) -> str:
    """Open the web app (the core compiled to wasm, running in the browser) and attach every
    tool to it. Uses the dev server on localhost:8800, starting `just web` when none is
    running (the first wasm build takes minutes), then opens Chrome on a throwaway profile at
    ?debug=<port>. The page relays read/write/invoke to its core through that port; dev builds
    only, as on the native apps. mock_vehicle() starts a simulated vehicle; the debug routes
    for links, missions and mock conditions are native-only.

    headless=False shows the Chrome window. actuators=True lets write/invoke act on a real
    (non-mock) vehicle: SITL only, props off."""
    stop_web()
    if not _listening(WEB_PORT):
        log = Path(TMP_DIR) / "qgc-web-dev.log"
        with open(log, "w") as out:
            RUNNING["vite"] = subprocess.Popen(
                ["just", "web"],
                cwd=REPO,
                stdout=out,
                stderr=out,
                start_new_session=True,
            )
        deadline = time.monotonic() + 1800
        while (
            not _listening(WEB_PORT)
            and RUNNING["vite"].poll() is None
            and time.monotonic() < deadline
        ):
            time.sleep(2)
        if not _listening(WEB_PORT):
            return f"the web dev server did not start; log at {log}"
    relay = _Relay(actuators)
    threading.Thread(target=relay.serve_forever, daemon=True).start()
    RUNNING["relay"] = relay
    profile = Path(TMP_DIR) / f"qgc-web-{relay.port}"
    argv = [
        CHROME,
        f"--user-data-dir={profile}",
        "--no-first-run",
        "--no-default-browser-check",
        *(["--headless=new"] if headless else []),
        f"{WEB_ORIGIN}/?debug={relay.port}",
    ]
    return _start(
        "web",
        argv,
        os.environ.copy(),
        profile / "chrome.log",
        relay.port,
        timeout_s=180,
    )


@mcp.tool()
def stop_web() -> str:
    """Close the browser run_web opened, its relay, and the dev server if run_web started it."""
    closed = _stop("web")
    relay = RUNNING.pop("relay", None)
    if relay is not None:
        relay.shutdown()
        relay.server_close()
    return "stopped" if closed | _stop("vite") | (relay is not None) else "was not running"


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
def wait_until(
    path: str, equals: str, timeout_s: int = 30, endpoint: str = "/status", object_name: str = ""
) -> dict:
    """Poll until a value stringifies to `equals` (case-insensitive), or fail after timeout_s.

    By default polls an endpoint ("/status" or "/vehicle") for a dotted path such as
    "cameras.1.receiving", or "connected" with endpoint="/vehicle". The endpoint can also be a
    core path read through read(): wait_until("state", "flying", endpoint="view.flyState").

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
        status = _api(_endpoint_url(endpoint))
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
    return _api(
        f"/ui/prop?name={quote(object_name)}&property={quote(','.join(properties), safe=',')}"
    )


@mcp.tool()
def ui_set_prop(object_name: str, property: str, value: str) -> dict:
    """Write a QML property, coerced to the type the property already holds ("true", "2.5", text).

    Use it to put the UI into a state that would otherwise take a scripted sequence of gestures
    to reach. Fails rather than guessing if the value will not convert or the property is
    read-only."""
    return _api(
        f"/ui/setprop?name={quote(object_name)}&property={quote(property)}&value={quote(value)}"
    )


@mcp.tool()
def ui_at(x: float, y: float) -> dict:
    """List every QML item under a scene point, outermost first, with geometry and whether each
    is enabled and accepts mouse buttons.

    This is the tool for "the click did nothing": it says whether anything is there at all,
    whether it is enabled, and which item would receive the press - which a screenshot cannot
    tell you and a failed click looks identical to a broken control without."""
    return _api(f"/ui/at?x={x}&y={y}")


@mcp.tool()
def ui_watch(
    object_name: str,
    properties: list[str],
    frames: int = 60,
    interval_ms: int = 8,
    timeout_s: int = 15,
) -> dict:
    """Stream property samples while the UI changes, and return them as a list.

    Each sample carries the app's own timestamp and the frame number, and all the properties
    are read at the same instant. Use it for anything that moves - animations, settling
    layouts, values converging - where polling would sample each property a round trip apart
    and report motion that is really measurement error."""
    if not properties:
        raise ValueError("at least one property required")
    request = (
        f"GET /ui/watch?name={quote(object_name)}&property={quote(','.join(properties), safe=',')}"
        f"&frames={frames}&interval={interval_ms} HTTP/1.1\r\n"
        f"Host: localhost\r\nX-QGC-Debug-Api: 1\r\n\r\n"
    )
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
            except TimeoutError:
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
def click(
    object_name: str = "",
    x: float | None = None,
    y: float | None = None,
    button: Literal["left", "right"] = "left",
) -> dict:
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
def drag(
    from_object: str = "",
    from_x: float | None = None,
    from_y: float | None = None,
    to_x: float = 0,
    to_y: float = 0,
    steps: int = 10,
) -> dict:
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
        raise RuntimeError(
            f"vehicle reports {result['matched']} params, above the 10000 fetch limit; diff would be incomplete"
        )
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
    return {
        "changed": changed,
        "missingOnVehicle": missing,
        "notInPreset": extra,
        "presetCount": len(preset),
        "vehicleCount": len(current),
    }


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
            cameras.append(
                {
                    "path": path,
                    "deviceSource": entry["source"],
                    "rtsp": f"rtsp://{host}:8554/{path}",
                    "whep": f"http://{host}:8889/{path}/whep",
                }
            )
    return {"host": host, "cameras": cameras}


@mcp.tool()
def configure_cameras(
    host: str, paths: list[str] | None = None, names: list[str] | None = None
) -> dict:
    """Configure QGC's cameras from an aircast device: camera 1 = RTSP, extras = WHEP.
    With no `paths`, discovers the device's cameras via its /api/stream/config."""
    if not paths:
        paths = [c["path"] for c in discover_device_cameras(host)["cameras"]]
    if not paths:
        raise ValueError(f"no cameras found on {host}")
    names = names or [f"{p} ({host})" for p in paths]
    result = {}
    result["primary"] = _api(
        f"/video/setting?fact=rtspUrl&value={quote(f'rtsp://{host}:8554/{paths[0]}')}"
    )
    _api(f"/video/setting?fact=videoSource&value={quote('RTSP Video Stream')}")
    _api(f"/video/setting?fact=primaryCameraName&value={quote(names[0])}")
    extras = [
        {
            "name": names[i] if i < len(names) else paths[i],
            "source": "WebRTC (WHEP) Video Stream",
            "url": f"http://{host}:8889/{paths[i]}/whep",
        }
        for i in range(1, len(paths))
    ]
    result["extras"] = _api(
        f"/video/setting?fact=extraVideoSources&value={quote(json.dumps(extras))}"
    )
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


def _signing_identity() -> str:
    override = os.environ.get("QGC_DEV_SIGNING_IDENTITY", "").strip()
    if override:
        return override
    found = subprocess.run(
        ["security", "find-identity", "-v", "-p", "codesigning"],
        capture_output=True,
        text=True,
    )
    quoted = [line.split('"')[1] for line in found.stdout.splitlines() if '"' in line]
    return next((name for name in quoted if name.startswith("Apple Develop")), "")


def _sign(bundle: pathlib.Path) -> str:
    identity = _signing_identity()
    if not identity or not bundle.is_dir():
        return " (unsigned: no codesigning identity)" if bundle.is_dir() else ""
    signed = subprocess.run(
        ["codesign", "--force", "--deep", "--sign", identity, str(bundle)],
        capture_output=True,
        text=True,
        timeout=600,
    )
    if signed.returncode != 0:
        return f" (signing failed: {signed.stderr.strip()[-200:]})"
    return ""


def build(target: str = "release") -> str:
    """Build the app. target: "release" (AircastQGC) or "test" (Debug unit-test build)."""
    cwd = RELEASE_BUILD if target == "release" else TEST_BUILD
    args = ["ninja"] + (["AircastQGC"] if target == "release" else [])
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True, timeout=1800)
    if result.returncode == 0:
        return "build ok" + _sign((RELEASE_APP if target == "release" else TEST_APP).parents[2])
    output = result.stdout + result.stderr
    errors = "\n".join(
        line for line in output.splitlines() if "error" in line.lower() or "FAILED" in line
    )
    return f"build failed:\n{errors or output[-2000:]}"


@mcp.tool()
def native_windows() -> list[dict]:
    """AppKit windows of the running macOS app: title, CGWindowID, frame, and whether the
    window hosts the embedded Qt scene. Use this instead of guessing screen coordinates."""
    return _api("/native/windows").get("windows", [])


@mcp.tool()
def native_screenshot(window: str = "") -> Image:
    """Screenshot one AppKit window by title, captured by window id so it works even when the
    window is behind others. Omit `window` to take the front visible one. For the Qt scene's own
    render surface use `screenshot` instead."""
    windows = _api("/native/windows").get("windows", [])
    visible = [w for w in windows if w.get("visible")]
    if window:
        matches = [w for w in visible if w.get("title") == window]
    else:
        matches = visible
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
def run_app(native_window: bool = False, actuators: bool = False) -> str:
    """(Re)start the desktop Qt app (build-aircast Release, else build-test Debug) with the
    debug API on a free port and attach every tool to it. Only the copy this tool started is
    restarted; the operator's own app keeps running. native_window=True opens the native
    macOS windows instead of the Qt ones.

    actuators=True lets write/invoke act on a real (non-mock) vehicle: SITL only, props off."""
    if not APP.exists():
        return f"no Qt app at {RELEASE_APP} or {TEST_APP}; build one first (just configure && just build)"
    port = _free_port()
    argv = [
        str(APP),
        "--allow-multiple",
        *(["--native-window"] if native_window else []),
    ]
    env = _env(
        actuators,
        QGC_DEBUG_API_PORT=str(port),
        QT_LOGGING_RULES="qgc.videomanager.videomanager.debug=true",
    )
    return _start("qt", argv, env, LOG_FILE, port)


@mcp.tool()
def stop_app() -> str:
    """Stop the Qt app started by run_app."""
    return "stopped" if _stop("qt") else "was not running"


def _adb(serial: str) -> list[str]:
    return ["adb", "-s", serial] if serial else ["adb"]


@mcp.tool()
def run_app_android(serial: str = "", package: str = ANDROID_PKG) -> str:
    """Launch Aircast QGC on an adb-connected Android device, enable its debug API via deep
    link (debug builds only), forward it to a free local port and attach every tool to it.

    serial: adb device serial (e.g. 192.168.0.112:5555); empty uses the only device."""
    global API_PORT
    adb = _adb(serial)
    deeplink = f"aircast-qgc://debug?debug={DEVICE_API_PORT}"
    launch = subprocess.run(
        [*adb, "shell", f"am start -a android.intent.action.VIEW -p {package} -d '{deeplink}'"],
        capture_output=True,
        text=True,
    )
    if launch.returncode != 0:
        return f"launch failed: {launch.stderr or launch.stdout}"

    port = _free_port()
    forward = subprocess.run(
        [*adb, "forward", f"tcp:{port}", f"tcp:{DEVICE_API_PORT}"], capture_output=True, text=True
    )
    if forward.returncode != 0:
        return f"adb forward failed: {forward.stderr or forward.stdout}"
    RUNNING[f"android:{serial}"] = port
    API_PORT = port

    if _wait_for_api(timeout_s=30):
        return f"started on {serial or 'device'}; debug api forwarded to 127.0.0.1:{port}, tools attached"
    return "app launched and forwarded, but debug api did not come up within 30s"


@mcp.tool()
def stop_app_android(serial: str = "", package: str = ANDROID_PKG) -> str:
    """Stop Aircast QGC on the Android device and remove the port forward run_app_android made."""
    adb = _adb(serial)
    subprocess.run([*adb, "shell", "am", "force-stop", package], capture_output=True)
    port = RUNNING.pop(f"android:{serial}", None)
    if port is not None:
        subprocess.run([*adb, "forward", "--remove", f"tcp:{port}"], capture_output=True)
    return "stopped"


@mcp.tool()
def logs(pattern: str = "", lines: int = 50) -> str:
    """Tail the log of the app the last run_* tool started, optionally filtered by a substring."""
    log = RUNNING.get("log", LOG_FILE)
    if not log.exists():
        return "no log file; start an app with a run_* tool first"
    with open(log, "rb") as f:
        f.seek(max(0, log.stat().st_size - 512 * 1024))
        content = f.read().decode(errors="replace").splitlines()
    if pattern:
        content = [line for line in content if pattern in line]
    return "\n".join(content[-lines:]) or "no matching lines"


@mcp.tool()
def run_test(name: str = "", allow_stale: bool = False) -> str:
    """Run QGC unit tests and classify any failures; empty name runs every suite.

    Refuses to run a binary older than the sources unless allow_stale is set.
    Failing suites are re-run in isolation and reported as REAL (fails every
    time), UNSTABLE (fails some isolated runs) or a LOAD FLAKE (passes alone).
    """
    args = ["python3", str(REPO / "tools/run-tests.py")]
    if name:
        args.append(name)
    if allow_stale:
        args.append("--allow-stale")
    result = subprocess.run(args, capture_output=True, text=True, timeout=3600)
    return (result.stdout + result.stderr).strip() or "no output"


@mcp.tool()
def test_history() -> str:
    """Show which test suites have been flaking, across recorded runs of run_test."""
    args = ["python3", str(REPO / "tools/run-tests.py"), "--history"]
    result = subprocess.run(args, capture_output=True, text=True, timeout=120)
    return (result.stdout + result.stderr).strip() or "no history yet"


if __name__ == "__main__":
    mcp.run()
