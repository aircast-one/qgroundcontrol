import { describe, expect, it } from 'vitest';
import { FenceCircle, FencePolygon, FenceRow, RallyPoint, fenceRows, rallyRows } from './FenceBridge';
import { PlanLayer, layerOf } from './PlanMapContent';

describe('FenceRowsTest', () => {
  it('polygons then circles, named like GeoFenceEditor rows with Inclusion or Exclusion beneath', () => {
    const polygon = FencePolygon({ index: 0, inclusion: false, vertices: [], detailText: '4 corners' });
    const circle = FenceCircle({ index: 0, inclusion: true, centre: { latitude: 0.0, longitude: 0.0 }, radius: 137.0, detailText: '137 m radius', kindText: 'Circle 1', radiusUnits: 'm' });
    expect(fenceRows([polygon], [circle])).toEqual([
      FenceRow({ index: 0, circle: false, title: 'Polygon 1', detail: 'Exclusion', inclusion: false }),
      FenceRow({ index: 0, circle: true, title: 'Circle 1', detail: 'Inclusion', radius: '137 m' }),
    ]);
  });

  it('rally points are numbered from one with their height and position', () => {
    const point = RallyPoint({ index: 1, latitude: 41.7151, longitude: 44.8271, altitude: 50.0, altitudeUnits: 'm' });
    expect(rallyRows([point])).toEqual([FenceRow({ index: 1, circle: false, title: 'Rally 2', detail: '50 m · 41.715100, 44.827100' })]);
    const unknown = rallyRows([{ ...point, altitude: NaN }]);
    expect(unknown.length).toBe(1);
    expect(unknown[0]?.detail).toBe('41.715100, 44.827100');
  });

  it("selecting a fence or rally shape opens its layer, a waypoint the mission's", () => {
    expect(layerOf({ kind: 'Circle', index: 0 })).toBe(PlanLayer.Fence);
    expect(layerOf({ kind: 'FenceVertex', polygon: 0, vertex: 1 })).toBe(PlanLayer.Fence);
    expect(layerOf({ kind: 'Rally', index: 0 })).toBe(PlanLayer.Rally);
    expect(layerOf({ kind: 'Waypoint', index: 2 })).toBe(PlanLayer.Mission);
    expect(layerOf({ kind: 'ShapeCentre', fence: false, owner: 3 })).toBe(PlanLayer.Mission);
    expect(layerOf(null)).toBeNull();
  });
});
