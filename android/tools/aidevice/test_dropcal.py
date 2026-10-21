import unittest

import numpy as np
from dropcal import separation_frame


class Separation(unittest.TestCase):
    def test_the_frame_the_payload_disappears_from_its_region_is_found(self):
        rng = np.random.default_rng(3)

        def frame(index):
            picture = rng.integers(60, 90, size=(240, 320, 3), dtype=np.uint8)
            if index < 37:
                picture[150:190, 140:170] = 230
            return picture

        frames = [frame(i) for i in range(60)]
        self.assertEqual(separation_frame(frames, (130, 140, 50, 60)), 37)


if __name__ == "__main__":
    unittest.main()
