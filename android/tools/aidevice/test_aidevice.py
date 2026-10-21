import json
import threading
import unittest
import urllib.request
from http.server import ThreadingHTTPServer

from aidevice import LOST_AFTER_S, Device, kept, payload, picked, picked_rect, serving


def box(x, y, w=0.1, h=0.1, id=1, conf=0.8, label="car"):
    return {"x": x, "y": y, "w": w, "h": h, "id": id, "conf": conf, "label": label}


class Picking(unittest.TestCase):
    def test_the_box_under_the_point_wins_over_a_nearer_centre(self):
        big = box(0.40, 0.40, w=0.30, h=0.30, id=1)
        small = box(0.47, 0.47, w=0.02, h=0.02, id=2)
        self.assertEqual(picked([big, small], 0.42, 0.42)["id"], 1)

    def test_a_tap_just_beside_a_small_car_still_picks_it(self):
        self.assertEqual(picked([box(0.50, 0.50, w=0.02, h=0.02, id=7)], 0.55, 0.51)["id"], 7)

    def test_nothing_near_the_tap_or_untracked_boxes_pick_nothing(self):
        self.assertIsNone(picked([box(0.1, 0.1)], 0.9, 0.9))
        self.assertIsNone(picked([box(0.5, 0.5, id=None)], 0.55, 0.55))

    def test_a_dragged_rectangle_picks_the_box_it_covers_most(self):
        boxes = [box(0.1, 0.1, id=1), box(0.6, 0.6, id=2)]
        self.assertEqual(picked_rect(boxes, {"x": 0.58, "y": 0.58, "w": 0.12, "h": 0.12})["id"], 2)
        self.assertIsNone(picked_rect(boxes, {"x": 0.3, "y": 0.3, "w": 0.05, "h": 0.05}))


class Lock(unittest.TestCase):
    def test_a_seen_lock_is_refreshed_and_a_briefly_missing_one_is_kept(self):
        lock = {"id": 3, "seen": 10.0, "at": (0.55, 0.55), "velocity": (0.0, 0.0)}
        self.assertEqual(kept(lock, [box(0.5, 0.5, id=3)], 11.0)["seen"], 11.0)
        self.assertEqual(kept(lock, [box(0.9, 0.9, id=4)], 10.0 + LOST_AFTER_S), lock)

    def test_a_renumbered_track_near_where_the_target_was_heading_keeps_the_lock(self):
        lock = {"id": 3, "seen": 10.0, "at": (0.40, 0.50), "velocity": (0.10, 0.0)}
        moved_on = box(0.45, 0.45, w=0.1, h=0.1, id=8)
        decoy = box(0.05, 0.05, id=9)
        held = kept(lock, [decoy, moved_on], 11.0)
        self.assertEqual(held["id"], 8)
        self.assertAlmostEqual(held["velocity"][0], 0.10, places=6)

    def test_a_box_far_from_where_the_target_should_be_is_not_adopted(self):
        lock = {"id": 3, "seen": 10.0, "at": (0.40, 0.50), "velocity": (0.0, 0.0)}
        self.assertEqual(kept(lock, [box(0.85, 0.85, id=8)], 10.5), lock)

    def test_a_lock_missing_too_long_is_released(self):
        self.assertIsNone(
            kept(
                {"id": 3, "seen": 10.0, "at": (0.5, 0.5), "velocity": (0.0, 0.0)},
                [],
                10.0 + LOST_AFTER_S + 0.1,
            )
        )
        self.assertIsNone(kept(None, [box(0.5, 0.5)], 1.0))


class Payload(unittest.TestCase):
    def test_it_carries_exactly_what_the_core_reads_and_marks_only_the_locked_box(self):
        sent = payload(
            [box(0.123456, 0.2, id=5), box(0.6, 0.6, id=6)], {"id": 5, "seen": 0.0}, 182.6
        )
        self.assertEqual(sent["ageMs"], 183)
        self.assertEqual(
            [set(b) for b in sent["boxes"]], [{"x", "y", "w", "h", "label", "conf", "target"}] * 2
        )
        self.assertEqual([b["target"] for b in sent["boxes"]], [True, False])
        self.assertEqual(sent["boxes"][0]["x"], 0.1235)
        self.assertEqual(sent["track"]["state"], "locked")
        self.assertEqual(payload([], None, 0)["track"]["state"], "idle")


class Api(unittest.TestCase):
    def setUp(self):
        self.device = Device(phone=None, camera=None, detector=None, fps=10)
        self.device.boxes, self.device.frames, self.device.captured = [box(0.4, 0.4, id=9)], 1, 0.0
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), serving(self.device))
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.base = f"http://127.0.0.1:{self.server.server_address[1]}/api/streams/sim"

    def tearDown(self):
        self.server.shutdown()

    def send(self, method, path, body=None):
        request = urllib.request.Request(
            self.base + path, data=json.dumps(body).encode() if body else None, method=method
        )
        with urllib.request.urlopen(request, timeout=3) as reply:
            return json.load(reply)

    def test_the_device_tracker_endpoints_lock_and_release_like_the_core_expects(self):
        self.assertEqual(
            self.send("PUT", "/track", {"x": 0.45, "y": 0.45}), {"status": "tracking", "id": 9}
        )
        self.assertEqual(
            self.send("PUT", "/track", {"x": 0.95, "y": 0.95}), {"status": "no-target"}
        )
        self.assertEqual(
            self.send("PUT", "/track/visual", {"x": 0.38, "y": 0.38, "w": 0.14, "h": 0.14})["id"], 9
        )
        self.assertEqual(self.send("DELETE", "/track"), {"status": "stopped"})
        self.assertIsNone(self.device.lock)

    def test_the_stream_sends_server_sent_events_with_the_target_marked(self):
        self.send("PUT", "/track", {"x": 0.45, "y": 0.45})
        with urllib.request.urlopen(self.base + "/detections/stream", timeout=3) as stream:
            event = json.loads(stream.readline().decode().removeprefix("data: "))
        self.assertTrue(event["boxes"][0]["target"])


if __name__ == "__main__":
    unittest.main()
