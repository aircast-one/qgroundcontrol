import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import type { MapHit } from './MissionEditing';
import { homeCovered } from './MissionLayers';
import { KIND_SURVEY, MissionItem } from './PlanBridge';
import { itemSeal, itemTitle } from './PlanItems';
import { advancedDetail, appendSequence, distanceWords, farFromAircraft, highestAltitude, placedPattern, planStats, routeStart, selectionTitle, startsFromAircraft, tapCloses, terrainWarning } from './PlanStrip';
import { Survey } from './SurveyBridge';
import { TrackPoint } from './VehicleMap';
import { NEWEST_PATTERN } from './VehicleSync';

const item = (index: number, altitude = NaN) =>
  MissionItem({ index, sequence: index, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: true, altitude, altitudeText: Number.isNaN(altitude) ? '' : `${Math.trunc(altitude)} m` });

const summary = parseJson('{"rows":[{"label":"Distance","value":"1.2 km"},{"label":"Time","value":"00:04:30"},{"label":"Furthest from launch","value":"640 m"}]}');

describe('PlanStripTest', () => {
  it("the strip reads items, the core's distance and time, and the highest altitude", () => {
    const items = [item(0, 488.0), item(1, 30.0), item(2, 80.0), item(3, 50.0)];

    expect(
      planStats(3, items, summary),
      "the planned home sits at ground height above sea level, so counting it would report the launch site's elevation as the plan's ceiling",
    ).toEqual([
      { label: 'Items', value: '3' },
      { label: 'Distance', value: '1.2 km' },
      { label: 'Time', value: '04:30' },
      { label: 'Max alt', value: '80 m' },
    ]);
  });

  it('a flight under an hour drops the empty hours so the time fits the strip', () => {
    const long = parseJson('{"rows":[{"label":"Time","value":"01:04:30"}]}');

    expect(planStats(0, [], long)[1]).toEqual({ label: 'Time', value: '01:04:30' });
  });

  it('a stat the core has not worked out is left off rather than shown as zero', () => {
    expect(planStats(1, [item(1)], null)).toEqual([{ label: 'Items', value: '1' }]);
    expect(highestAltitude([item(0, 488.0)])).toBeNull();
  });

  it('a selection that is not a mission item is named for what it is', () => {
    const items = [MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Survey', selected: true })];
    const hits: readonly MapHit[] = [
      { kind: 'FenceVertex', polygon: 0, vertex: 1 },
      { kind: 'CircleRadius', index: 0 },
      { kind: 'Rally', index: 2 },
      { kind: 'BreachReturn' },
      { kind: 'SurveyVertex', item: 2, vertex: 0 },
    ];

    expect(hits.map((it) => selectionTitle(it, items))).toEqual(['Fence corner', 'Circular fence', 'Rally point 3', 'Breach return point', 'Survey corner']);
  });

  it('a map tap closes an open editor first, and adds the next point once it is folded', () => {
    expect([tapCloses({ kind: 'Waypoint', index: 2 }, true), tapCloses({ kind: 'Waypoint', index: 2 }, false), tapCloses(null, true), tapCloses({ kind: 'Rally', index: 0 }, false)]).toEqual([true, false, false, true]);
  });

  it('a terrain conflict is counted in legs when the core names them', () => {
    expect([terrainWarning(1, 1), terrainWarning(3, 2), terrainWarning(0, 2), terrainWarning(0, 0)]).toEqual(['1 leg hits the terrain', '3 legs hit the terrain', '2 items hit the terrain', null]);
  });

  it('advanced names the mission items behind a waypoint with actions', () => {
    const folded = MissionItem({ index: 3, sequence: 3, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: true, altitude: 50.0, foldedCommands: 1 });
    expect(advancedDetail(folded)).toBe('Mission items 3–4');
    expect(advancedDetail(MissionItem({ index: 3, sequence: 3, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: true, altitude: 50.0 }))).toBeNull();
  });

  it('a takeoff on the planned home hides home, so the map and the strip both read T', () => {
    const home = MissionItem({ index: 0, sequence: 0, latitude: 41.0, longitude: 44.0, command: 'Home', selected: false, altitude: NaN });
    const takeoff = MissionItem({ index: 1, sequence: 1, latitude: 41.0, longitude: 44.0, command: 'Takeoff', selected: false, altitude: 50.0, abbreviation: 'Takeoff' });
    const away = { ...takeoff, latitude: 41.001 };
    const plain = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0 });
    expect([homeCovered([home, takeoff]), homeCovered([home, away])]).toEqual([true, false]);
    expect([itemSeal(home), itemSeal(takeoff), itemSeal(plain)]).toEqual(['H', 'T', '2']);
    expect([itemTitle(takeoff), itemTitle(plain)]).toEqual(['Takeoff', 'Waypoint 2']);
  });

  it("a template's pattern is the newest one, a rail pattern the one just inserted", () => {
    const surveys = [2, 5].map((index) => Survey({ index, area: [], transects: [], cameraShots: 0, kind: KIND_SURVEY, shape: '', property: 'surveyAreaPolygon' }));
    expect([placedPattern(surveys, NEWEST_PATTERN)?.index ?? null, placedPattern(surveys, 2)?.index ?? null, placedPattern(surveys, 7)?.index ?? null]).toEqual([5, 2, null]);
  });

  it('with nothing selected the core weighs new items at the end, where they are added', () => {
    const items = [
      ...[0, 1, 2].map((it) => MissionItem({ index: it, sequence: it, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0 })),
      MissionItem({ index: 3, sequence: 5, latitude: 41.0, longitude: 44.0, command: 'Survey', selected: false, altitude: 50.0 }),
    ];
    expect(appendSequence(items)).toBe(5);
    expect(appendSequence([])).toBe(0);
  });

  it('a route that begins more than a kilometre from the aircraft is flagged with the distance', () => {
    const home = MissionItem({ index: 0, sequence: 0, latitude: 37.0, longitude: -122.0, command: 'Home', selected: false, altitude: NaN });
    const takeoff = MissionItem({ index: 1, sequence: 1, latitude: 39.237, longitude: -123.149, command: 'Takeoff', selected: false, altitude: 50.0 });
    const zurich = TrackPoint(47.397, 8.545);
    expect(routeStart([home, takeoff])).toEqual(TrackPoint(39.237, -123.149));
    expect(farFromAircraft(TrackPoint(39.2371, -123.1491), [home, takeoff])).toBeNull();
    expect(farFromAircraft(null, [home, takeoff])).toBeNull();
    expect(startsFromAircraft(farFromAircraft(zurich, [home, takeoff])!, false).slice(0, 9)).toBe('Starts 9,');
  });

  it("distances read in the operator's units, coarser as they grow", () => {
    expect([850.0, 1500.0, 9_412_000.0].map((it) => distanceWords(it, false))).toEqual(['850 m', '1.5 km', '9,412 km']);
    expect([150.0, 3000.0, 9_412_000.0].map((it) => distanceWords(it, true))).toEqual(['492 ft', '1.9 mi', '5,848 mi']);
  });
});
