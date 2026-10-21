import math
import os
import unittest
from pathlib import Path

import numpy as np
from timesync import delay, flow_rates, telemetry_rates

INJECTED_MS = 180


def heading(t):
    return 10.0 * math.sin(2 * math.pi * 0.2 * t) + 3.0 * math.sin(2 * math.pi * 0.5 * t)


class Delay(unittest.TestCase):
    def test_a_known_lag_between_two_rate_series_is_found(self):
        telemetry = [(t / 50, heading(t / 50)) for t in range(0, 600)]
        times, rates = telemetry_rates(telemetry)
        video = [
            (t, float(np.interp(t - INJECTED_MS / 1000, times, rates)))
            for t in np.arange(0.5, 11.5, 0.1)
        ]
        self.assertLessEqual(abs(delay(video, (times, rates))["delayMs"] - INJECTED_MS), 10)


CAMERA = Path(os.environ.get("VIRTUAL_CAMERA", ""))


@unittest.skipUnless(CAMERA.is_file(), "set VIRTUAL_CAMERA to a built virtual_camera binary")
class Rendered(unittest.TestCase):
    def test_a_panning_camera_over_real_imagery_recovers_the_injected_delay(self):
        from scene import VirtualCamera, looking_at

        home = (47.3977419, 8.5455938)
        base = looking_at(home, home, 80.0, -30.0, 0.0)
        camera = VirtualCamera(640, 360, binary=CAMERA)
        [
            camera.picture({**base, "heading": warm % 360}, [], settle_ms=2000)
            for warm in (-14, 0, 14)
        ]
        stamps = [i / 10 for i in range(120)]
        frames = [
            camera.picture(
                {**base, "heading": heading(t - INJECTED_MS / 1000) % 360},
                [],
                settle_ms=150,
            )[..., ::-1].copy()
            for i, t in enumerate(stamps)
        ]
        camera.close()
        telemetry = [(t / 50, heading(t / 50) % 360) for t in range(0, 650)]
        found = delay(flow_rates(frames, stamps, 70.0), telemetry_rates(telemetry))
        print(found)
        self.assertLessEqual(abs(found["delayMs"] - INJECTED_MS), 30, found)


if __name__ == "__main__":
    unittest.main()
