import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { PlanLayer, profileShown } from './PlanMapContent';
import { ProfilePoint, TerrainProfile, terrainProfile } from './TerrainProfile';
import { collisionSegments, heightRange, missingSpans, missionItemStatusShown, plannedRuns, profileLabel, tappedSequence, terrainRuns } from './TerrainProfileView';

const point = (distance: number, terrain: number | null, planned: number) => ProfilePoint({ distance, terrain, planned });

describe('ProfileShapeTest', () => {
  const profile = (...points: ProfilePoint[]) => TerrainProfile({ points });

  const at = (distance: number, planned: number, terrain: number | null = null) => point(distance, terrain, planned);

  it("the distance is the last point's", () => {
    expect(profile(at(0.0, 50.0), at(600.0, 50.0)).distance).toBeCloseTo(600.0, 9);
    expect(profile().distance).toBeCloseTo(0.0, 9);
  });

  it('the band covers the ground as well as the plan', () => {
    const walked = profile(at(0.0, 150.0, 100.0), at(600.0, 150.0, 900.0));

    expect(walked.lowest).toBeCloseTo(100.0, 9);
    expect(walked.highest).toBeCloseTo(900.0, 9);
  });

  it('with no ground the band is the plan alone', () => {
    const flown = profile(at(0.0, 40.0), at(600.0, 90.0));

    expect(flown.lowest).toBeCloseTo(40.0, 9);
    expect(flown.highest).toBeCloseTo(90.0, 9);
  });

  it('one ground reading is not a terrain line', () => {
    expect(profile(at(0.0, 50.0, 100.0), at(600.0, 50.0)).hasTerrain).toBe(false);
    expect(profile(at(0.0, 50.0, 100.0), at(600.0, 50.0, 120.0)).hasTerrain).toBe(true);
  });

  it('coverage is the share of the route with ground at both ends', () => {
    const half = profile(at(0.0, 50.0, 100.0), at(500.0, 50.0, 100.0), at(1000.0, 50.0));

    expect(half.terrainCoverage).toBeCloseTo(0.5, 9);
    expect(profile().terrainCoverage).toBeCloseTo(0.0, 9);
  });

  it('a plan flown at one altitude is flat but still drawable', () => {
    const level = profile(at(0.0, 50.0), at(600.0, 50.0));

    expect(level.flat).toBe(true);
    expect(level.drawable).toBe(true);
    expect(level.span).toBeCloseTo(1.0, 9);
  });

  it('a single point is not enough to draw', () => {
    expect(profile(at(0.0, 50.0)).drawable).toBe(false);
    expect(profile().drawable).toBe(false);
  });
});

describe('ProfileLabelTest', () => {
  const profile = (points: readonly ProfilePoint[], { lowest = '40 m', band = '40 m to 90 m', distance = '0.50 km' }: { lowest?: string; band?: string; distance?: string } = {}) =>
    TerrainProfile({ points, lowestText: lowest, bandText: band, distanceText: distance });

  it('a level plan states one altitude rather than a range', () => {
    expect(profileLabel(profile([point(0.0, null, 50.0), point(6430.0, null, 50.0)], { lowest: '50 m', distance: '6.43 km' }))).toBe('50 m AMSL · 6.43 km · ground height unknown');
  });

  it('a climbing plan states the band the core spelled', () => {
    expect(profileLabel(profile([point(0.0, null, 40.0), point(500.0, null, 90.0)]))).toBe('40 m to 90 m AMSL · 0.50 km · ground height unknown');
  });

  it('the band carries whatever unit the core spelled it in', () => {
    expect(profileLabel(profile([point(0.0, null, 40.0), point(500.0, null, 90.0)], { band: '131 ft to 295 ft', distance: '1640 ft' }))).toBe(
      '131 ft to 295 ft AMSL · 1640 ft · ground height unknown',
    );
  });

  it('terrain at points that do not join up is not one per cent of the route', () => {
    const measured = profile([point(0.0, 440.0, 440.0), point(0.0, null, 490.0), point(36.0, 444.0, 490.0)], { band: '430 m to 500 m', distance: '36 m' });
    expect(measured.terrainCoverage).toBe(0.0);
    expect(profileLabel(measured)).toBe('430 m to 500 m AMSL · 36 m · ground height at points, none along the route');
  });

  it('ground under part of the route says how much', () => {
    expect(profileLabel(profile([point(0.0, 40.0, 40.0), point(500.0, 45.0, 65.0), point(1000.0, null, 90.0)], { distance: '1.00 km' }))).toBe(
      '40 m to 90 m AMSL · 1.00 km · ground height for 50% of the route',
    );
  });

  it('a sliver of ground is never reported as none of the route', () => {
    expect(profileLabel(profile([point(0.0, 40.0, 40.0), point(1.0, 41.0, 41.0), point(1000.0, null, 90.0)], { distance: '1.00 km' }))).toBe(
      '40 m to 90 m AMSL · 1.00 km · ground height for 1% of the route',
    );
  });

  it('known ground height drops the caveat', () => {
    expect(profileLabel(profile([point(0.0, 40.0, 40.0), point(500.0, 45.0, 90.0)]))).toBe('40 m to 90 m AMSL · 0.50 km');
  });
});

describe('TerrainViewTest', () => {
  const view = (...points: string[]) => parseJson(`{"kind":"object","class":"TerrainProfile","points":[${points.join(',')}]}`);

  const at = (distance: number, planned: number, terrain: string) => `{"distance":${distance},"missionAltitude":${planned},"terrainAltitude":${terrain}}`;

  it("the profile is read straight from the core's points", () => {
    const profile = terrainProfile(view(at(0.0, 50.0, '948.0'), at(200.0, 100.0, '1000.0')));

    expect(profile.points.map((it) => it.distance)).toEqual([0.0, 200.0]);
    expect(profile.points.map((it) => it.planned)).toEqual([50.0, 100.0]);
    expect(profile.points.map((it) => it.terrain)).toEqual([948.0, 1000.0]);
    expect(profile.distance).toBeCloseTo(200.0, 9);
  });

  it('ground the core could not resolve stays unknown rather than becoming zero', () => {
    const profile = terrainProfile(view(at(0.0, 50.0, 'null'), at(200.0, 100.0, '1000.0')));

    expect(profile.points.map((it) => it.terrain)).toEqual([null, 1000.0]);
    expect(profile.lowest).toBeCloseTo(50.0, 9);
  });

  it('a point with no planned altitude stays on the distance axis but draws no flight line', () => {
    const profile = terrainProfile(view('{"distance":10.0,"terrainAltitude":900.0}'));

    expect(profile.points.length).toBe(1);
    expect(Number.isNaN(profile.points[0]?.planned)).toBe(true);
  });

  it('an absent or empty view draws nothing', () => {
    expect(terrainProfile(null).points.length).toBe(0);
    expect(terrainProfile(view()).drawable).toBe(false);
  });
});

describe('HeightRangeTest', () => {
  const profile = (low: number, high: number, lowText = '', highText = '', bandText = '') =>
    TerrainProfile({ points: [point(0.0, null, low), point(100.0, null, high)], lowestText: lowText, highestText: highText, bandText });

  it('the band is spelled by the core, which gives both ends one precision', () => {
    expect(heightRange(profile(-10.0, 60.0, '-32.8 ft', '197 ft', '-33 ft to 197 ft'))).toBe('-33 ft to 197 ft AMSL');
  });

  it('a flat route names one height, not a range of one', () => {
    expect(heightRange(profile(50.0, 50.0, '50 m', '50 m', '50 m to 50 m'))).toBe('50 m AMSL');
  });

  it('markers and collisions come from the core and a tap picks the nearest item', () => {
    const profile = terrainProfile(
      parseJson(`{"points":[
            {"distance":0,"missionAltitude":100,"terrainAltitude":50},
            {"distance":100,"missionAltitude":100,"terrainAltitude":120,"collision":true},
            {"distance":200,"missionAltitude":100,"terrainAltitude":130,"collision":true},
            {"distance":1000,"missionAltitude":100,"terrainAltitude":50}],
            "markers":[{"sequence":0,"distance":0,"label":"T"},{"sequence":2,"distance":200,"label":"2","complex":{"endDistance":600,"lastSequence":7,"pattern":"Survey"}}]}`),
    );
    expect(profile.markers.map((it) => it.label)).toEqual(['T', '2']);
    expect(profile.markers[1]?.lastSequence).toBe(7);
    expect(collisionSegments(profile, 1000, 100).length).toBe(1);
    expect([10, 590, 400, 850].map((it) => tappedSequence(profile, 1000, it)), 'the exit line and the band between belong to the pattern; open ground selects nothing').toEqual([0, 2, 2, null]);
  });

  it('the profile follows showMissionItemStatus, shown by default like PlanViewSettings', () => {
    expect([null, { value: null }, { value: false }, { value: true }].map((it) => missionItemStatusShown(it))).toEqual([true, true, false, true]);
  });
});

describe('TerrainGapTest', () => {
  const profile = TerrainProfile({
    points: [point(0.0, 100.0, 150.0), point(100.0, 110.0, 150.0), point(200.0, null, 150.0), point(300.0, 120.0, 150.0), point(400.0, 130.0, 150.0)],
    band: [90.0, 160.0],
  });

  it('missing ground breaks the terrain line and is marked along the bottom like TerrainProfile', () => {
    expect(terrainRuns(profile, 400, 70).map((it) => it.length)).toEqual([2, 2]);
    expect(missingSpans(profile, 400)).toEqual([
      [100, 200],
      [200, 300],
    ]);
    expect(profile.lowest, "the chart uses the core's padded band").toBe(90.0);
    const waiting = terrainProfile(
      parseJson(`{"minAltitudeMeters":90,"maxAltitudeMeters":160,"points":[
            {"distance":0,"missionAltitude":150,"terrainAltitude":100},{"distance":100,"missionAltitude":150,"terrainAltitude":110},
            {"distance":100,"missionAltitude":null},{"distance":300,"missionAltitude":null},
            {"distance":300,"missionAltitude":150,"terrainAltitude":120},{"distance":400,"missionAltitude":150,"terrainAltitude":130}]}`),
    );
    expect(plannedRuns(waiting, 400, 70).map((it) => it.length), 'a span with no heights yet keeps its points, draws no flight line there').toEqual([2, 2]);
    expect(missingSpans(waiting, 400)).toEqual([
      [100, 100],
      [100, 300],
      [300, 300],
    ]);
  });

  it('the profile and its chip belong to the Mission layer, as PlanView shows TerrainStatus only there', () => {
    const drawn = TerrainProfile({ points: [point(0.0, null, 10.0)] });
    expect(profileShown(PlanLayer.Mission, drawn)).toBe(true);
    expect(profileShown(PlanLayer.Fence, drawn)).toBe(false);
    expect(profileShown(PlanLayer.Mission, TerrainProfile({ points: [] }))).toBe(false);
  });
});
