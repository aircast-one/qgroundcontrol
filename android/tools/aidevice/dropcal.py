import argparse
import json
import subprocess
import sys
import urllib.parse
import urllib.request

import cv2
import numpy as np

REFERENCE_FRAMES = 5
HELD_FRAMES = 3
THRESHOLD = 12.0


def separation_frame(frames, roi):
    x, y, w, h = roi
    patches = [
        cv2.cvtColor(frame[y : y + h, x : x + w], cv2.COLOR_BGR2GRAY).astype(float)
        for frame in frames
    ]
    reference = np.median(np.stack(patches[:REFERENCE_FRAMES]), axis=0)
    changed = [float(np.abs(patch - reference).mean()) > THRESHOLD for patch in patches]
    return next(
        (i for i in range(len(changed)) if all(changed[i : i + HELD_FRAMES])),
        None,
    )


def video(path):
    capture = cv2.VideoCapture(path)
    fps = capture.get(cv2.CAP_PROP_FPS) or 30.0
    frames = []
    while True:
        read, frame = capture.read()
        if not read:
            break
        frames.append(frame)
    return frames, fps


def separation(args):
    frames, fps = video(args.video)
    roi = tuple(int(v) for v in args.roi.split(","))
    found = args.frame if args.frame is not None else separation_frame(frames, roi)
    if found is None:
        sys.exit("no separation seen in the region; pass --frame N after checking the video")
    json.dump(
        {"frame": found, "separationUs": args.video_start_us + round(found / fps * 1e6)}, sys.stdout
    )
    print()


def fit(args):
    run = subprocess.run(
        [args.binary, "--tlog", args.tlog, "--drops", args.drops, "--mass-g", str(args.mass_g)]
        + (["--vehicle", str(args.vehicle)] if args.vehicle else []),
        capture_output=True,
        text=True,
        check=True,
    )
    result = json.loads(run.stdout)
    applied = {
        path: urllib.request.urlopen(
            f"{args.apply}/bridge/set?{urllib.parse.urlencode({'path': path, 'value': json.dumps(value)})}",
            timeout=5,
        )
        .read()
        .decode()
        for path, value in result["settings"].items()
        if args.apply and value is not None
    }
    json.dump({**result, "applied": applied}, sys.stdout, indent=2)
    print()


def main():
    parser = argparse.ArgumentParser(
        description="Calibrate payload drag and release delay from filmed test drops."
    )
    commands = parser.add_subparsers(required=True)
    seen = commands.add_parser("separation", help="find the frame the payload leaves the drone")
    seen.add_argument("--video", required=True)
    seen.add_argument("--video-start-us", type=int, required=True)
    seen.add_argument("--roi", required=True, help="x,y,w,h in pixels around the hanging payload")
    seen.add_argument("--frame", type=int, help="the separation frame, when picked by hand")
    seen.set_defaults(run=separation)
    fitted = commands.add_parser(
        "fit", help="fit drag area and release delay, optionally write them to the app"
    )
    fitted.add_argument("--binary", required=True, help="built drop_calibration example")
    fitted.add_argument("--tlog", required=True)
    fitted.add_argument(
        "--drops",
        required=True,
        help='JSON: [{"separationUs": ..., "landed": {"latitude": ..., "longitude": ...}}]',
    )
    fitted.add_argument("--mass-g", type=float, required=True)
    fitted.add_argument("--vehicle", type=int)
    fitted.add_argument("--apply", help="debug API base URL, e.g. http://127.0.0.1:8777")
    fitted.set_defaults(run=fit)
    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
