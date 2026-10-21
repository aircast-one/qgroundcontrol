import json
import math
import subprocess
from pathlib import Path

import numpy as np

EARTH_RADIUS_M = 6_371_000
ROOT = Path(__file__).resolve().parents[3]
CAMERA = ROOT / "groundstation" / "target" / "release" / "examples" / "virtual_camera"


def moved(latitude, longitude, north_m, east_m):
    return (
        latitude + math.degrees(north_m / EARTH_RADIUS_M),
        longitude + math.degrees(east_m / (EARTH_RADIUS_M * math.cos(math.radians(latitude)))),
    )


def car(latitude, longitude, heading_deg, speed=0.01):
    return {
        "latitude": latitude,
        "longitude": longitude,
        "aboveHome": 0.0,
        "northSpeed": speed * math.cos(math.radians(heading_deg)),
        "eastSpeed": speed * math.sin(math.radians(heading_deg)),
        "ageMs": 0.0,
    }


def looking_at(target, home, slant_m, tilt_deg, heading_deg, fov=70.0, imagery="Bing Satellite"):
    down = math.radians(-tilt_deg)
    altitude, back = slant_m * math.sin(down), slant_m * math.cos(down)
    latitude, longitude = moved(
        target[0],
        target[1],
        -back * math.cos(math.radians(heading_deg)),
        -back * math.sin(math.radians(heading_deg)),
    )
    return {
        "available": True,
        "latitude": latitude,
        "longitude": longitude,
        "aboveHome": altitude,
        "homeLatitude": home[0],
        "homeLongitude": home[1],
        "heading": heading_deg,
        "pitch": tilt_deg,
        "roll": 0.0,
        "fov": fov,
        "imagery": imagery,
        "northSpeed": 0.0,
        "eastSpeed": 0.0,
        "climbSpeed": 0.0,
        "rollRate": 0.0,
    }


class VirtualCamera:
    def __init__(self, width=1280, height=720, binary=CAMERA):
        self.width, self.height = width, height
        self.process = subprocess.Popen(
            [str(binary), "--width", str(width), "--height", str(height)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )

    def picture(self, pose, cars, settle_ms=0):
        request = {"pose": pose, "cars": cars, "settleMs": settle_ms}
        self.process.stdin.write((json.dumps(request) + "\n").encode())
        self.process.stdin.flush()
        size = self.width * self.height * 3
        raw = self.process.stdout.read(size)
        return np.frombuffer(raw, dtype=np.uint8).reshape(self.height, self.width, 3)

    def close(self):
        self.process.stdin.close()
        self.process.wait(timeout=10)
