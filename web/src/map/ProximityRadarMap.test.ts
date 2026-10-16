import { describe, expect, it } from 'vitest';
import { pointAt } from './FenceBridge';
import { placedRadars, radarLines, radarReading } from './ProximityRadarMap';
import { metresBetween } from './TerrainProfile';
import { VehicleChoice } from './VehiclePicker';

describe('ProximityRadarMapTest', () => {
  const centre = { latitude: 47.4, longitude: 8.5 };

  it('hidden radar reads nothing and sectors without a distance are dropped', () => {
    expect(radarReading({ shown: false })).toBeNull();
    const reading = radarReading({ shown: true, maxMeters: 40, sectors: [{ bearing: 0, meters: 10 }, { bearing: 45, meters: null }] });
    expect(reading).toEqual({ maxMeters: 40.0, sectors: [[0.0, 10.0]] });
    expect(radarReading({ shown: true, maxMeters: null, sectors: [] })?.maxMeters).toBeNull();
  });

  it('limit circle closes and each arc spans its sector turned by heading', () => {
    const lines = radarLines(centre, 90.0, { maxMeters: 40.0, sectors: [[0.0, 10.0]] });
    expect(lines.map((it) => it[0])).toEqual(['#FFFFFF', '#FF0000']);
    const limit = lines[0]?.[1] ?? [];
    expect(limit[0]).toEqual(limit[limit.length - 1]);
    const arc = lines[1]?.[1] ?? [];
    expect(arc.every((it) => Math.abs(metresBetween(centre, it) - 10.0) <= 0.01)).toBe(true);
    expect(arc[0]?.latitude).toBeCloseTo(pointAt(centre, 10.0, 67.5).latitude, 9);
    expect(arc[arc.length - 1]?.longitude).toBeCloseTo(pointAt(centre, 10.0, 112.5).longitude, 9);
  });

  it('no max distance draws only the arcs', () => {
    expect(radarLines(centre, NaN, { maxMeters: null, sectors: [[180.0, 5.0]] }).length).toBe(1);
  });
});

describe('FleetRadarTest', () => {
  const reading = { maxMeters: 40.0, sectors: [[0.0, 10.0]] as const };

  it('every vehicle with sensors gets its own radar, the active one at its live position', () => {
    const fleet = [
      VehicleChoice({ id: 1, name: 'A', state: '', link: '', contactLost: false, active: true, latitude: 1.0, longitude: 1.0, radar: { ...reading, sectors: [[0.0, 10.0]] } }),
      VehicleChoice({ id: 2, name: 'B', state: '', link: '', contactLost: false, active: false, latitude: 47.0, longitude: 8.0, heading: 90.0, radar: { ...reading, sectors: [[0.0, 10.0]] } }),
      VehicleChoice({ id: 3, name: 'C', state: '', link: '', contactLost: false, active: false, latitude: 48.0, longitude: 8.0 }),
    ];
    const placed = placedRadars(fleet, { latitude: 47.5, longitude: 8.5 }, 10.0);
    expect(placed.map((it) => it.at)).toEqual([{ latitude: 47.5, longitude: 8.5 }, { latitude: 47.0, longitude: 8.0 }]);
    expect(placed.map((it) => it.heading)).toEqual([10.0, 90.0]);
  });
});
