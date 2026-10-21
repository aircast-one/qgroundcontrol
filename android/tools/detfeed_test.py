import unittest

from detfeed import car_box, delayed, in_picture


class CarBox(unittest.TestCase):
    def test_the_box_stands_on_the_car_s_picture_point(self):
        box = car_box({"x": 0.4, "y": 0.6}, 40.0)
        self.assertAlmostEqual(box["x"] + box["w"] / 2, 0.4, places=3)
        self.assertAlmostEqual(box["y"] + box["h"], 0.6, places=3)
        self.assertTrue(box["target"])

    def test_a_farther_car_draws_a_smaller_box(self):
        self.assertLess(car_box({"x": 0.5, "y": 0.5}, 200.0)["w"], car_box({"x": 0.5, "y": 0.5}, 40.0)["w"])


class InPicture(unittest.TestCase):
    def test_a_detector_only_sees_what_is_inside_the_frame(self):
        self.assertTrue(in_picture({"x": 0.5, "y": 0.9}))
        self.assertFalse(in_picture({"x": 0.5, "y": -0.35}))
        self.assertFalse(in_picture(None))


class Delayed(unittest.TestCase):
    def test_a_frame_comes_out_only_once_it_is_as_old_as_the_latency(self):
        samples = [(t / 10, [t]) for t in range(10)]
        self.assertEqual([boxes[0] for _, boxes in delayed(iter(samples), 0.3)], list(range(7)))

    def test_no_latency_passes_every_frame_straight_through(self):
        samples = [(t / 10, [t]) for t in range(5)]
        self.assertEqual([boxes[0] for _, boxes in delayed(iter(samples), 0)], list(range(5)))


if __name__ == "__main__":
    unittest.main()
