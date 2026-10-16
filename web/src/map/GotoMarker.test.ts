import { describe, expect, it } from 'vitest';
import { GotoLocation, draggedGotoRadius, editedGoto, gotoArrows, gotoLocation, gotoPath, gotoRadiusHandle, gotoRing, previewGoto } from './GotoMarker';
import { MINIMUM_CIRCLE_RADIUS_METRES } from './MissionLayers';
import { metresBetween } from './TerrainProfile';

describe('GotoMarkerTest', () => {
  it('the go here marker carries a loiter ring only when the core sends a radius', () => {
    const fixedWing = gotoLocation({ gotoLocation: { latitude: 47.4, longitude: 8.5, loiterRadiusMetres: 80.0 } });
    expect(fixedWing).toEqual(GotoLocation({ at: { latitude: 47.4, longitude: 8.5 }, loiterRadiusMetres: 80.0 }));
    const ring = gotoRing(fixedWing);
    expect(ring[0]).toEqual(ring[ring.length - 1]);
    expect(ring.length).toBeGreaterThan(3);
    const copter = gotoLocation({ gotoLocation: { latitude: 47.4, longitude: 8.5, loiterRadiusMetres: null } });
    expect(copter).toEqual(GotoLocation({ at: { latitude: 47.4, longitude: 8.5 }, loiterRadiusMetres: null }));
    expect(gotoRing(copter)).toEqual([]);
    expect(gotoLocation({ gotoLocation: null })).toBeNull();
  });

  it('the loiter ring points its rotation arrows the way the circle was committed', () => {
    const anticlockwise = gotoLocation({ gotoLocation: { latitude: 47.4, longitude: 8.5, loiterRadiusMetres: 80.0, loiterClockwise: false } });
    expect(gotoArrows(anticlockwise).map((it) => it[1])).toEqual([270.0, 90.0]);
    expect(gotoArrows(anticlockwise === null ? null : { ...anticlockwise, loiterClockwise: true }).map((it) => it[1])).toEqual([90.0, 270.0]);
    expect(gotoArrows(anticlockwise === null ? null : { ...anticlockwise, loiterRadiusMetres: null })).toEqual([]);
  });

  it('a radius edit redraws the loiter ring with a drag handle due east, never below the circle minimum', () => {
    const committed = GotoLocation({ at: { latitude: 47.4, longitude: 8.5 }, loiterRadiusMetres: 80.0, loiterRadiusText: '80 m', loiterClockwise: true });
    const edit = { radiusMetres: 152.4, clockwise: false, unit: 'ft', metresPerUnit: 0.3048 };
    const shown = editedGoto(committed, edit);
    expect(shown).toEqual(GotoLocation({ at: { latitude: 47.4, longitude: 8.5 }, loiterRadiusMetres: 152.4, loiterRadiusText: '500 ft', loiterClockwise: false }));
    expect(editedGoto(committed, null)).toEqual(committed);
    expect(editedGoto({ ...committed, loiterRadiusMetres: null }, edit)).toEqual({ ...committed, loiterRadiusMetres: null });
    const handle = gotoRadiusHandle(shown);
    expect(handle).not.toBeNull();
    if (handle === null) return;
    expect(Math.abs(metresBetween(committed.at, handle) - 152.4)).toBeLessThanOrEqual(0.5);
    expect(handle.longitude).toBeGreaterThan(committed.at.longitude);
    expect(Math.abs(handle.latitude - 47.4)).toBeLessThanOrEqual(1e-4);
    expect(Math.abs(draggedGotoRadius(committed, handle) - 152.4)).toBeLessThanOrEqual(0.5);
    expect(draggedGotoRadius(committed, committed.at)).toBe(MINIMUM_CIRCLE_RADIUS_METRES);
    expect(gotoRadiusHandle({ ...committed, loiterRadiusMetres: null })).toBeNull();
  });

  it('while confirming the target shows with a path, and an arrived target is no longer drawn', () => {
    const target = { latitude: 47.398, longitude: 8.546 };
    const aircraft = { latitude: 47.397, longitude: 8.545 };
    expect(previewGoto(null, target)?.at).toEqual(target);
    expect(gotoPath(aircraft, target)).toEqual([aircraft, target]);
    expect(gotoPath(aircraft, null)).toEqual([]);
    const arrived = gotoLocation({ gotoLocation: { latitude: 47.398, longitude: 8.546, arrived: true, progressText: 'Arrived' } });
    expect(arrived?.progressText).toBe('Arrived');
    expect(previewGoto(arrived, null)).toBeNull();
  });
});
