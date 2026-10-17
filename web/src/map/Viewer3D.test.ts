import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { type LonLat, type PathMarker, type Point3D, type Slab, buildingFeatures, lifted, pathLabels, pathMarkers, pathSlabs, pickedMarker, quadFrame, ribbon, scene3d, selectedAfterTap, selectionKept, signedArea, vehicleSlabs, wound } from './Viewer3D';

const point3d = (lon: number, lat: number, alt: number): Point3D => ({ lon, lat, alt });

const average = (values: readonly number[]): number => values.reduce((sum, it) => sum + it, 0) / values.length;

describe('Viewer3DTest', () => {
  it("the core's buildings become extruded polygons centred on the file's bounds", () => {
    const view = parseJson(`{"class":"Viewer3D","available":true,"reason":null,"bounds":{"south":47.0,"west":8.0,"north":47.2,"east":8.2},
            "buildings":[{"outer":[[[8.01,47.01],[8.02,47.01],[8.02,47.02],[8.01,47.01]]],"inner":[],"height":6.0},{"outer":[[[8.0,47.0]]],"inner":[],"height":3.0}]}`);
    const scene = scene3d(view);
    expect(scene.buildings.length).toBe(1);
    expect(scene.centre?.[0]).toBeCloseTo(8.1, 9);
    expect(scene.centre?.[1]).toBeCloseTo(47.1, 9);
    const features = buildingFeatures(scene.buildings).features;
    expect(features.length).toBe(1);
    expect(features[0]?.properties?.height).toBe(6.0);
  });

  it('an unavailable view carries the reason to show', () => {
    const scene = scene3d(parseJson('{"available":false,"reason":"Turn on the 3D view in Settings.","buildings":[]}'));
    expect(scene.reason).toBe('Turn on the 3D view in Settings.');
  });

  it('outer rings wind counter-clockwise and holes clockwise so MapLibre keeps courtyards as holes', () => {
    const square: readonly LonLat[] = [
      [0.0, 0.0],
      [0.0, 1.0],
      [1.0, 1.0],
      [1.0, 0.0],
    ];
    expect(signedArea(wound(square, true)) > 0).toBe(true);
    expect(signedArea(wound(square, false)) < 0).toBe(true);
    expect(wound(square, true)[0]).toEqual(wound(square, true).at(-1));
  });

  it('a climbing segment floats as stepped slabs at its altitude', () => {
    const pieces = ribbon(point3d(8.0, 47.0, 20.0), point3d(8.0, 47.0003, 40.0), 'orange');
    expect(pieces.length).toBe(5);
    expect((pieces[0]?.base ?? NaN) < (pieces.at(-1)?.base ?? NaN)).toBe(true);
    expect(((pieces[0]?.base ?? NaN) + (pieces[0]?.top ?? NaN)) / 2).toBeCloseTo(22.0, 9);
  });

  it('markers float at their altitude and the vehicle comes from its own view', () => {
    const view = parseJson('{"markers":[{"at":[8.0,47.0,30.0],"name":"W","colour":"black"}],"segments":[]}');
    expect(pathSlabs(view).map((it) => it.base)).toEqual([28.5]);
    const one = vehicleSlabs(parseJson('{"vehicles":[{"at":[8.0,47.0,12.0],"heading":0.0}]}'));
    expect(one.length > 0 && one.every((it) => it.base >= 11.0 && it.base <= 13.0)).toBe(true);
    expect(vehicleSlabs(parseJson('{"vehicles":[{"at":[8.0,47.0,12.0],"heading":0.0},{"at":[8.001,47.0,30.0],"heading":90.0}]}')).length).toBe(2 * one.length);
    expect(vehicleSlabs(parseJson('{"vehicles":[]}'))).toEqual([]);
  });

  it("the quad frame's red front arms point along the heading", () => {
    const at = point3d(8.0, 47.0, 20.0);
    const centre = (slabs: readonly Slab[]) => average(slabs.flatMap((it) => it.corners).map((it) => it[0]));
    const frame = quadFrame(at, 90.0);
    expect(centre(frame.filter((it) => it.colour === '#E53935')) > at.lon).toBe(true);
    expect(centre(frame.filter((it) => it.colour === '#ECEFF1')) < at.lon).toBe(true);
    const north = average(
      quadFrame(at, 0.0)
        .filter((it) => it.colour === '#E53935')
        .flatMap((it) => it.corners)
        .map((it) => it[1]),
    );
    expect(north > at.lat).toBe(true);
  });

  it('a straight climb is one column from the lower to the higher altitude', () => {
    const column = ribbon(point3d(8.0, 47.0, 10.0), point3d(8.0, 47.0, 40.0), 'orange');
    expect(column.length).toBe(1);
    expect([column[0]?.base, column[0]?.top]).toEqual([9.5, 40.5]);
  });

  it('a steep climb leaves no gaps and a long leg stays capped', () => {
    const steep = ribbon(point3d(8.0, 47.0, 0.0), point3d(8.0, 47.0003, 20.0), 'orange');
    expect(steep.slice(1).every((b, index) => b.base <= (steep[index]?.top ?? NaN) + 1e-9)).toBe(true);
    expect(ribbon(point3d(8.0, 47.0, 10.0), point3d(8.0, 48.0, 10.0), 'orange').length).toBe(64);
  });

  it('every marker carries its label three metres above it like Viewer3DVehicleItems', () => {
    const labels = pathLabels(parseJson('{"markers":[{"at":[8.0,47.0,30.0],"name":"W","label":"3","colour":"black"},{"at":[8.1,47.1,0.0],"name":"","label":"","colour":"black"}]}'));
    expect(labels).toEqual([{ at: point3d(8.0, 47.0, 33.0), text: '3' }]);
  });

  it('a label rises on screen by its height foreshortened by the camera tilt', () => {
    expect(lifted([100, 200], 2.0, 0.0, 50.0)).toEqual([100, 200]);
    const tilted = lifted([100, 200], 2.0, 30.0, 50.0);
    expect(tilted[0]).toBe(100);
    expect(tilted[1]).toBeCloseTo(150, 4);
    expect(lifted([100, 200], 2.0, 30.0, -5.0)).toEqual([100, 200]);
  });

  it("a tapped marker turns yellow like Viewer3DInstancing's highlight and a miss picks nothing", () => {
    const view = parseJson('{"markers":[{"at":[8.0,47.0,30.0],"colour":"black"},{"at":[8.001,47.0,30.0],"colour":"green"}],"segments":[]}');
    expect(pathSlabs(view, new Set([1])).map((it) => it.colour)).toEqual(['black', '#FFFF00']);
    expect(pathMarkers(view).length).toBe(2);
    const onScreen = [[10, 10] as const, null, [100, 100] as const];
    expect(pickedMarker([95, 104], onScreen, 24)).toBe(2);
    expect(pickedMarker([50, 50], onScreen, 24)).toBeNull();
    expect(pickedMarker([20, 10], onScreen, 24)).toBe(0);
  });

  it("each vehicle keeps its own selected marker like Viewer3DModel's per-vehicle waypointInstancing", () => {
    const a = point3d(8.0, 47.0, 30.0);
    const b = point3d(8.001, 47.0, 30.0);
    const c = point3d(8.002, 47.0, 30.0);
    const markers: readonly PathMarker[] = [
      { mission: 0, at: a },
      { mission: 0, at: b },
      { mission: 1, at: c },
      { mission: 1, at: a },
    ];
    const first = selectedAfterTap(new Set(), markers, 1);
    const both = selectedAfterTap(first, markers, 3);
    expect(both).toEqual(new Set([1, 3]));
    expect(selectedAfterTap(both, markers, 0)).toEqual(new Set([0, 3]));
    expect(selectedAfterTap(both, markers, null)).toEqual(new Set());
    const firstMissionChanged: readonly PathMarker[] = [
      { mission: 0, at: a },
      { mission: 1, at: c },
      { mission: 1, at: a },
    ];
    expect(selectionKept(both, markers, firstMissionChanged)).toEqual(new Set([2]));
    expect(selectionKept(both, markers, markers)).toEqual(new Set([1, 3]));
  });

  it('markers carry the mission they belong to', () => {
    const view = parseJson('{"markers":[{"at":[8.0,47.0,30.0],"mission":0},{"at":[8.0,47.0,30.0],"mission":2}]}');
    expect(pathMarkers(view).map((it) => it.mission)).toEqual([0, 2]);
  });
});
