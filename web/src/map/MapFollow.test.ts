import { describe, expect, it } from 'vitest';
import { bearingDegrees, inWindow, nearestLongitude, projectedWindow, vehicleOffScreen } from './MapFollow';
import type { TrackPoint } from './VehicleMap';

const SCALE = 10_000;
const rect = { left: 0, top: 0, right: 100, bottom: 100 };
const centre = { latitude: 47.395, longitude: 8.545 };

const northUp = (at: TrackPoint) => ({ x: 50 + (at.longitude - centre.longitude) * SCALE, y: 50 - (at.latitude - centre.latitude) * SCALE });

const turned = (degrees: number) => (at: TrackPoint) => {
  const { x, y } = northUp(at);
  const [dx, dy, radians] = [x - 50, y - 50, (degrees * Math.PI) / 180];
  return { x: 50 + dx * Math.cos(radians) - dy * Math.sin(radians), y: 50 + dx * Math.sin(radians) + dy * Math.cos(radians) };
};

describe('MapFollow', () => {
  it('a vehicle inside the map window needs no pointer', () => {
    const seen = projectedWindow(northUp, rect, centre);
    expect(inWindow(northUp, rect, centre, { latitude: 47.395, longitude: 8.545 })).toBe(true);
    expect(vehicleOffScreen(seen, { latitude: 47.398, longitude: 8.548 })).toBeNull();
    expect(vehicleOffScreen(null, { latitude: 0, longitude: 0 }), 'before the map reports its window nothing is off screen').toBeNull();
  });

  it('a vehicle off the map is pointed at with its distance and compass direction', () => {
    const seen = projectedWindow(northUp, rect, centre);
    const away = vehicleOffScreen(seen, { latitude: 47.395, longitude: 8.552 });
    expect(away?.bearing).toBeCloseTo(90, 0);
    expect(away?.text).toMatch(/^Vehicle \d+ m east of the map center$/);
    expect(vehicleOffScreen(seen, { latitude: 47.5, longitude: 8.545 })?.text).toMatch(/^Vehicle 11\.\d km north of the map center$/);
  });

  it('on a turned map a vehicle past a screen edge is off screen, even inside the box the corners span', () => {
    const corner = { latitude: 47.3995, longitude: 8.5495 };
    expect(inWindow(northUp, rect, centre, corner), 'the corner of a north-up map').toBe(true);
    expect(inWindow(turned(45), rect, centre, corner), 'swung outside once the map turns').toBe(false);
    expect(inWindow(turned(45), rect, centre, { latitude: 47.399, longitude: 8.545 }), 'and a point straight north is still shown').toBe(true);
  });

  it('a vehicle just across the antimeridian is on screen when the map looks at it', () => {
    const dateLine = { latitude: 0, longitude: 179.999 };
    const project = (at: TrackPoint) => ({ x: 50 + (at.longitude - dateLine.longitude) * SCALE, y: 50 - at.latitude * SCALE });
    expect(inWindow(project, rect, dateLine, { latitude: 0, longitude: -179.9985 })).toBe(true);
    expect(inWindow(project, rect, dateLine, { latitude: 0, longitude: -179.9 })).toBe(false);
    expect([nearestLongitude(179.999, -179.9985), nearestLongitude(8.5, 8.6), nearestLongitude(-179.9, 179.9)].map((it) => Number(it.toFixed(4)))).toEqual([180.0015, 8.6, -180.1]);
  });

  it('bearings run clockwise from north', () => {
    const from = { latitude: 0, longitude: 0 };
    expect([{ latitude: 1, longitude: 0 }, { latitude: 0, longitude: 1 }, { latitude: -1, longitude: 0 }, { latitude: 0, longitude: -1 }].map((to) => Math.round(bearingDegrees(from, to)))).toEqual([0, 90, 180, 270]);
  });
});
