import unittest

from detfeed import car_box


class CarBox(unittest.TestCase):
    def test_the_box_stands_on_the_car_s_picture_point(self):
        box = car_box({"x": 0.4, "y": 0.6}, 40.0)
        self.assertAlmostEqual(box["x"] + box["w"] / 2, 0.4, places=3)
        self.assertAlmostEqual(box["y"] + box["h"], 0.6, places=3)
        self.assertTrue(box["target"])

    def test_a_farther_car_draws_a_smaller_box(self):
        self.assertLess(car_box({"x": 0.5, "y": 0.5}, 200.0)["w"], car_box({"x": 0.5, "y": 0.5}, 40.0)["w"])


if __name__ == "__main__":
    unittest.main()
