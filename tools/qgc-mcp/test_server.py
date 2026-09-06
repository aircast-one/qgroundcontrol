#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = ["mcp<2"]
# ///
"""Tests for the MCP tool layer, run against a stub debug API rather than a running app.

What breaks here is URL construction, quoting and response parsing - not QGC. A stub makes those
assertable without a build, and records the exact paths requested, which is the only way to catch
a tool that asks the right question with the wrong URL. The comma in a multi-property read was
exactly that: percent-encoded, it became one property with a very strange name, and every value
the caller wanted came back as an error.

    uv run tools/qgc-mcp/test_server.py
"""

import importlib.util
import json
import sys
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

SERVER_PY = Path(__file__).with_name("server.py")

REQUESTS: list[str] = []
RESPONSES: dict[str, dict] = {}


class _Stub(BaseHTTPRequestHandler):
    def log_message(self, *args):  # keep the test output readable
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
    # The separator has to survive quoting or the endpoint sees one impossibly named property.
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
    check("property=caption" in request and "value=two%20words%26more" in request,
          "writes are encoded", request)

    print("ui_at")
    _tool(module, "ui_at")(12.5, 34)
    check("x=12.5" in last_request() and "y=34" in last_request(), "passes coordinates", last_request())

    print("ui_tree")
    _tool(module, "ui_tree")(filter="Batt")
    check("all=1" not in last_request(), "all omitted by default", last_request())
    _tool(module, "ui_tree")(filter="Batt", all=True, visible_only=True)
    check("all=1" in last_request() and "visible=1" in last_request(), "all/visible passed", last_request())

    print("ui_watch")
    watched = _tool(module, "ui_watch")("pill", ["width"], frames=4, interval_ms=5, timeout_s=5)
    check(watched["count"] == 4, "parses every ndjson sample", json.dumps(watched))
    check(all("t" in s and "frame" in s for s in watched["samples"]),
          "samples carry timestamp and frame")
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

    print("error mapping")
    try:
        _tool(module, "ui_prop")("pill", ["width"]) if False else module._api("/ui/nope")
        check(False, "http error becomes RuntimeError")
    except RuntimeError as error:
        check("400" in str(error), "http error becomes RuntimeError", str(error))

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
