import argparse
import csv
import json
import sys
import urllib.parse
import urllib.request

import cv2
import numpy as np

FLOW_WIDTH = 320
SEARCH_MS = 1500
STEP_MS = 5


def flow_rates(frames, times_s, hfov_deg):
    def grey(frame):
        scale = FLOW_WIDTH / frame.shape[1]
        small = cv2.resize(frame, (FLOW_WIDTH, int(frame.shape[0] * scale)))
        return cv2.cvtColor(small, cv2.COLOR_BGR2GRAY)

    greys = [grey(frame) for frame in frames]
    deg_per_px = hfov_deg / FLOW_WIDTH
    pairs = zip(greys, greys[1:], times_s, times_s[1:], strict=False)
    return [
        (
            (t0 + t1) / 2,
            -float(
                np.median(
                    cv2.calcOpticalFlowFarneback(a, b, None, 0.5, 3, 21, 3, 5, 1.1, 0)[..., 0]
                )
            )
            * deg_per_px
            / (t1 - t0),
        )
        for a, b, t0, t1 in pairs
        if t1 > t0
    ]


def telemetry_rates(rows):
    times = np.array([row[0] for row in rows])
    headings = np.unwrap(np.radians([row[1] for row in rows]))
    return times, np.degrees(np.gradient(headings, times))


def delay(video, telemetry):
    times, rates = telemetry
    at = np.array([t for t, _ in video])
    seen = np.array([r for _, r in video])

    def score(shift_s):
        expected = np.interp(at - shift_s, times, rates)
        if expected.std() == 0 or seen.std() == 0:
            return -1.0
        return float(np.corrcoef(expected, seen)[0, 1])

    shifts = [ms / 1000 for ms in range(-SEARCH_MS, SEARCH_MS + 1, STEP_MS)]
    scored = [(score(shift), shift) for shift in shifts]
    best, shift = max(scored)
    return {"delayMs": round(shift * 1000), "correlation": round(best, 3)}


def video_frames(path, start_s):
    capture = cv2.VideoCapture(path)
    fps = capture.get(cv2.CAP_PROP_FPS) or 30.0
    frames = []
    while True:
        read, frame = capture.read()
        if not read:
            break
        frames.append(frame)
    return frames, [start_s + i / fps for i in range(len(frames))]


def attitude_rows(path):
    with open(path) as source:
        return [
            (int(row["wallUs"]) / 1e6, float(row["headingDeg"])) for row in csv.DictReader(source)
        ]


def apply(api, latency_ms):
    query = urllib.parse.urlencode(
        {"path": "settings.appSettings.detectionsLatencyMs", "value": json.dumps(latency_ms)}
    )
    return urllib.request.urlopen(f"{api}/bridge/set?{query}", timeout=5).read().decode()


def main():
    parser = argparse.ArgumentParser(
        description="Measure the camera delay: how long after the drone turns the video shows it turn."
    )
    parser.add_argument("--video", required=True)
    parser.add_argument(
        "--video-start-us",
        type=int,
        required=True,
        help="wall-clock microseconds of the first video frame (the video marker in the .detections.jsonl)",
    )
    parser.add_argument("--attitude", required=True, help="CSV from flight_replay --attitude")
    parser.add_argument(
        "--hfov", type=float, default=70.0, help="horizontal field of view of the video in degrees"
    )
    parser.add_argument(
        "--apply",
        help="debug API base URL (e.g. http://127.0.0.1:8777) to write detectionsLatencyMs",
    )
    args = parser.parse_args()
    frames, times = video_frames(args.video, args.video_start_us / 1e6)
    result = delay(
        flow_rates(frames, times, args.hfov), telemetry_rates(attitude_rows(args.attitude))
    )
    if args.apply:
        result["applied"] = apply(args.apply, max(result["delayMs"], 0))
    json.dump(result, sys.stdout)
    print()


if __name__ == "__main__":
    main()
