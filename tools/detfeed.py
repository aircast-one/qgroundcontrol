import json
import math
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


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
        started = time.time()
        while True:
            phase = (time.time() - started) * 0.6
            x = 0.30 + 0.22 * math.sin(phase)
            y = 0.30 + 0.16 * math.cos(phase * 0.7)
            payload = {
                "ageMs": 0,
                "boxes": [
                    {"label": "car", "conf": 0.91, "x": round(x, 4), "y": round(y, 4),
                     "w": 0.22, "h": 0.18, "target": True},
                    {"label": "person", "conf": 0.47, "x": 0.62, "y": 0.55,
                     "w": 0.12, "h": 0.30, "target": False},
                ],
            }
            try:
                self.wfile.write(b"data: " + json.dumps(payload).encode() + b"\n\n")
                self.wfile.flush()
            except Exception:
                print("follower gone", flush=True)
                return
            time.sleep(0.2)

    def log_message(self, *args):
        pass


ThreadingHTTPServer(("0.0.0.0", 8099), Feed).serve_forever()
