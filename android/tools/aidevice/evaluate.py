import argparse
import itertools
import json
import time
from pathlib import Path

import cv2
from detector import Detector
from scene import VirtualCamera, car, looking_at

HOME = (47.3977419, 8.5455938)
SITES = {
    "open road": ((47.3972617, 8.5460329), 206),
    "shaded street": ((47.3963121, 8.5436726), 106),
}
SLANTS = [20, 40, 80, 150]
TILTS = [-30, -60, -90]
TURNS = [0, 90]
MATCH_IOU = 0.3


def shots():
    return [
        {"site": site, "slant": slant, "tilt": tilt, "turn": turn, "car": with_car}
        for site, slant, tilt, turn, with_car in itertools.product(
            SITES, SLANTS, TILTS, TURNS, [True, False]
        )
    ]


def render(out):
    camera = VirtualCamera()
    planned = shots()
    first_at_site = {shot["site"]: index for index, shot in reversed(list(enumerate(planned)))}

    def picture(index, shot):
        target, heading = SITES[shot["site"]]
        return camera.picture(
            looking_at(target, HOME, shot["slant"], shot["tilt"], heading + shot["turn"]),
            [car(*target, heading)] if shot["car"] else [],
            settle_ms=25000 if first_at_site[shot["site"]] == index else 4000,
        )

    rendered = [
        shot | {"file": f"{index:03d}.png"}
        for index, shot in enumerate(planned)
        if cv2.imwrite(
            str(out / f"{index:03d}.png"), cv2.cvtColor(picture(index, shot), cv2.COLOR_RGB2BGR)
        )
    ]
    camera.close()
    (out / "shots.json").write_text(json.dumps(rendered, indent=1))


def truth(out, shot, shots_rendered):
    twin = next(
        t
        for t in shots_rendered
        if not t["car"] and all(t[k] == shot[k] for k in ("site", "slant", "tilt", "turn"))
    )
    with_car, without = (cv2.imread(str(out / s["file"])).astype(int) for s in (shot, twin))
    changed = (abs(with_car - without).sum(axis=2) > 60).astype("uint8")
    count, _, stats, _ = cv2.connectedComponentsWithStats(changed)
    blobs = [stats[i] for i in range(1, count) if stats[i][cv2.CC_STAT_AREA] >= 12]
    if not blobs:
        return None
    x, y, w, h, _ = max(blobs, key=lambda b: b[cv2.CC_STAT_AREA])
    height, width = changed.shape
    return {"x": x / width, "y": y / height, "w": w / width, "h": h / height}


def overlap(a, b):
    width = min(a["x"] + a["w"], b["x"] + b["w"]) - max(a["x"], b["x"])
    height = min(a["y"] + a["h"], b["y"] + b["h"]) - max(a["y"], b["y"])
    shared = max(width, 0) * max(height, 0)
    return shared / (a["w"] * a["h"] + b["w"] * b["h"] - shared or 1)


def scored(detector, out, shot, shots_rendered):
    started = time.perf_counter()
    boxes = detector.boxes(cv2.cvtColor(cv2.imread(str(out / shot["file"])), cv2.COLOR_BGR2RGB))
    took = time.perf_counter() - started
    car_box = truth(out, shot, shots_rendered) if shot["car"] else None
    found = [b for b in boxes if car_box and overlap(b, car_box) >= MATCH_IOU]
    return shot | {
        "visible": car_box is not None,
        "hit": bool(found),
        "conf": max((b["conf"] for b in found), default=0.0),
        "false": len(boxes) - (1 if found else 0),
        "ms": took * 1000,
    }


def table(results):
    def rate(rows):
        return f"{sum(r['hit'] for r in rows)}/{len(rows)}" if rows else "-"

    with_car = [r for r in results if r["car"] and r["visible"]]
    lines = [
        f"{site:13s} {tilt:4d}°  "
        + "  ".join(
            rate(
                [
                    r
                    for r in with_car
                    if r["site"] == site and r["tilt"] == tilt and r["slant"] == slant
                ]
            )
            for slant in SLANTS
        )
        for site in SITES
        for tilt in TILTS
    ]
    hits = [r["conf"] for r in with_car if r["hit"]]
    empty = [r for r in results if not r["car"]]
    return "\n".join(
        [
            f"{'':13s} tilt   " + "  ".join(f"{s:3d}m" for s in SLANTS),
            *lines,
            f"recall {sum(r['hit'] for r in with_car)}/{len(with_car)}, mean conf on hits {sum(hits) / max(len(hits), 1):.2f}",
            f"false vehicles: {sum(r['false'] for r in empty)} in {len(empty)} frames without our car, {sum(r['false'] for r in with_car)} more beside it",
            f"median {sorted(r['ms'] for r in results)[len(results) // 2]:.0f} ms per frame (first excluded: {sorted(r['ms'] for r in results[1:])[len(results) // 2 - 1]:.0f} ms)",
        ]
    )


def score(out, model, imgsz, conf):
    detector = Detector(model, imgsz=imgsz, conf=conf)
    shots_rendered = json.loads((out / "shots.json").read_text())
    results = [scored(detector, out, shot, shots_rendered) for shot in shots_rendered]
    (out / f"scores-{Path(model).stem}-{imgsz}-{conf}.json").write_text(
        json.dumps(results, indent=1)
    )
    print(f"== {model} at {imgsz}, conf {conf} ==\n{table(results)}", flush=True)


def main():
    parser = argparse.ArgumentParser(
        description="How well does a detector see the simulated car? Renders a grid of drone views, then scores a model on them"
    )
    parser.add_argument("--out", default="out/eval")
    parser.add_argument("--render", action="store_true")
    parser.add_argument("--model", action="append", default=[])
    parser.add_argument("--imgsz", type=int, action="append", default=[])
    parser.add_argument("--conf", type=float, default=0.2)
    args = parser.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    if args.render:
        render(out)
    [score(out, model, imgsz, args.conf) for model in args.model for imgsz in (args.imgsz or [640])]


if __name__ == "__main__":
    main()
