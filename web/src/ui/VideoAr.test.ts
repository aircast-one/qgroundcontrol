import { describe, expect, it } from 'vitest';
import { arOffset, arReading } from '@/ui/VideoAr';

describe('VideoArTest', () => {
  it("the core's projection becomes marks and legs on the picture", () => {
    const reading = arReading({
      available: true,
      home: { x: 0.2, up: -0.1, label: 'H', text: '120 m' },
      waypoints: [{ x: 0.5, up: 0.0, label: '1' }],
      route: [{ from: { x: 0.2, up: -0.1 }, to: { x: 0.5, up: 0.0 } }],
    })!;
    expect(reading.home).toEqual({ at: { x: 0.2, up: -0.1 }, label: 'H', text: '120 m' });
    expect(reading.waypoints.map((it) => it.label)).toEqual(['1']);
    expect(reading.route).toHaveLength(1);
    expect(arReading({ available: false })).toBeNull();
  });

  it("a spot is placed from the centre line in widths, so the frame's aspect does not distort it", () => {
    const centre = arOffset({ x: 0.5, up: 0 }, 1600, 900);
    expect(centre.x).toBeCloseTo(800, 3);
    expect(centre.y).toBeCloseTo(450, 3);
    expect(arOffset({ x: 0.5, up: 0.1 }, 1600, 900).y).toBeCloseTo(450 - 160, 3);
  });
});
