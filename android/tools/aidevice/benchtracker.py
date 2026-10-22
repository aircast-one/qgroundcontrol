import argparse
import json
import sys

import numpy as np
from aidevice import sidecar
from lockhold import on_target, truth
from scene import VirtualCamera


class Eyes:
    def __init__(self, directory, fps):
        self.camera = VirtualCamera()
        self.tracker = sidecar(directory, fps).VisualTracker()
        self.tracking = False

    def look(self, request):
        if request.get("reset"):
            self.tracker, self.tracking = type(self.tracker)(), False
            return {"box": None, "onTarget": None, "reason": "reset"}
        shown = self.camera.picture(request["pose"], request["cars"], request.get("settleMs", 0))
        seen = (
            truth(shown, self.camera.picture(request["pose"], [], 0))
            if request.get("seed") or self.tracking
            else None
        )
        bgr = np.ascontiguousarray(shown[:, :, ::-1])
        if request.get("seed") and seen:
            self.tracker.ensure(bgr, seen, 1)
            self.tracking = True
            return {"box": seen, "onTarget": True, "reason": "seeded"}
        if not self.tracking:
            return {"box": None, "onTarget": None, "reason": "idle"}
        found = self.tracker.update(bgr)
        box = found and {"x": found.x, "y": found.y, "w": found.w, "h": found.h}
        return {
            "box": box,
            "onTarget": bool(on_target(box, seen)),
            "reason": self.tracker.reason,
            "truthPx": seen and round(seen["w"] * shown.shape[1]),
        }


def main():
    parser = argparse.ArgumentParser(
        description="The device's visual tracker on virtual camera frames, one JSON request per line, for interceptbench's closed loop"
    )
    parser.add_argument("--sidecar", required=True)
    parser.add_argument("--fps", type=float, default=15)
    parser.add_argument("--log", type=argparse.FileType("a"), help="append each request and reply")
    args = parser.parse_args()
    eyes = Eyes(args.sidecar, args.fps)
    for line in sys.stdin:
        request = json.loads(line)
        reply = eyes.look(request)
        print(json.dumps(reply), flush=True)
        if args.log:
            args.log.write(json.dumps({"request": request, "reply": reply}) + "\n")
            args.log.flush()


if __name__ == "__main__":
    main()
