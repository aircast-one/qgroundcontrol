import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { pointAt } from './FenceBridge';
import {
  GLIDE_SLOPE_KIND,
  LANDING_AREA_KIND,
  LandingPattern,
  approachPath,
  glideSlope,
  isLandingPattern,
  landingArea,
  landingAreaFeatures,
  landingLabelFeatures,
  landingLabels,
  landingLoiterFeatures,
  landingPathFeatures,
  landingPattern,
  landingText,
  loiterRings,
  structureScanLabels,
} from './LandingPattern';
import {
  HANDLE_KIND_LANDING,
  HANDLE_KIND_PROPERTY,
  LANDING_PLACE_APPROACH,
  LANDING_PLACE_TOUCHDOWN,
  LANDING_SHAPE_KIND,
  TERRAIN_COLLISION,
  VERTEX_INDEX_PROPERTY,
  WAYPOINT_EXIT_PROPERTY,
  WAYPOINT_ID_PROPERTY,
  WAYPOINT_LABEL_PROPERTY,
  WAYPOINT_SIDE_LABEL_PROPERTY,
  missionFeatures,
  vertexHandleFeatures,
} from './MissionLayers';
import { KIND_LAND, KIND_STRUCTURE, MissionItem } from './PlanBridge';
import { movedText } from './PlanItems';
import { selectedLanding, selectionSurvives } from './SelectionValidity';
import { metresBetween } from './TerrainProfile';

const parsed = (text: string): Json => JSON.parse(text) as Json;

const placed = (index: number, view: Json | null): LandingPattern => {
  const pattern = landingPattern(index, view);
  if (pattern === null) throw new Error('no pattern');
  return pattern;
};

describe('LandingPatternTest', () => {
  const whole = `
        {"landing":{"latitude":41.70,"longitude":44.82,"altitude":0},
         "slopeStart":{"latitude":41.71,"longitude":44.83,"altitude":40},
         "finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60},
         "loiterRadiusMetres":75.0,"loiterClockwise":true}
    `;

  it('a pattern is drawn from approach through slope start to the landing', () => {
    const pattern = placed(4, parsed(whole));
    expect(approachPath(pattern)).toEqual([pattern.finalApproach, pattern.slopeStart, pattern.landing]);
    expect(landingPathFeatures([pattern]).features.length).toBe(1);
  });

  it('an unset corner is absent, so the pattern is never plotted at null island', () => {
    const partial = placed(
      4,
      parsed(`{"landing":null,"slopeStart":null,
                     "finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60},
                     "loiterRadiusMetres":75.0}`),
    );
    expect(partial.landing).toBeNull();
    expect(approachPath(partial)).toEqual([partial.finalApproach]);
  });

  it('a pattern with one place draws no path, having nothing to join', () => {
    const one = placed(4, parsed('{"finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60}}'));
    expect(landingPathFeatures([one]).features.length).toBe(0);
  });

  it('the loiter circle is drawn around the approach, and only when it has a radius', () => {
    const pattern = placed(4, parsed(whole));
    expect(landingLoiterFeatures([pattern]).features.length, 'FWLandingPatternMapVisual shows the circle only with loiter-to-altitude').toBe(0);
    expect(landingLoiterFeatures([{ ...pattern, loiterToAltitude: true }]).features.length).toBe(1);
    const noRadius = placed(
      4,
      parsed(`{"finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60},
                     "landing":{"latitude":41.70,"longitude":44.82,"altitude":0}}`),
    );
    expect(landingLoiterFeatures([noRadius]).features.length).toBe(0);
  });

  it('an item with no pattern is not one, however the core says so', () => {
    expect(landingPattern(4, null)).toBeNull();
    expect(landingPattern(4, { kind: 'null' })).toBeNull();
    expect(landingPattern(4, { reason: 'only a fixed wing or a VTOL gets one' })).toBeNull();
    expect(landingPattern(4, { loiterRadiusMetres: 75.0 })).toBeNull();
  });
});

describe('LandingHandleTest', () => {
  const pattern = LandingPattern({
    index: 4,
    landing: { latitude: 41.7, longitude: 44.82 },
    slopeStart: { latitude: 41.71, longitude: 44.83 },
    finalApproach: { latitude: 41.72, longitude: 44.84 },
    loiterRadiusMetres: 75.0,
    loiterClockwise: true,
  });

  it('only the two places QGC lets you set get a handle', () => {
    const handles = vertexHandleFeatures([], [], [], [pattern]).features;
    expect(handles.length).toBe(2);
    expect(handles.map((it) => it.properties?.[VERTEX_INDEX_PROPERTY])).toEqual([LANDING_PLACE_APPROACH, LANDING_PLACE_TOUCHDOWN]);
    expect(handles.map((it) => it.properties?.[HANDLE_KIND_PROPERTY])).toEqual([HANDLE_KIND_LANDING, HANDLE_KIND_LANDING]);
  });

  it('an unplaced pattern offers nothing to drag', () => {
    const empty = { ...pattern, landing: null, slopeStart: null, finalApproach: null };
    expect(vertexHandleFeatures([], [], [], [empty]).features.length).toBe(0);
  });

  it('a selection on a landing place survives while the pattern does', () => {
    expect(selectionSurvives({ kind: 'LandingPlace', index: 4, place: LANDING_PLACE_APPROACH }, [], [], [], [], [], [pattern])).toBe(true);
    expect(!selectionSurvives({ kind: 'LandingPlace', index: 9, place: LANDING_PLACE_APPROACH }, [], [], [], [], [], [pattern])).toBe(true);
  });

  it('each place says which one moved', () => {
    expect(movedText({ kind: 'LandingPlace', index: 4, place: LANDING_PLACE_APPROACH }, [])).toBe('Moved the final approach');
    expect(movedText({ kind: 'LandingPlace', index: 4, place: LANDING_PLACE_TOUCHDOWN }, [])).toBe('Moved the touchdown');
  });
});

describe('IsLandingPatternTest', () => {
  it('a refusal is not a pattern, and neither is an absent view', () => {
    expect(!isLandingPattern(null)).toBe(true);
    expect(!isLandingPattern({ kind: 'null' })).toBe(true);
    expect(!isLandingPattern({ reason: 'only a fixed wing or a VTOL gets one' })).toBe(true);
  });

  it('a pattern with no places is still a pattern, which is the whole distinction', () => {
    const unplaced = { landing: null, slopeStart: null, finalApproach: null };
    expect(isLandingPattern(unplaced)).toBe(true);
    expect(landingPattern(4, unplaced)).toBeNull();
  });

  it("a loiter item's ring is its radius whichever way it turns", () => {
    const loiter = MissionItem({ index: 2, sequence: 2, latitude: 47.0, longitude: 8.0, command: '', selected: false, loiterRadius: -80.0 });
    const waypoint = MissionItem({ index: 1, sequence: 1, latitude: 47.1, longitude: 8.0, command: '', selected: false });
    expect(loiterRings([], [waypoint, loiter])).toEqual([[{ latitude: 47.0, longitude: 8.0 }, 80.0]]);
  });

  it('the landing area is a 15 by 100 metre box on the touchdown and the glide slope runs to the approach', () => {
    const landing = { latitude: 47.0, longitude: 8.0 };
    const slope = pointAt(landing, 400.0, 90.0);
    const approach = pointAt(landing, 800.0, 90.0);
    const straight = LandingPattern({ index: 3, landing, slopeStart: slope, finalApproach: approach, loiterRadiusMetres: null, loiterClockwise: true });
    const area = landingArea(straight) ?? [];
    expect(area.length).toBe(4);
    const corner = area[0];
    expect(Math.abs((corner === undefined ? NaN : metresBetween(landing, corner)) - Math.hypot(7.5, 50.0)), 'corners are the half-diagonal from touchdown').toBeLessThanOrEqual(0.5);
    expect(glideSlope(straight)?.at(-1), 'without loiter-to-altitude the slope reaches the final approach').toEqual(approach);
    expect(glideSlope({ ...straight, loiterRadiusMetres: 75.0 })?.at(-1), 'a radius alone is not loiter-to-altitude').toEqual(approach);
    expect(glideSlope({ ...straight, loiterRadiusMetres: 75.0, loiterToAltitude: true })?.at(-1), 'with it, the slope start').toEqual(slope);
    const labels = landingLabels({ ...straight, heights: { transition: '5 m*', midSlope: '52 m*', approach: '100.0 m' } }).map((it) => it.text);
    expect(labels, "FWLandingPatternMapVisual's two names and three heights").toEqual(['Landing Area', 'Glide Slope', '5 m*', '52 m*', '100.0 m']);
    expect(landingAreaFeatures([straight]).features.length, 'FWLandingPatternMapVisual draws both whether or not the item is current').toBe(2);
    expect(landingLabelFeatures([straight], 2).features.length, "only the current item's labels show").toBe(0);
  });

  it('a structure scan names its entry and exit', () => {
    const scan = MissionItem({ index: 2, sequence: 2, latitude: 47.0, longitude: 8.0, command: '', selected: false, kind: KIND_STRUCTURE, exit: { latitude: 47.1, longitude: 8.0 } });
    expect(structureScanLabels([scan]).map((it) => it.text)).toEqual(['Entry', 'Exit']);
    expect(
      structureScanLabels([{ ...scan, exit: null }]).map((it) => it.at.latitude),
      'exitCoordinateSameAsEntry: the core sends no exit, both labels sit on the entry',
    ).toEqual([scan.latitude, scan.latitude]);
  });

  it("a pattern's exit marker is flagged so dragging it never moves the entry", () => {
    const scan = MissionItem({ index: 2, sequence: 2, latitude: 47.0, longitude: 8.0, command: '', selected: false, kind: KIND_STRUCTURE, exit: { latitude: 47.1, longitude: 8.0 }, complexPattern: true });
    const marks = missionFeatures([scan]).features.map((it) => [it.properties?.[WAYPOINT_ID_PROPERTY], it.properties?.[WAYPOINT_EXIT_PROPERTY]]);
    expect(marks).toEqual([
      [2, false],
      [2, true],
    ]);
  });

  it('a colliding glide slope and loiter circle turn red like their QGC visuals', () => {
    const slope = LandingPattern({
      index: 3,
      landing: { latitude: 47.0, longitude: 8.0 },
      slopeStart: { latitude: 47.01, longitude: 8.0 },
      finalApproach: { latitude: 47.02, longitude: 8.0 },
      loiterRadiusMetres: 80.0,
      loiterClockwise: true,
      collides: true,
    });
    const shapes = Object.fromEntries(landingAreaFeatures([slope]).features.map((it) => [it.properties?.[LANDING_SHAPE_KIND], it.properties?.[TERRAIN_COLLISION]]));
    expect(shapes, 'only the glide slope takes the collision colour').toEqual({ [LANDING_AREA_KIND]: false, [GLIDE_SLOPE_KIND]: true });
    const loiter = MissionItem({ index: 4, sequence: 4, latitude: 47.0, longitude: 8.0, command: 'Loiter', selected: false, altitude: 50.0, loiterRadius: 80.0, terrainCollision: true });
    const rings = landingLoiterFeatures([], [loiter]).features;
    expect(rings.length).toBe(1);
    expect(rings[0]?.properties?.[TERRAIN_COLLISION]).toBe(true);
  });

  it('a VTOL landing pattern draws no landing area or glide slope, as VTOLLandingPatternMapVisual', () => {
    const vtol = LandingPattern({
      index: 3,
      landing: { latitude: 47.0, longitude: 8.0 },
      slopeStart: { latitude: 47.01, longitude: 8.0 },
      finalApproach: { latitude: 47.02, longitude: 8.0 },
      loiterRadiusMetres: 80.0,
      loiterClockwise: true,
      glideSlopeShown: false,
      heights: { transition: '1', midSlope: '2', approach: '3' },
    });
    expect(landingAreaFeatures([vtol]).features.length).toBe(0);
    expect(landingLabels(vtol)).toEqual([]);
  });

  it("a landing pattern is marked at its approach and touchdown like QGC's visuals", () => {
    const item = MissionItem({ index: 3, sequence: 7, latitude: 47.0, longitude: 8.0, command: 'Landing Pattern', selected: false, altitude: 30.0, kind: KIND_LAND, foldedCommands: 4 });
    const pattern = LandingPattern({
      index: 3,
      landing: { latitude: 47.0, longitude: 8.0 },
      slopeStart: { latitude: 47.01, longitude: 8.0 },
      finalApproach: { latitude: 47.02, longitude: 8.0 },
      loiterRadiusMetres: 80.0,
      loiterClockwise: true,
    });
    const marks = missionFeatures([item], null, [pattern]).features.map((it) => [it.properties?.[WAYPOINT_LABEL_PROPERTY], it.properties?.[WAYPOINT_SIDE_LABEL_PROPERTY]]);
    expect(marks).toEqual([
      ['7', 'Approach'],
      ['11', 'Land'],
    ]);
    const loiter = missionFeatures([item], null, [{ ...pattern, loiterToAltitude: true }]).features[0]?.properties?.[WAYPOINT_SIDE_LABEL_PROPERTY];
    expect(loiter).toBe('Loiter');
  });
});

describe('LandingTextTest', () => {
  const pattern = (radius: string, clockwise = true, index = 3): LandingPattern =>
    LandingPattern({ index, landing: { latitude: 41.0, longitude: 44.0 }, slopeStart: null, finalApproach: null, loiterRadiusMetres: 75.0, loiterClockwise: clockwise, loiterRadiusText: radius });

  it('a drawn circle says how wide it is and which way round', () => {
    expect(landingText(pattern('75.0 m'))).toBe('circles 75.0 m clockwise');
    expect(landingText(pattern('75.0 m', false))).toBe('circles 75.0 m anticlockwise');
  });

  it('a pattern the core gave no radius for says nothing rather than circling zero', () => {
    expect(landingText(pattern(''))).toBeNull();
    expect(landingText(null)).toBeNull();
  });

  it('a landing picked from the list is the same one as picked by its touchdown', () => {
    const landings = [pattern('75.0 m', true, 3)];
    expect(selectedLanding({ kind: 'Waypoint', index: 3 }, landings)?.index).toBe(3);
    expect(selectedLanding({ kind: 'LandingPlace', index: 3, place: LANDING_PLACE_TOUCHDOWN }, landings)).toEqual(selectedLanding({ kind: 'Waypoint', index: 3 }, landings));
    expect(selectedLanding({ kind: 'Waypoint', index: 9 }, landings)).toBeNull();
    expect(selectedLanding(null, landings)).toBeNull();
  });
});
