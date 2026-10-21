import argparse
import concurrent.futures
import json
import math
import threading
import time
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PICK_RADIUS = 0.08
LOST_AFTER_S = 2.0
REACQUIRE_RADIUS = 0.12
LENS_EVERY_S = 2.0
FIELDS = ("x", "y", "w", "h", "label", "conf")
LENS = "settings.gimbalControllerSettings.cameraHFov"


def centre(box):
    return (box["x"] + box["w"] / 2, box["y"] + box["h"] / 2)


def picked(boxes, x, y):
    tracked = [b for b in boxes if b.get("id") is not None]
    inside = [
        b for b in tracked if b["x"] <= x <= b["x"] + b["w"] and b["y"] <= y <= b["y"] + b["h"]
    ]
    near = [b for b in tracked if math.dist(centre(b), (x, y)) <= PICK_RADIUS]
    return min(inside or near, key=lambda b: math.dist(centre(b), (x, y)), default=None)


def overlap(box, rect):
    width = min(box["x"] + box["w"], rect["x"] + rect["w"]) - max(box["x"], rect["x"])
    height = min(box["y"] + box["h"], rect["y"] + rect["h"]) - max(box["y"], rect["y"])
    shared = max(width, 0) * max(height, 0)
    return shared / (box["w"] * box["h"] + rect["w"] * rect["h"] - shared or 1)


def picked_rect(boxes, rect):
    best = max(
        (b for b in boxes if b.get("id") is not None), key=lambda b: overlap(b, rect), default=None
    )
    return best if best and overlap(best, rect) > 0.1 else None


def locked_on(box, now, previous=None):
    at = centre(box)
    moved = previous and now > previous["seen"]
    velocity = (
        tuple((a - b) / (now - previous["seen"]) for a, b in zip(at, previous["at"], strict=True))
        if moved
        else (previous or {}).get("velocity", (0.0, 0.0))
    )
    return {"id": box["id"], "seen": now, "at": at, "velocity": velocity}


def expected(lock, now):
    return tuple(
        a + v * (now - lock["seen"]) for a, v in zip(lock["at"], lock["velocity"], strict=True)
    )


def kept(lock, boxes, now):
    if lock is None:
        return None
    same = next((b for b in boxes if b.get("id") == lock["id"]), None)
    there = expected(lock, now)
    nearest = min(
        (
            b
            for b in boxes
            if b.get("id") is not None and math.dist(centre(b), there) <= REACQUIRE_RADIUS
        ),
        key=lambda b: math.dist(centre(b), there),
        default=None,
    )
    found = same or nearest
    if found:
        return locked_on(found, now, lock)
    return lock if now - lock["seen"] <= LOST_AFTER_S else None


def payload(boxes, lock, age_ms):
    target = lock["id"] if lock else None
    visible = any(b.get("id") == target for b in boxes)
    return {
        "ageMs": round(age_ms),
        "boxes": [
            {k: round(b[k], 4) if isinstance(b[k], float) else b[k] for k in FIELDS}
            | {"target": target is not None and b.get("id") == target}
            for b in boxes
        ],
        "track": {"state": "idle" if lock is None else "locked" if visible else "searching"},
    }


def read(phone, path):
    url = "{}/bridge/get?{}".format(phone, urllib.parse.urlencode({"path": path}))
    with urllib.request.urlopen(url, timeout=2) as reply:
        return json.load(reply)


class Device:
    def __init__(self, phone, camera, detector, fps, log=None):
        self.phone, self.camera, self.detector, self.period, self.log = (
            phone,
            camera,
            detector,
            1 / fps,
            log,
        )
        self.changed = threading.Condition()
        self.pool = concurrent.futures.ThreadPoolExecutor(2)
        self.lens, self.lens_read = None, 0.0
        self.boxes, self.lock, self.captured, self.frames = [], None, 0.0, 0

    def world(self):
        pose = self.pool.submit(read, self.phone, "view.syntheticView")
        overlays = self.pool.submit(read, self.phone, "view.syntheticOverlays")
        if time.time() - self.lens_read > LENS_EVERY_S:
            self.lens, self.lens_read = read(self.phone, LENS).get("rawValue"), time.time()
        shown = pose.result()
        return shown | {"fov": self.lens or shown.get("fov")}, overlays.result().get("cars") or []

    def run(self):
        while True:
            started = time.time()
            try:
                pose, cars = self.world()
            except OSError:
                time.sleep(0.5)
                continue
            if not pose.get("available"):
                time.sleep(0.5)
                continue
            boxes = self.detector.boxes(self.camera.picture(pose, cars), track=True)
            with self.changed:
                self.boxes, self.captured, self.frames = boxes, started, self.frames + 1
                self.lock = kept(self.lock, boxes, started)
                self.changed.notify_all()
            if self.log:
                locked = (self.lock or {}).get("id")
                record = {
                    "t": started,
                    "boxes": len(boxes),
                    "lock": locked,
                    "seen": any(b.get("id") == locked for b in boxes),
                }
                self.log.write(json.dumps(record) + "\n")
                self.log.flush()
            time.sleep(max(self.period - (time.time() - started), 0))

    def latest(self, after):
        with self.changed:
            self.changed.wait_for(lambda: self.frames != after, timeout=5)
            return self.frames, payload(self.boxes, self.lock, (time.time() - self.captured) * 1000)

    def track(self, request, visual):
        with self.changed:
            box = (
                picked_rect(self.boxes, request)
                if visual
                else picked(self.boxes, request["x"], request["y"])
            )
            self.lock = locked_on(box, self.captured) if box else None
            return {"status": "tracking", "id": box["id"]} if box else {"status": "no-target"}

    def stop(self):
        with self.changed:
            self.lock = None
        return {"status": "stopped"}


def serving(device):
    class Api(BaseHTTPRequestHandler):
        def reply(self, status, body):
            data = json.dumps(body).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            if not self.path.split("?")[0].endswith("/detections/stream"):
                return self.reply(404, {"error": "not found"})
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            frame = None
            try:
                while True:
                    frame, body = device.latest(frame)
                    self.wfile.write(b"data: " + json.dumps(body).encode() + b"\n\n")
                    self.wfile.flush()
            except OSError:
                pass

        def do_PUT(self):
            path = self.path.split("?")[0]
            if not (path.endswith("/track") or path.endswith("/track/visual")):
                return self.reply(404, {"error": "not found"})
            length = int(self.headers.get("Content-Length") or 0)
            request = json.loads(self.rfile.read(length) or b"{}")
            self.reply(200, device.track(request, path.endswith("/visual")))

        do_POST = do_PUT

        def do_DELETE(self):
            self.reply(200, device.stop()) if self.path.split("?")[0].endswith(
                "/track"
            ) else self.reply(404, {"error": "not found"})

        def log_message(self, *args):
            pass

    return Api


def main():
    parser = argparse.ArgumentParser(
        description="Stand in for the Aircast device: render what the drone camera sees, run YOLO on it, serve its detections and tracker"
    )
    parser.add_argument(
        "--phone",
        default="http://127.0.0.1:8777",
        help="the phone's debug API, through adb forward",
    )
    parser.add_argument("--port", type=int, default=8099)
    parser.add_argument("--model", default="visdrone-yolo11m.pt")
    parser.add_argument("--imgsz", type=int, default=640)
    parser.add_argument("--conf", type=float, default=0.2)
    parser.add_argument("--fps", type=float, default=10)
    parser.add_argument("--log", type=argparse.FileType("a"), help="append one JSON line per frame")
    args = parser.parse_args()
    from detector import Detector
    from scene import VirtualCamera

    device = Device(
        args.phone,
        VirtualCamera(),
        Detector(args.model, imgsz=args.imgsz, conf=args.conf),
        args.fps,
        args.log,
    )
    threading.Thread(target=device.run, daemon=True).start()
    ThreadingHTTPServer(("0.0.0.0", args.port), serving(device)).serve_forever()


if __name__ == "__main__":
    main()
