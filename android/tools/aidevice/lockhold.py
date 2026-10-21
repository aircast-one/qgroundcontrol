import argparse
import itertools
import json
import math
from pathlib import Path

import numpy as np
from aidevice import centre, kept, locked_on, picked, sidecar
from detector import Detector
from scene import VirtualCamera, car, looking_at, moved

HOME = (47.3977419, 8.5455938)
START, HEADING = (47.3972617, 8.5460329), 206
ALTITUDE_M, CAR_MPS, CLOSING_MPS, BEHIND_M = 40.0, 5.0, 5.0, 100.0
FPS, SECONDS, RELEASE_S = 10, 18.0, 17.1
ON_TARGET = 0.06


def approach(step):
    t = step / FPS
    target = moved(
        *START,
        CAR_MPS * t * math.cos(math.radians(HEADING)),
        CAR_MPS * t * math.sin(math.radians(HEADING)),
    )
    ahead = BEHIND_M - CLOSING_MPS * t
    tilt = -math.degrees(math.atan2(ALTITUDE_M, ahead))
    return looking_at(target, HOME, math.hypot(ahead, ALTITUDE_M), tilt, HEADING), [
        car(*target, HEADING, CAR_MPS)
    ]


def rendered():
    camera = VirtualCamera()

    def pair(step):
        pose, cars = approach(step)
        shown = camera.picture(pose, cars, settle_ms=25000 if step == 0 else 400)
        return shown, camera.picture(pose, [], settle_ms=0)

    pairs = [pair(step) for step in range(int(SECONDS * FPS))]
    camera.close()
    return pairs


def truth(shown, empty):
    changed = np.argwhere(np.abs(shown.astype(int) - empty.astype(int)).sum(axis=2) > 40)
    if len(changed) < 4:
        return None
    (top, left), (bottom, right) = changed.min(axis=0), changed.max(axis=0)
    height, width = shown.shape[:2]
    return {
        "x": left / width,
        "y": top / height,
        "w": (right - left) / width,
        "h": (bottom - top) / height,
    }


def on_target(box, car_box):
    return (
        box is not None
        and car_box is not None
        and math.dist(centre(box), centre(car_box)) <= max(car_box["w"], car_box["h"], ON_TARGET)
    )


def nearest(boxes, car_box):
    return min(
        (b for b in boxes if on_target(b, car_box)),
        key=lambda b: math.dist(centre(b), centre(car_box)),
        default=None,
    )


def first_boxed(frames, truths, model):
    detector = Detector(model)
    return next(
        (i, seed)
        for i, seed in (
            (i, nearest(detector.boxes(f), t))
            for i, (f, t) in enumerate(zip(frames, truths, strict=True))
        )
        if seed
    )


def detector_follow(frames, model, start, at):
    detector = Detector(model)
    seed = picked(detector.boxes(frames[start], track=True), *at)

    def step(carried, indexed):
        index, frame = indexed
        boxes = detector.boxes(frame, track=True)
        lock = kept(carried[0], boxes, index / FPS)
        return lock, next((b for b in boxes if lock and b.get("id") == lock["id"]), None)

    steps = itertools.accumulate(
        enumerate(frames[start + 1 :], start + 1),
        step,
        initial=(locked_on(seed, start / FPS) if seed else None, seed),
    )
    return [None] * start + [held for _, held in steps]


def visual_follow(frames, seed, directory, start):
    tracker = sidecar(directory, FPS).VisualTracker()
    bgr = [np.ascontiguousarray(f[:, :, ::-1]) for f in frames[start:]]
    tracker.ensure(bgr[0], {k: seed[k] for k in ("x", "y", "w", "h")}, 1)
    boxes = [tracker.update(f) for f in bgr[1:]]
    return [None] * start + [
        seed,
        *[{"x": b.x, "y": b.y, "w": b.w, "h": b.h} if b else None for b in boxes],
    ]


def scored(reports, truths):
    first = int((SECONDS - 10) * FPS)
    window = list(zip(reports[first:], truths[first:], strict=True))
    release = int(RELEASE_S * FPS)
    last_seen = max(
        (
            i
            for i, (r, t) in enumerate(zip(reports[: release + 1], truths, strict=False))
            if on_target(r, t)
        ),
        default=None,
    )
    errors = [math.dist(centre(r), centre(t)) for r, t in window if on_target(r, t)]
    seconds = [
        round(
            sum(
                on_target(r, t)
                for r, t in zip(reports[i : i + FPS], truths[i : i + FPS], strict=False)
            )
            / FPS,
            1,
        )
        for i in range(0, int(SECONDS * FPS), FPS)
    ]
    return {
        "onTargetEachSecond": seconds,
        "onTargetLast10s": round(sum(on_target(r, t) for r, t in window) / len(window), 3),
        "sightingAgeAtReleaseMs": None
        if last_seen is None
        else round((release - last_seen) * 1000 / FPS),
        "meanCentreError": round(sum(errors) / len(errors), 4) if errors else None,
    }


def main():
    parser = argparse.ArgumentParser(
        description="How long each tracker keeps the car on a scripted approach, rendered by the virtual camera"
    )
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--sidecar", required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    pairs = rendered()
    frames = [shown for shown, _ in pairs]
    truths = [truth(shown, empty) for shown, empty in pairs]
    start, seed = first_boxed(frames, truths, "visdrone-yolov8s.pt")
    at = centre(seed)
    results = {
        "tap possible from": f"{start / FPS:.1f} s ({math.hypot(BEHIND_M - CLOSING_MPS * start / FPS, ALTITUDE_M):.0f} m away)",
        "old stand-in (yolo11m, follow detections)": scored(
            detector_follow(frames, "visdrone-yolo11m.pt", start, at), truths
        ),
        "device tap today (yolov8s, follow detections)": scored(
            detector_follow(frames, "visdrone-yolov8s.pt", start, at), truths
        ),
        "device visual tracker (yolov8s seed, ViT)": scored(
            visual_follow(frames, seed, args.sidecar, start), truths
        ),
    }
    (args.out / "lockhold.json").write_text(json.dumps(results, indent=1))
    print(json.dumps(results, indent=1))


if __name__ == "__main__":
    main()
