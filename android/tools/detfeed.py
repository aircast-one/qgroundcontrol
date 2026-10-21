import argparse
import json
import math
import time
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

EARTH_RADIUS_M = 6_371_000
CAR_LENGTH_M = 4.4
FOCAL = 0.5 / math.tan(math.radians(35))
ASPECT = 16 / 9
RANGE_REFRESH_S = 1.0


def demo_boxes(phase):
    x = 0.30 + 0.22 * math.sin(phase)
    y = 0.30 + 0.16 * math.cos(phase * 0.7)
    return [
        {"label": "car", "conf": 0.91, "x": round(x, 4), "y": round(y, 4), "w": 0.22, "h": 0.18, "target": True},
        {"label": "person", "conf": 0.47, "x": 0.62, "y": 0.55, "w": 0.12, "h": 0.30, "target": False},
    ]


def read(base, path):
    url = "%s/bridge/get?%s" % (base, urllib.parse.urlencode({"path": path}))
    with urllib.request.urlopen(url, timeout=2) as reply:
        return json.load(reply)


def drone_fix(base):
    coordinate = read(base, "vehicle.coordinate")
    altitude = read(base, "vehicle.altitudeRelative")
    return coordinate.get("value", coordinate), altitude.get("rawValue") or 0.0


def range_m(drone, altitude, car):
    north = math.radians(car["latitude"] - drone["latitude"]) * EARTH_RADIUS_M
    east = math.radians(car["longitude"] - drone["longitude"]) * EARTH_RADIUS_M * math.cos(math.radians(drone["latitude"]))
    return math.sqrt(north**2 + east**2 + altitude**2)


def car_box(picture, metres):
    w = min(max(CAR_LENGTH_M / metres * FOCAL, 0.02), 0.5)
    h = w * ASPECT * 0.6
    return {
        "label": "car",
        "conf": 0.88,
        "x": round(picture["x"] - w / 2, 4),
        "y": round(picture["y"] - h, 4),
        "w": round(w, 4),
        "h": round(h, 4),
        "target": True,
    }


def sim_boxes(base, fix):
    cars = read(base, "view.syntheticOverlays").get("cars") or []
    drone, altitude = fix
    return [car_box(car["picture"], range_m(drone, altitude, car)) for car in cars if car.get("picture")][:1]


def frames(sim, period):
    started = time.time()
    fix = drone_fix(sim) if sim else None
    refreshed = started
    while True:
        now = time.time()
        if sim and now - refreshed >= RANGE_REFRESH_S:
            fix, refreshed = drone_fix(sim), now
        try:
            boxes = sim_boxes(sim, fix) if sim else demo_boxes((now - started) * 0.6)
        except OSError:
            boxes = []
        yield {"ageMs": 0, "boxes": boxes}
        time.sleep(period)


def serving(sim, period):
    class Feed(BaseHTTPRequestHandler):
        def do_GET(self):
            if "/detections/stream" not in self.path:
                self.send_response(404)
                self.end_headers()
                return
            print("follower connected: %s" % self.path, flush=True)
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            try:
                for payload in frames(sim, period):
                    self.wfile.write(b"data: " + json.dumps(payload).encode() + b"\n\n")
                    self.wfile.flush()
            except OSError:
                print("follower gone", flush=True)

        def log_message(self, *args):
            pass

    return Feed


def main():
    parser = argparse.ArgumentParser(description="SSE detection feed: demo boxes, or a detector that sees the simulated car")
    parser.add_argument("--port", type=int, default=8099)
    parser.add_argument("--sim", metavar="DEBUG_API", help="e.g. http://127.0.0.1:8777 - box the simulated car where the phone's camera sees it")
    args = parser.parse_args()
    ThreadingHTTPServer(("0.0.0.0", args.port), serving(args.sim, 0.1 if args.sim else 0.2)).serve_forever()


if __name__ == "__main__":
    main()
