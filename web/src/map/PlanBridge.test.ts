import { describe, expect, it } from 'vitest';
import { type Json, optJSONArray, parseJson } from '@/bridge/Json';
import { legArrows, missionPath } from './MissionLayers';
import { type MissionKind, allMissionItems, blockedReason, kindAllows, kindOffered, legsAfterLanding, missionItems, missionKinds, placed, planItemCount, planShape, routeEndsAfter, scanPatterns } from './PlanBridge';
import { TrackPoint } from './VehicleMap';

const json = (text: string): Json => parseJson(text)!;

const INT_MAX_VALUE = 2147483647;

describe('PlanBridgeTest', () => {
  const element = ({ latitude = 41.0, longitude = 44.0, specifies = true, sequence = 1, command = 'Waypoint', selected = false, withCoordinate = true } = {}): string => {
    const coordinate = withCoordinate && specifies ? `,"coordinate":{"latitude":${latitude},"longitude":${longitude}}` : '';
    return `{"flownLeg":${specifies},"sequence":${sequence},"name":"${command}","selected":${selected}${coordinate}}`;
  };

  const model = (...elements: string[]) => json(`{"kind":"object","items":[${elements.join(',')}]}`);

  it('a landing does not end the route, it only breaks the leg after it', () => {
    const plan = json(
      '{"kind":"object","items":[' +
        '{"sequence":0,"name":"Mission Start","flownLeg":false,"coordinate":{"latitude":41.0,"longitude":44.0}},' +
        '{"sequence":1,"name":"Waypoint","flownLeg":true,"coordinate":{"latitude":41.1,"longitude":44.1}},' +
        '{"sequence":2,"name":"Land","flownLeg":true,"endsRoute":true,"command":21,"coordinate":{"latitude":41.2,"longitude":44.2}},' +
        '{"sequence":3,"name":"Waypoint","flownLeg":true,"coordinate":{"latitude":41.3,"longitude":44.3}}]}',
    );

    const items = missionItems(plan);

    expect(
      items.map((it) => it.afterRouteEnds),
      'MissionController keeps going past a landing',
    ).toEqual([false, false, false, false]);
    expect(items[3]!.routed, 'the waypoint after the land is still routed').toBe(true);
    expect(items[3]!.legBroken, 'but the leg into it is not drawn').toBe(true);
  });

  it('a plan that never ends its route strands nothing', () => {
    const items = missionItems(model(element({ sequence: 1 }), element({ sequence: 2 })));

    expect(items.map((it) => it.afterRouteEnds)).toEqual([false, false]);
  });

  it('items carry their sequence command and position', () => {
    const items = missionItems(model(element({ latitude: 41.1, longitude: 44.1, sequence: 1, command: 'Takeoff' }), element({ latitude: 41.2, longitude: 44.2, sequence: 2, selected: true })));

    expect(items.length).toBe(2);
    expect(items[0]!.command).toBe('Takeoff');
    expect(items[0]!.sequence).toBe(1);
    expect(items[0]!.latitude).toBeCloseTo(41.1, 9);
    expect(items[1]!.selected, 'the editor selection, which is what the core calls it').toBe(true);
  });

  it('items without a coordinate of their own are not placed', () => {
    const items = missionItems(model(element({ specifies: false }), element({ withCoordinate: false }), element({ latitude: 41.3, longitude: 44.3 })));

    expect(items.length).toBe(1);
    expect(items[0]!.latitude).toBeCloseTo(41.3, 9);
  });

  it('an unusable coordinate is not placed', () => {
    expect(missionItems(model(element({ latitude: 0.0, longitude: 0.0 }))).length).toBe(0);
  });

  it('the index survives skipped items so writes address the right one', () => {
    const items = missionItems(model(element({ specifies: false }), element({ latitude: 41.4, longitude: 44.4 })));

    expect(items[0]!.index).toBe(1);
  });

  it('an empty or absent model yields nothing', () => {
    expect(missionItems(null).length).toBe(0);
    expect(missionItems(json('{"kind":"object","elements":[]}')).length).toBe(0);
    expect(missionItems(json('{"kind":"null"}')).length).toBe(0);
  });

  it('an item reads the altitude the core resolved for it', () => {
    const withAltitude = '{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0},"altitude":75.0}';

    expect(missionItems(model(withAltitude))[0]!.altitude).toBeCloseTo(75.0, 9);
  });

  it('an item the core gave no altitude reports it as unknown', () => {
    const plain = '{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}}';
    const nulled = '{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0},"altitude":null}';

    expect(missionItems(model(plain))[0]!.altitude).toBeNaN();
    expect(missionItems(model(nulled))[0]!.altitude).toBeNaN();
  });

  it('an item reads where it leaves as well as where it starts', () => {
    const withExit = '{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},"exitCoordinate":{"latitude":41.5,"longitude":44.5}}';

    expect(missionItems(model(withExit))[0]!.exit).toEqual(TrackPoint(41.5, 44.5));
  });

  it('an exit that repeats the entry or cannot be plotted is not carried', () => {
    const same = '{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},"exitCoordinate":{"latitude":41.0,"longitude":44.0}}';
    const unusable = '{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},"exitCoordinate":{"latitude":0.0,"longitude":0.0}}';

    expect(missionItems(model(same))[0]!.exit).toBeNull();
    expect(missionItems(model(unusable))[0]!.exit).toBeNull();
  });

  it('an item says whether the waypoint line goes through it', () => {
    const roi = '{"specifiesCoordinate":true,"isStandaloneCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0}}';

    expect(missionItems(model(roi))[0]!.routed).toBe(false);
    expect(missionItems(model(element()))[0]!.routed).toBe(true);
  });
});

describe('PlanItemCountTest', () => {
  const model = (count: number) => json(`{"kind":"object","items":[${Array.from({ length: count }, () => '{}').join(',')}]}`);

  it('the settings item is not something the pilot added', () => {
    expect(planItemCount(model(1))).toBe(0);
    expect(planItemCount(model(4))).toBe(3);
  });

  it('a plan that could not be read counts nothing rather than going negative', () => {
    expect(planItemCount(null)).toBe(0);
    expect(planItemCount(model(0))).toBe(0);
  });

  it('a takeoff and a return to launch are named by flag and command', () => {
    const plan = json(
      '{"kind":"object","items":[{},' +
        '{"kind":"takeoff","name":"Start"},' +
        '{"kind":"command","command":20,"endsRoute":true,"name":"irrelevant"},' +
        '{"kind":"waypoint","flownLeg":true}]}',
    );

    expect(planShape(plan)).toEqual(['takeoff', 'RTL', '1 after RTL']);
  });

  it('a plan with neither names nothing', () => {
    expect(planShape(model(3))).toEqual([]);
    expect(planShape(null)).toEqual([]);
  });

  it('an item added after the landing is named as such', () => {
    const plan = json(
      '{"kind":"object","items":[{},{"kind":"waypoint","flownLeg":true},' +
        '{"kind":"command","command":20,"endsRoute":true},{"kind":"waypoint","flownLeg":true},{"kind":"waypoint","flownLeg":true}]}',
    );

    expect(planShape(plan)).toEqual(['RTL', '2 after RTL']);
  });

  it('a plan that ends at its landing strands nothing', () => {
    const plan = json('{"kind":"object","items":[{},{"kind":"waypoint","flownLeg":true},{"kind":"command","command":20,"endsRoute":true}]}');

    expect(planShape(plan)).toEqual(['RTL']);
  });
});

describe('ScanPatternsTest', () => {
  const served = json(`{"kind":"object","kinds":[
    {"id":"waypoint","simple":true,"complexName":null,"enabled":true},
    {"id":"roi","simple":true,"complexName":null,"enabled":true},
    {"id":"survey","simple":false,"complexName":"Survey","enabled":true,"disabledReason":""},
    {"id":"corridor","simple":false,"complexName":"Corridor Scan","enabled":true,"disabledReason":""},
    {"id":"structure","simple":false,"complexName":"Structure Scan","enabled":false,
     "disabledReason":"This mission starts from the ground, so a takeoff has to come first."}]}`);

  it("only the patterns are offered, and by the core's own names", () => {
    const patterns = scanPatterns(missionKinds(served));

    expect(patterns.map((it) => it.id)).toEqual(['survey', 'corridor', 'structure']);
    expect(patterns.map((it) => it.label)).toEqual(['Survey', 'Corridor Scan', 'Structure Scan']);
  });

  it("a pattern the plan will not take is offered disabled, with the core's reason", () => {
    const structure = scanPatterns(missionKinds(served)).find((it) => it.id === 'structure')!;

    expect(structure.enabled).toBe(false);
    expect(structure.disabledReason).toContain('takeoff');
    expect(scanPatterns(missionKinds(served)).find((it) => it.id === 'survey')!.enabled).toBe(true);
  });

  it('a fourth pattern the core adds is offered without this head being changed', () => {
    const later = json('{"kind":"object","kinds":[{"id":"spiral","simple":false,"complexName":"Spiral Scan","enabled":true}]}');

    expect(scanPatterns(missionKinds(later)).map((it) => it.label)).toEqual(['Spiral Scan']);
  });

  it('no answer offers nothing rather than a list this head remembers', () => {
    expect(scanPatterns(missionKinds(null))).toEqual([]);
    expect(scanPatterns(missionKinds(json('{"kind":"null"}')))).toEqual([]);
  });
});

describe('MissionKindGateTest', () => {
  const empty = json(`{"kind":"object","kinds":[
    {"id":"takeoff","simple":true,"enabled":true,"disabledReason":""},
    {"id":"land","simple":true,"enabled":false,
     "disabledReason":"This mission starts from the ground, so a takeoff has to come first."},
    {"id":"roi","simple":true,"enabled":false,
     "disabledReason":"This mission starts from the ground, so a takeoff has to come first."}]}`);

  it('only the kind the plan will take is offered', () => {
    const kinds = missionKinds(empty);

    expect(kindAllows(kinds, 'takeoff')).toBe(true);
    expect(kindAllows(kinds, 'land')).toBe(false);
    expect(kindAllows(kinds, 'roi')).toBe(false);
  });

  it('a kind the core never mentioned stays offered rather than disappearing on silence', () => {
    expect(kindAllows(missionKinds(empty), 'spiral')).toBe(true);
    expect(kindAllows([], 'land')).toBe(true);
  });

  it('the reason is shown once, from the core, rather than beside every dead control', () => {
    expect(blockedReason(missionKinds(empty))).toContain('takeoff has to come');
    expect(blockedReason([])).toBeNull();
  });

  it("a kind the core leaves out of the list is not offered, like the rover's Takeoff", () => {
    const kinds: readonly MissionKind[] = [{ id: 'waypoint', label: '', title: 'Waypoint', enabled: true, disabledReason: '' }];
    expect([kindOffered(kinds, 'waypoint'), kindOffered(kinds, 'takeoff'), kindOffered([], 'takeoff')]).toEqual([true, false, true]);
  });
});

describe('LandingCutTest', () => {
  const plan = (...items: string[]) => json(`{"kind":"object","items":[${items.join(',')}]}`);

  const at = (longitude: number) => `{"kind":"waypoint","flownLeg":true,"coordinate":{"latitude":41.0,"longitude":${longitude}}}`;

  const rtl = '{"kind":"command","command":20,"endsRoute":true}';
  const landed = '{"kind":"land","endsRoute":true}';
  const settings = '{"kind":"settings","flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}}';

  it('an undrawable landing still cuts the route', () => {
    const items = missionItems(plan(settings, at(44.1), rtl, at(44.2)));

    expect(items.length).toBe(3);
    expect(items[1]!.routed).toBe(true);
    expect(items[2]!.routed).toBe(false);
  });

  it('nothing is after the landing when there is no landing', () => {
    expect(missionItems(plan(settings, at(44.1), at(44.2))).every((it) => it.routed)).toBe(true);
  });

  it('the route is cut at the item the core marked as ending it', () => {
    const items = optJSONArray(plan(settings, at(44.1), rtl, at(44.2)), 'items');

    expect(routeEndsAfter(items)).toBe(2);
    expect(routeEndsAfter(null)).toBe(INT_MAX_VALUE);
  });

  it('only a return ends the route, a landing just skips the leg after it', () => {
    const byCommand = optJSONArray(plan(settings, at(44.1), rtl, at(44.2)), 'items');
    const byLanding = optJSONArray(plan(settings, at(44.1), landed, at(44.2)), 'items');

    expect(routeEndsAfter(byCommand)).toBe(2);
    expect(routeEndsAfter(byLanding), 'MissionController breaks only at an RTL').toBe(INT_MAX_VALUE);
  });

  it('the leg out of a landing is not drawn, the ones after it are', () => {
    const land = '{"kind":"land","endsRoute":true,"command":21,"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.15}}';
    const items = missionItems(plan(settings, at(44.1), land, at(44.2), at(44.3)));
    expect(legsAfterLanding(optJSONArray(plan(settings, at(44.1), land, at(44.2), at(44.3)), 'items'))).toEqual(new Set([3]));
    const geometry = missionPath(items, false)?.geometry;
    expect(geometry?.type === 'MultiLineString' ? geometry.coordinates.map((line) => line.map((it) => it[0])) : null, "Don't draw segments immediately after a landing item").toEqual([
      [44.1, 44.15],
      [44.2, 44.3],
    ]);
    expect(legArrows(items, false).some((it) => it.at.longitude >= 44.15 && it.at.longitude <= 44.2)).toBe(false);
  });

  it('an item the core says is not a flown leg is never routed', () => {
    const roi = '{"kind":"roi","flownLeg":false,"coordinate":{"latitude":41.0,"longitude":44.9}}';
    const items = missionItems(plan(settings, at(44.1), roi, at(44.2)));

    expect(items.find((it) => it.command === '' && it.longitude === 44.9)!.routed).toBe(false);
    expect(items.find((it) => it.longitude === 44.1)!.routed).toBe(true);
  });

  it('the stray items after a landing are counted in the plan shape', () => {
    expect(planShape(plan(settings, rtl, at(44.2)))).toEqual(['RTL', '1 after RTL']);
  });

  it('an item with no usable coordinate is not plotted', () => {
    expect(placed(json('{"kind":"x"}'), 'coordinate'), 'Mission Start carries no coordinate at all').toBeNull();
    expect(placed(json('{"coordinate":{"latitude":null,"longitude":null}}'), 'coordinate'), 'a coordinate the core could not resolve arrives as nulls, and NaN is not plottable').toBeNull();
  });
});

describe('ComplexPatternTest', () => {
  const items = (text: string) => allMissionItems(json(text));

  it('a corridor scan is a pattern even though it is not a survey', () => {
    const plan = items(`{"kind":"object","items":[
      {"index":0,"sequence":0,"name":"Waypoint","kind":"waypoint","simple":true},
      {"index":1,"sequence":1,"name":"Survey","kind":"survey","simple":false},
      {"index":2,"sequence":2,"name":"Corridor Scan","kind":"corridor","simple":false},
      {"index":3,"sequence":3,"name":"Structure Scan","kind":"structure","simple":false}]}`);

    expect(plan[0]!.complexPattern).toBe(false);
    expect(plan.filter((it) => it.complexPattern).map((it) => it.index)).toEqual([1, 2, 3]);
  });

  it('an item the core said nothing about is treated as simple rather than asked about', () => {
    expect(items('{"kind":"object","items":[{"index":0,"sequence":0,"name":"Waypoint","kind":"waypoint"}]}')[0]!.complexPattern).toBe(false);
  });

  it('the camera question is asked of every pattern, not only the one named survey', () => {
    const plan = items('{"kind":"object","items":[{"index":2,"sequence":2,"name":"Corridor Scan","kind":"corridor","simple":false,"cameraShots":27}]}');

    expect(plan[0]!.complexPattern, 'a corridor inherits cameraCalc from TransectStyleComplexItem, so gating on the survey type hides camera work it really did').toBe(true);
  });
});

describe('DropMarkerTest', () => {
  it('the plan map read marks the waypoint before a drop, though the drop itself has no place', () => {
    const plan = allMissionItems(
      json(`{"kind":"object","items":[
        {"sequence":1,"name":"Waypoint","command":16,"specifiesCoordinate":true,"coordinate":{"latitude":47.0,"longitude":8.0}},
        {"sequence":2,"name":"Gripper Action","command":211}]}`),
    );

    expect(plan.filter((it) => it.placed).map((it) => it.drops)).toEqual([true]);
  });
});
