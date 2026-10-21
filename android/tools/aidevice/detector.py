import hashlib
import urllib.request
from pathlib import Path

import numpy as np
import torch
from ultralytics import YOLO

PINNED = {
    "visdrone-yolov8s.pt": (
        "https://huggingface.co/dronefreak/visdrone-yolov8s/resolve/main/best.pt",
        "29dae68ac5028cedac1f396333ac89cd9c74232691155243d70d9a1c16cdb24e",
    ),
    "visdrone-yolo11m.pt": (
        "https://huggingface.co/dronefreak/visdrone-yolo11m/resolve/main/best.pt",
        "d29ca86813dbaf7bd1d9d0c1387ce39cc50840635de2696bc4f25dc77ca3769d",
    ),
}

LABELS = {"car": "car", "van": "car", "truck": "truck", "bus": "bus"}


def fetched(model):
    path = Path(__file__).parent / model
    if model not in PINNED or path.exists():
        return str(path) if path.exists() else model
    url, sha256 = PINNED[model]
    data = urllib.request.urlopen(url, timeout=120).read()
    if hashlib.sha256(data).hexdigest() != sha256:
        raise RuntimeError(f"{model} from {url} does not match its pinned sha256")
    path.write_bytes(data)
    return str(path)


def device():
    return (
        "mps"
        if torch.backends.mps.is_available()
        else "cuda"
        if torch.cuda.is_available()
        else "cpu"
    )


class Detector:
    def __init__(self, model, imgsz=640, conf=0.2):
        self.model = YOLO(fetched(model))
        self.imgsz, self.conf, self.device = imgsz, conf, device()

    def boxes(self, rgb, track=False):
        height, width = rgb.shape[:2]
        bgr = np.ascontiguousarray(rgb[:, :, ::-1])
        options = {"imgsz": self.imgsz, "conf": self.conf, "device": self.device, "verbose": False}
        result = (
            self.model.track(
                bgr, persist=True, tracker=str(Path(__file__).parent / "tracker.yaml"), **options
            )
            if track
            else self.model.predict(bgr, **options)
        )[0]
        ids = (
            result.boxes.id.int().tolist()
            if result.boxes.id is not None
            else [None] * len(result.boxes)
        )
        return [
            {
                "x": x1 / width,
                "y": y1 / height,
                "w": (x2 - x1) / width,
                "h": (y2 - y1) / height,
                "label": LABELS[result.names[kind]],
                "conf": round(score, 3),
                "id": track_id,
            }
            for (x1, y1, x2, y2), kind, score, track_id in zip(
                result.boxes.xyxy.tolist(),
                result.boxes.cls.int().tolist(),
                result.boxes.conf.tolist(),
                ids,
                strict=True,
            )
            if result.names[kind] in LABELS
        ]
