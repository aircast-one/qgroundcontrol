#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = ["mcp<2"]
# ///

import importlib.util
import json
import sys
import threading
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

SERVER_PY = Path(__file__).with_name("server.py")

REQUESTS: list[str] = []
RESPONSES: dict[str, dict] = {}


class _Stub(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        REQUESTS.append(self.path)
        if self.headers.get("X-QGC-Debug-Api") is None:
            self._send(403, {"error": "missing header"})
            return
        route = self.path.split("?")[0]
        if route == "/ui/watch":
            self._stream()
            return
        if route in RESPONSES:
            self._send(200, RESPONSES[route])
            return
        self._send(400, {"error": f"unstubbed {route}"})

    def _send(self, code: int, payload: dict):
        body = json.dumps(payload).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _stream(self):
        self.send_response(200)
        self.send_header("Content-Type", "application/x-ndjson")
        self.end_headers()
        for i in range(4):
            line = json.dumps({"t": 1000 + i, "frame": i, "values": {"width": 10 + i}})
            self.wfile.write(line.encode() + b"\n")
            self.wfile.flush()


def _load(port: int):
    spec = importlib.util.spec_from_file_location("qgcmcp_under_test", SERVER_PY)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.API_PORT = port
    return module


def _tool(module, name):
    attribute = getattr(module, name)
    return attribute.fn if hasattr(attribute, "fn") else attribute


FAILURES: list[str] = []


def check(condition: bool, label: str, detail: str = ""):
    if condition:
        print(f"  PASS  {label}")
    else:
        FAILURES.append(f"{label}  {detail}")
        print(f"  FAIL  {label}  {detail}")


def last_request() -> str:
    return REQUESTS[-1]


def _page(relay_port: int, origin: str, core: dict, stop: threading.Event) -> None:
    base = f"http://127.0.0.1:{relay_port}"
    while not stop.is_set():
        with urllib.request.urlopen(
            urllib.request.Request(f"{base}/poll", headers={"Origin": origin})
        ) as polled:
            if polled.status != 200:
                continue
            request = json.load(polled)
        core["seen"].append(request)
        body = json.dumps(core["answers"].get(request["path"], {"ok": True}))
        reply = json.dumps({"id": request["id"], "body": body}).encode()
        urllib.request.urlopen(
            urllib.request.Request(f"{base}/reply", data=reply, headers={"Origin": origin})
        )


def _relay_checks(module) -> None:
    print("web relay")
    relay = module._Relay(actuators=False)
    threading.Thread(target=relay.serve_forever, daemon=True).start()
    core = {
        "seen": [],
        "answers": {
            "view.flyState": {"state": "flying"},
            "links.linkConfigurations": {
                "elements": [{"class": "MockConfiguration", "heardVehicle": True}]
            },
        },
    }
    stop = threading.Event()
    threading.Thread(
        target=_page, args=(relay.port, module.WEB_ORIGIN, core, stop), daemon=True
    ).start()
    module.API_PORT = relay.port

    state = _tool(module, "read")("view.flyState")
    check(state == {"state": "flying"}, "a read comes back from the page's core", json.dumps(state))

    _tool(module, "write")("vehicle.flightMode", "Guided")
    written = core["seen"][-1]
    check(
        written["call"] == "set" and json.loads(written["payload"]) == {"value": "Guided"},
        "a write reaches the core wrapped as the bridge expects",
        json.dumps(written),
    )
    _tool(module, "invoke")("vehicle.guidedModeTakeoff", [10])
    check(
        core["seen"][-1]["payload"] == "[10]",
        "invoke args pass through",
        json.dumps(core["seen"][-1]),
    )

    _tool(module, "mock_vehicle")("arducopter")
    started = core["seen"][-1]
    check(
        started["path"] == "links.startMockLink" and started["payload"] == '["apmCopter"]',
        "mock_vehicle starts the core's own mock link",
        json.dumps(started),
    )

    core["answers"]["links.linkConfigurations"] = {
        "elements": [{"class": "TcpConfiguration", "heardVehicle": True}]
    }
    try:
        _tool(module, "write")("vehicle.armed", True)
        check(False, "writes refused while a real vehicle is heard")
    except RuntimeError as error:
        check(
            "real vehicle" in str(error), "writes refused while a real vehicle is heard", str(error)
        )

    try:
        module._api("/links/connect?host=1.2.3.4&port=5760")
        check(False, "native-only routes refused")
    except RuntimeError as error:
        check("404" in str(error), "native-only routes refused", str(error))

    stolen = urllib.request.Request(
        f"http://127.0.0.1:{relay.port}/poll", headers={"Origin": "http://evil.example"}
    )
    try:
        urllib.request.urlopen(stolen)
        check(False, "another origin cannot take requests")
    except urllib.error.HTTPError as error:
        check(error.code == 403, "another origin cannot take requests", str(error.code))

    stop.set()
    relay.shutdown()
    relay.server_close()


def main() -> int:
    server = HTTPServer(("127.0.0.1", 0), _Stub)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    module = _load(server.server_port)

    RESPONSES["/ui/prop"] = {"name": "pill", "t": 5, "values": {"width": 10, "height": 20}}
    RESPONSES["/ui/setprop"] = {"name": "pill", "property": "caption", "value": "hi", "t": 6}
    RESPONSES["/ui/at"] = {"x": 1, "y": 2, "count": 1, "hits": [{"objectName": "pill"}]}
    RESPONSES["/ui/tree"] = {"items": [], "truncated": False}

    print("ui_prop")
    result = _tool(module, "ui_prop")("pill", ["width", "height"])
    check("property=width,height" in last_request(), "commas survive quoting", last_request())
    check(result["values"]["width"] == 10, "parses values")
    try:
        _tool(module, "ui_prop")("pill", [])
        check(False, "empty property list rejected")
    except ValueError:
        check(True, "empty property list rejected")

    print("ui_prop escaping")
    _tool(module, "ui_prop")("a b&c", ["width"])
    check("name=a%20b%26c" in last_request(), "name is percent-encoded", last_request())

    print("ui_set_prop")
    _tool(module, "ui_set_prop")("pill", "caption", "two words&more")
    request = last_request()
    check(
        "property=caption" in request and "value=two%20words%26more" in request,
        "writes are encoded",
        request,
    )

    print("ui_at")
    _tool(module, "ui_at")(12.5, 34)
    check(
        "x=12.5" in last_request() and "y=34" in last_request(),
        "passes coordinates",
        last_request(),
    )

    print("ui_tree")
    _tool(module, "ui_tree")(filter="Batt")
    check("all=1" not in last_request(), "all omitted by default", last_request())
    _tool(module, "ui_tree")(filter="Batt", all=True, visible_only=True)
    check(
        "all=1" in last_request() and "visible=1" in last_request(),
        "all/visible passed",
        last_request(),
    )

    print("ui_watch")
    watched = _tool(module, "ui_watch")("pill", ["width"], frames=4, interval_ms=5, timeout_s=5)
    check(watched["count"] == 4, "parses every ndjson sample", json.dumps(watched))
    check(
        all("t" in s and "frame" in s for s in watched["samples"]),
        "samples carry timestamp and frame",
    )
    try:
        _tool(module, "ui_watch")("pill", [], timeout_s=2)
        check(False, "empty watch property list rejected")
    except ValueError:
        check(True, "empty watch property list rejected")

    print("wait_until on a property")
    RESPONSES["/ui/prop"] = {"name": "drawer", "t": 7, "values": {"_morph": 1}}
    state = _tool(module, "wait_until")("_morph", "1", timeout_s=2, object_name="drawer")
    check(state["values"]["_morph"] == 1, "returns on match")
    check("/ui/prop?name=drawer" in last_request(), "polls the property endpoint", last_request())
    try:
        _tool(module, "wait_until")("_morph", "0", timeout_s=1, object_name="drawer")
        check(False, "raises on timeout")
    except RuntimeError:
        check(True, "raises on timeout")

    print("read")
    RESPONSES["/bridge/get"] = {"kind": "object", "state": "flying"}
    _tool(module, "read")("vehicle.parameterManager.getParameter(-1,RTL_ALT_M)")
    check(
        "path=vehicle.parameterManager.getParameter%28-1%2CRTL_ALT_M%29" in last_request(),
        "core path is percent-encoded",
        last_request(),
    )

    print("write")
    RESPONSES["/bridge/set"] = {"ok": True}
    _tool(module, "write")("vehicle.flightMode", "Position Hold")
    check(
        "value=Position%20Hold" in last_request(),
        "spaces survive every head's decoder",
        last_request(),
    )
    _tool(module, "write")("vehicle.armed", True)
    check("value=true" in last_request(), "booleans go as JSON", last_request())
    _tool(module, "write")("vehicle.parameterManager.getParameter(-1,RTL_ALT_M)", 20)
    check("value=20" in last_request(), "numbers go as JSON", last_request())

    print("invoke")
    RESPONSES["/bridge/invoke"] = {"ok": True}
    _tool(module, "invoke")(
        "vehicle.guidedModeGotoLocation", [{"latitude": 47.5, "longitude": 8.5}]
    )
    check(
        "args=%5B%7B%22latitude%22%3A47.5%2C%22longitude%22%3A8.5%7D%5D" in last_request(),
        "args go as one JSON array",
        last_request(),
    )
    _tool(module, "invoke")("vehicle.guidedModeLand")
    check("args=%5B%5D" in last_request(), "no args is an empty array", last_request())

    print("views")
    RESPONSES["/bridge/get"] = {
        "views": [
            {"path": "view.flyState", "deps": []},
            {"path": "view.coreParameter", "deps": []},
        ],
        "argumentModes": [{"path": "view.coreParameter", "arguments": "<vehicle id>,<name>"}],
    }
    listed = _tool(module, "views")("param")
    check("path=view.dependencies" in last_request(), "reads the view catalog", last_request())
    check(
        listed == [{"path": "view.coreParameter", "arguments": "<vehicle id>,<name>"}],
        "filters and carries argument shapes",
        json.dumps(listed),
    )

    print("wait_until on a core path")
    RESPONSES["/bridge/get"] = {"kind": "object", "state": "flying"}
    _tool(module, "wait_until")("state", "flying", timeout_s=2, endpoint="view.flyState")
    check("/bridge/get?path=view.flyState" in last_request(), "polls the core path", last_request())

    print("mock_vehicle")
    RESPONSES["/links/mocklink"] = {"started": True}
    _tool(module, "mock_vehicle")("px4", {"wind": "8,270"})
    check(
        "autopilot=px4" in last_request() and "wind=8%2C270" in last_request(),
        "passes autopilot and conditions",
        last_request(),
    )

    print("error mapping")
    try:
        _tool(module, "ui_prop")("pill", ["width"]) if False else module._api("/ui/nope")
        check(False, "http error becomes RuntimeError")
    except RuntimeError as error:
        check("400" in str(error), "http error becomes RuntimeError", str(error))

    _relay_checks(module)

    server.shutdown()
    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)})")
        for failure in FAILURES:
            print("  ", failure)
        return 1
    print("ALL PASSED")
    return 0


if __name__ == "__main__":
    sys.exit(main())
