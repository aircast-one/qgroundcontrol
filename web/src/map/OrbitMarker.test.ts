import { describe, expect, it } from 'vitest';
import { MINIMUM_CIRCLE_RADIUS_METRES } from './MissionLayers';
import { OrbitCircle, draggedOrbitRadius, orbitCircle, orbitHandles, orbitRing } from './OrbitMarker';
import { metresBetween } from './TerrainProfile';

describe('OrbitMarkerTest', () => {
  it('the circle is drawn only while the vehicle orbits, as FlyViewMap shows orbitMapCircle', () => {
    const turning = orbitCircle({ class: 'Orbit', orbiting: true, centre: { latitude: 47.0, longitude: 8.0 }, radiusMetres: 50 });
    expect(turning).toEqual(OrbitCircle({ centre: { latitude: 47.0, longitude: 8.0 }, radiusMetres: 50.0 }));
    const ring = orbitRing(turning);
    expect(ring[0]).toEqual(ring[ring.length - 1]);
    expect(orbitCircle({ class: 'Orbit', orbiting: false, centre: null })).toBeNull();
    expect(orbitCircle(null)).toBeNull();
    expect(orbitRing(null)).toEqual([]);
  });

  it('an orbit preview has a centre handle and a radius handle due east that drags the radius', () => {
    const preview = OrbitCircle({ centre: { latitude: 47.4, longitude: 8.5 }, radiusMetres: 30.0, clockwise: true });
    const handles = orbitHandles(preview);
    const last = handles[handles.length - 1];
    expect(handles[0]).toEqual(preview.centre);
    expect(last).toBeDefined();
    if (last === undefined) return;
    expect(Math.abs(metresBetween(preview.centre, last) - 30.0)).toBeLessThanOrEqual(0.5);
    expect(last.longitude).toBeGreaterThan(preview.centre.longitude);
    expect(Math.abs(draggedOrbitRadius(preview, last) - 30.0)).toBeLessThanOrEqual(0.5);
    expect(draggedOrbitRadius(preview, preview.centre)).toBe(MINIMUM_CIRCLE_RADIUS_METRES);
    expect(orbitHandles(null)).toEqual([]);
  });
});
