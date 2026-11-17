import { describe, expect, it } from 'vitest';
import { bearingDegrees, inWindow, vehicleOffScreen } from './MapFollow';

const window = [
  { latitude: 47.40, longitude: 8.54 },
  { latitude: 47.40, longitude: 8.55 },
  { latitude: 47.39, longitude: 8.55 },
  { latitude: 47.39, longitude: 8.54 },
];

describe('MapFollow', () => {
  it('a vehicle inside the map window needs no pointer', () => {
    expect(inWindow(window, { latitude: 47.395, longitude: 8.545 })).toBe(true);
    expect(vehicleOffScreen(window, { latitude: 47.395, longitude: 8.545 })).toBeNull();
    expect(inWindow([], { latitude: 0, longitude: 0 }), 'before the map reports its window nothing is off screen').toBe(true);
  });

  it('a vehicle off the map is pointed at with its distance and compass direction', () => {
    const away = vehicleOffScreen(window, { latitude: 47.395, longitude: 8.552 });
    expect(away?.bearing).toBeCloseTo(90, 0);
    expect(away?.text).toMatch(/^Vehicle \d+ m east of the map center$/);
    expect(vehicleOffScreen(window, { latitude: 47.5, longitude: 8.545 })?.text).toMatch(/^Vehicle 11\.\d km north of the map center$/);
  });

  it('bearings run clockwise from north', () => {
    const from = { latitude: 0, longitude: 0 };
    expect([{ latitude: 1, longitude: 0 }, { latitude: 0, longitude: 1 }, { latitude: -1, longitude: 0 }, { latitude: 0, longitude: -1 }].map((to) => Math.round(bearingDegrees(from, to)))).toEqual([0, 90, 180, 270]);
  });
});
