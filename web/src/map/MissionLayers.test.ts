import type { FeatureCollection } from 'geojson';
import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { EditableShape } from './EditableShape';
import { FenceCircle, FencePolygon, RallyPoint, azimuthBetween, circleRing, pointAt } from './FenceBridge';
import { clickMarkerFeatures } from './GotoMarker';
import { shotPoints } from './ItemCamera';
import { type MapHit, handleHit } from './MissionEditing';
import {
  CIRCLE_INDEX_PROPERTY,
  CROWDED_ITEMS,
  CROWDED_RADIUS,
  HANDLE_KIND_CIRCLE,
  HANDLE_KIND_FENCE,
  HANDLE_KIND_FENCE_CIRCLE_RADIUS,
  HANDLE_KIND_LOITER_RADIUS,
  HANDLE_KIND_LOITER_ROTATION,
  HANDLE_KIND_PROPERTY,
  HANDLE_KIND_SURVEY,
  KEEPS_IN_PROPERTY,
  KEEP_IN_COLOUR,
  KEEP_OUT_COLOUR,
  LAND_COLOUR,
  LOITER_COLOUR,
  MARKER_RADIUS,
  MAV_CMD_NAV_LOITER_TIME,
  POLYGON_INDEX_PROPERTY,
  RETURN_COLOUR,
  SHAPE_PATH_PROPERTY,
  SPLIT_INVOKABLE_PROPERTY,
  START_COLOUR,
  SURVEY_COLLISION,
  TAKEOFF_COLOUR,
  VERTEX_INDEX_PROPERTY,
  WAYPOINT_COLOUR,
  WAYPOINT_ID_PROPERTY,
  WAYPOINT_LABEL_PROPERTY,
  WAYPOINT_SIDE_LABEL_PROPERTY,
  WAYPOINT_RADIUS_PROPERTY,
  WAYPOINT_SELECTED_PROPERTY,
  circleEdge,
  collisionLegFeatures,
  crowded,
  draggedCircleRadius,
  draggedLoiterRadius,
  fenceFeatures,
  flownRoute,
  legSplit,
  loiterHandleFeatures,
  loiterRotationArrows,
  markerRadius,
  markerStroke,
  midpointFeatures,
  missionFeatures,
  missionPath,
  radiusHandleFeatures,
  rallyFeatures,
  shadedArea,
  shotFeatures,
  DROP_SIDE,
  dropsAt,
  withDrops,
  sideLabel,
  surveyAreaFeatures,
  surveyLineFeatures,
  surveyTransectFeatures,
  vertexHandleFeatures,
  waypointColour,
  waypointLabel,
} from './MissionLayers';
import { KIND_SURVEY, MAV_CMD_NAV_RETURN_TO_LAUNCH, MissionItem, linksStartToHome, missionItems } from './PlanBridge';
import { layersText, moveGeneration, patternName, writeDragStep, writeMove } from './PlanItems';
import { ShapeTarget, circleAround, circleRadius, fencePath, liveCircles, polygonCentre, positionTitle, shapeCentreHit, shapeVertices } from './PolygonTools';
import { selectionSurvives } from './SelectionValidity';
import { SHAPE_AREA, SHAPE_LINE, Survey } from './SurveyBridge';
import { collidingItems, collisionLegs, metresBetween } from './TerrainProfile';
import { type TrackPoint, vehicleFeatures } from './VehicleMap';

const parsed = (text: string): Json => JSON.parse(text) as Json;

const at = (latitude: number, longitude: number): TrackPoint => ({ latitude, longitude });

const near = (actual: number | null | undefined, expected: number, delta: number): void => expect(Math.abs((actual ?? NaN) - expected)).toBeLessThanOrEqual(delta);

const property = (collection: FeatureCollection, index: number, key: string): unknown => collection.features[index]?.properties?.[key] ?? null;

const properties = (collection: FeatureCollection, key: string): readonly unknown[] => collection.features.map((it) => it.properties?.[key] ?? null);

const pointsOf = (collection: FeatureCollection): readonly (readonly [number, number] | null)[] =>
  collection.features.map((it) => (it.geometry.type === 'Point' ? [it.geometry.coordinates[1] ?? NaN, it.geometry.coordinates[0] ?? NaN] : null));

const ringOf = (collection: FeatureCollection): readonly (readonly [number, number])[] => {
  const geometry = collection.features[0]?.geometry;
  return geometry?.type === 'Polygon' ? (geometry.coordinates[0] ?? []).map((it) => [it[1] ?? NaN, it[0] ?? NaN] as const) : [];
};

const lineOf = (collection: FeatureCollection): readonly (readonly [number, number])[] => {
  const geometry = collection.features[0]?.geometry;
  return geometry?.type === 'LineString' ? geometry.coordinates.map((it) => [it[1] ?? NaN, it[0] ?? NaN] as const) : [];
};

describe('MissionPathTest', () => {
  const item = (index: number, longitude: number) => MissionItem({ index, sequence: index + 1, latitude: 41.0, longitude, command: 'Waypoint', selected: false, altitude: NaN });
  const home = item(0, 44.0);
  const first = item(1, 44.1);
  const second = item(2, 44.2);

  const longitudes = (items: readonly MissionItem[], link: boolean): readonly number[] | null => {
    const geometry = missionPath(items, link)?.geometry;
    if (geometry === undefined || geometry.type !== 'MultiLineString') return null;
    expect(geometry.coordinates.length).toBe(1);
    return (geometry.coordinates[0] ?? []).map((it) => it[0] ?? NaN);
  };

  it('without a takeoff the line does not start at the planned home', () => {
    expect(longitudes([home, first, second], false)).toEqual([44.1, 44.2]);
  });

  it('with a takeoff the line starts at the planned home', () => {
    expect(longitudes([home, first, second], true)).toEqual([44.0, 44.1, 44.2]);
  });

  it('one point left after dropping home is no line at all', () => {
    expect(missionPath([home, first], false)).toBeNull();
  });

  it('an item with an exit is flown through rather than touched', () => {
    const survey = MissionItem({ index: 1, sequence: 2, latitude: 41.0, longitude: 44.1, command: 'Survey', selected: false, altitude: NaN, exit: at(41.0, 44.15) });
    expect(longitudes([home, survey, second], false)).toEqual([44.1, 44.15, 44.2]);
  });

  it('an exit equal to the entry is not a second point', () => {
    expect(longitudes([home, first, second], false)).toEqual([44.1, 44.2]);
  });

  it('the route does not detour through a standalone coordinate', () => {
    const roi = MissionItem({ index: 2, sequence: 3, latitude: 41.9, longitude: 44.9, command: 'ROI', selected: false, altitude: NaN, exit: null, routed: false });
    expect(longitudes([home, first, roi, second], false)).toEqual([44.1, 44.2]);
  });

  it('an item still being set up is not linked into the route', () => {
    const halfMade = MissionItem({ index: 2, sequence: 3, latitude: 41.5, longitude: 44.15, command: 'Survey', selected: false, altitude: NaN, exit: null, routed: false });
    expect(longitudes([home, first, halfMade, second], false)).toEqual([44.1, 44.2]);
  });

  it('the core decides whether the line starts at home, and this head no longer guesses', () => {
    expect(linksStartToHome(parsed('{"kind":"object","linksStartToHome":false,"items":[{},{"kind":"takeoff"},{}]}'))).toBe(false);
  });

  it('a rover links to home with no takeoff item anywhere', () => {
    expect(linksStartToHome(parsed('{"kind":"object","linksStartToHome":true,"items":[{},{},{}]}'))).toBe(true);
  });

  it('an empty plan says false rather than leaving the key out', () => {
    expect(linksStartToHome(parsed('{"kind":"object","linksStartToHome":false,"items":[]}'))).toBe(false);
    expect(linksStartToHome(parsed('{"kind":"object"}'))).toBe(false);
    expect(linksStartToHome(null)).toBe(false);
  });
});

describe('MarkerDensityTest', () => {
  it('an ordinary plan is not crowded and is drawn exactly as before', () => {
    expect(crowded(4)).toBe(false);
    expect(crowded(CROWDED_ITEMS)).toBe(false);
    expect(markerRadius(false, false)).toBe(MARKER_RADIUS);
    expect(waypointLabel(7, false)).toBe('7');
  });

  it('a mission downloaded off a vehicle is crowded and drops to dots', () => {
    expect(crowded(CROWDED_ITEMS + 1)).toBe(true);
    expect(crowded(212)).toBe(true);
    expect(markerRadius(true, false)).toBe(CROWDED_RADIUS);
  });

  it('numbering is dropped only when there are too many to read', () => {
    expect(waypointLabel(150, true)).toBe('');
    expect(waypointLabel(150, false)).toBe('150');
  });

  it('the selected item stays full size so it can still be found in a crowd', () => {
    expect(markerRadius(true, true)).toBe(MARKER_RADIUS);
  });

  it('a selected item keeps its heavy outline whatever the crowd', () => {
    expect(markerStroke(true, true)).toBe(markerStroke(false, true));
    expect(markerStroke(true, false)).toBeLessThan(markerStroke(false, false));
  });

  it('every waypoint carries its own size, so the layer needs no zoom rule', () => {
    const items = Array.from({ length: 81 }, (_, it) => MissionItem({ index: it, sequence: it, latitude: 41.0 + it * 0.001, longitude: 44.0, command: 'Waypoint', selected: false, kind: 'waypoint' }));
    const features = missionFeatures(items, 3);
    expect(features.features.length).toBe(81);
    expect(property(features, 0, WAYPOINT_RADIUS_PROPERTY)).toBe(CROWDED_RADIUS);
    expect(property(features, 3, WAYPOINT_RADIUS_PROPERTY)).toBe(MARKER_RADIUS);
    expect(property(features, 0, WAYPOINT_LABEL_PROPERTY)).toBe('');
  });
});

describe('WaypointColourTest', () => {
  it('the commands that change what a plan means each get their own colour', () => {
    expect(waypointColour('takeoff', 22)).toBe(TAKEOFF_COLOUR);
    expect(waypointColour('land', 21)).toBe(LAND_COLOUR);
    expect(waypointColour('command', MAV_CMD_NAV_RETURN_TO_LAUNCH)).toBe(RETURN_COLOUR);
    expect(waypointColour('command', MAV_CMD_NAV_LOITER_TIME)).toBe(LOITER_COLOUR);
  });

  it('the launch point is not drawn as somewhere the aircraft flies to', () => {
    expect(waypointColour('settings', 16)).toBe(START_COLOUR);
  });

  it('an ordinary waypoint keeps the plain colour', () => {
    expect(waypointColour('waypoint', 16)).toBe(WAYPOINT_COLOUR);
    expect(waypointColour('', 0)).toBe(WAYPOINT_COLOUR);
  });

  it('a VTOL takeoff is a takeoff, because the kind says so and the name need not', () => {
    expect(waypointColour('takeoff', 84)).toBe(TAKEOFF_COLOUR);
    expect(waypointColour('land', 85)).toBe(LAND_COLOUR);
  });

  it('a plan in another language is drawn the same, because no colour reads a name', () => {
    const german = parsed(
      '{"items":[{"kind":"takeoff","command":22,"name":"Starten","flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}},' +
        '{"kind":"command","command":20,"name":"Rückkehr zum Start","flownLeg":true,"coordinate":{"latitude":41.1,"longitude":44.1}}]}',
    );
    expect(missionItems(german).map((it) => waypointColour(it.kind, it.commandId))).toEqual([TAKEOFF_COLOUR, RETURN_COLOUR]);
  });
});

describe('WaypointSelectionTest', () => {
  const items = () =>
    missionItems(
      parsed('{"kind":"object","items":[{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}},{"flownLeg":true,"coordinate":{"latitude":41.1,"longitude":44.1}}]}'),
    );

  it('only the selected waypoint is marked', () => {
    const features = missionFeatures(items(), 1);
    expect(property(features, 0, WAYPOINT_SELECTED_PROPERTY)).toBe(false);
    expect(property(features, 1, WAYPOINT_SELECTED_PROPERTY)).toBe(true);
  });

  it('nothing is marked when nothing is selected', () => {
    expect(properties(missionFeatures(items(), null), WAYPOINT_SELECTED_PROPERTY).some((it) => it === true)).toBe(false);
  });

  it('every feature carries the flag so the layer never reads a missing property', () => {
    expect(missionFeatures(items(), 99).features.every((it) => it.properties !== null && WAYPOINT_SELECTED_PROPERTY in it.properties)).toBe(true);
  });
});

describe('FeaturePositionTest', () => {
  const item = (index: number, latitude: number, longitude: number) => MissionItem({ index, sequence: index, latitude, longitude, command: 'Waypoint', selected: false });

  it('a waypoint marker sits on the waypoint', () => {
    expect(pointsOf(missionFeatures([item(0, 41.0, 44.0), item(1, 41.2, 44.3)]))).toEqual([
      [41.0, 44.0],
      [41.2, 44.3],
    ]);
  });

  it('a rally marker sits on the rally point', () => {
    expect(pointsOf(rallyFeatures([RallyPoint({ index: 0, latitude: 41.5, longitude: 44.5 })]))).toEqual([[41.5, 44.5]]);
  });

  it('the vehicle marker sits on the vehicle', () => {
    expect(pointsOf(vehicleFeatures(41.7, 44.8, 90.0))).toEqual([[41.7, 44.8]]);
  });

  it('a fence ring runs through its vertices and closes', () => {
    const fence = FencePolygon({ index: 0, inclusion: true, vertices: [at(41.0, 44.0), at(41.0, 44.1), at(41.1, 44.1)] });
    expect(ringOf(fenceFeatures([fence]))).toEqual([
      [41.0, 44.0],
      [41.0, 44.1],
      [41.1, 44.1],
      [41.0, 44.0],
    ]);
  });

  it('a survey area runs through its corners and closes', () => {
    const survey = Survey({ index: 0, area: [at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1)], transects: [], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: 'surveyAreaPolygon' });
    expect(ringOf(surveyAreaFeatures([survey]))).toEqual([
      [42.0, 45.0],
      [42.0, 45.1],
      [42.1, 45.1],
      [42.0, 45.0],
    ]);
  });

  it('a corridor is drawn open, because its path is not a boundary', () => {
    const corridor = Survey({ index: 0, area: [at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1)], transects: [], cameraShots: 0, kind: 'corridor', shape: SHAPE_LINE, property: 'corridorPolyline' });
    expect(lineOf(surveyLineFeatures([corridor]))).toEqual([
      [42.0, 45.0],
      [42.0, 45.1],
      [42.1, 45.1],
    ]);
    expect(surveyAreaFeatures([corridor]).features.length).toBe(0);
  });

  it('a structure scan is drawn closed, like a survey', () => {
    const structure = Survey({ index: 0, area: [at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1)], transects: [], cameraShots: 0, kind: 'structure', shape: SHAPE_AREA, property: 'structurePolygon' });
    expect(ringOf(surveyAreaFeatures([structure]))).toEqual([
      [42.0, 45.0],
      [42.0, 45.1],
      [42.1, 45.1],
      [42.0, 45.0],
    ]);
    expect(surveyLineFeatures([structure]).features.length).toBe(0);
  });

  it('survey transects run through their points in order', () => {
    const survey = Survey({ index: 0, area: [], transects: [at(43.0, 46.0), at(43.1, 46.1)], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: 'surveyAreaPolygon' });
    expect(lineOf(surveyTransectFeatures([survey]))).toEqual([
      [43.0, 46.0],
      [43.1, 46.1],
    ]);
  });
});

describe('FenceFeatureTest', () => {
  const ring = (index: number, inclusion = true) => FencePolygon({ index, inclusion, vertices: [at(41.0, 44.0), at(41.0, 44.1), at(41.1, 44.1)] });

  it('a circle carries the index that finds it again', () => {
    const features = fenceFeatures([], [ring(4)]);
    expect(features.features.length).toBe(1);
    expect(property(features, 0, CIRCLE_INDEX_PROPERTY)).toBe(4);
  });

  it('a polygon carries no circle index so it cannot be mistaken for one', () => {
    const features = fenceFeatures([ring(0)], []);
    expect(features.features.length).toBe(1);
    expect(property(features, 0, CIRCLE_INDEX_PROPERTY)).toBeNull();
  });

  it('polygons and circles keep separate index spaces in one collection', () => {
    const features = fenceFeatures([ring(0)], [ring(0)]);
    expect(features.features.length).toBe(2);
    expect(properties(features, CIRCLE_INDEX_PROPERTY).filter((it) => it !== null).length).toBe(1);
  });

  it('a keep-out fence is drawn in a different colour from a keep-in one', () => {
    expect(properties(fenceFeatures([ring(0, true), ring(1, false)], []), KEEPS_IN_PROPERTY)).toEqual([true, false]);
    expect(KEEP_IN_COLOUR).not.toBe(KEEP_OUT_COLOUR);
  });

  it('a keep-out circle carries its boundary too, not just polygons', () => {
    expect(property(fenceFeatures([], [ring(2, false)]), 0, KEEPS_IN_PROPERTY)).toBe(false);
  });
});

describe('FenceHandleTest', () => {
  const polygon = (index: number, ...vertices: readonly (readonly [number, number])[]) => FencePolygon({ index, inclusion: true, vertices: vertices.map(([latitude, longitude]) => at(latitude, longitude)) });
  const survey = (index: number, ...vertices: readonly (readonly [number, number])[]) =>
    Survey({ index, area: vertices.map(([latitude, longitude]) => at(latitude, longitude)), transects: [], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: 'surveyAreaPolygon' });

  it('every vertex of every polygon gets a handle', () => {
    const features = vertexHandleFeatures(
      [
        polygon(0, [41.0, 44.0], [41.0, 44.1], [41.1, 44.1]),
        polygon(1, [42.0, 45.0], [42.0, 45.1], [42.1, 45.1], [42.1, 45.0]),
      ],
      [],
    );
    expect(features.features.length).toBe(7);
  });

  it('a handle sits on the vertex it edits, longitude not swapped for latitude', () => {
    expect(pointsOf(vertexHandleFeatures([polygon(0, [41.0, 44.0], [41.2, 44.3])], []))).toEqual([
      [41.0, 44.0],
      [41.2, 44.3],
    ]);
  });

  it('survey and circle handles are placed on their own points too', () => {
    const features = vertexHandleFeatures([polygon(0, [41.0, 44.0])], [survey(1, [42.0, 45.0])], [FenceCircle({ index: 2, inclusion: true, centre: at(43.0, 46.0), radius: 150.0 })]);
    expect(pointsOf(features).slice(0, 3)).toEqual([
      [41.0, 44.0],
      [42.0, 45.0],
      [43.0, 46.0],
    ]);
  });

  it('a handle carries the polygon and vertex it belongs to', () => {
    const features = vertexHandleFeatures([polygon(3, [41.0, 44.0], [41.0, 44.1], [41.1, 44.1])], []);
    expect(property(features, 1, HANDLE_KIND_PROPERTY)).toBe(HANDLE_KIND_FENCE);
    expect(property(features, 1, POLYGON_INDEX_PROPERTY)).toBe(3);
    expect(property(features, 1, VERTEX_INDEX_PROPERTY)).toBe(1);
  });

  it('a survey area gets handles marked as its own kind', () => {
    const features = vertexHandleFeatures([], [survey(5, [41.0, 44.0], [41.0, 44.1], [41.1, 44.1])]);
    expect(features.features.length).toBe(3);
    expect(property(features, 0, HANDLE_KIND_PROPERTY)).toBe(HANDLE_KIND_SURVEY);
    expect(property(features, 0, POLYGON_INDEX_PROPERTY)).toBe(5);
  });

  it('fence and survey handles share one collection without colliding', () => {
    const features = vertexHandleFeatures([polygon(0, [41.0, 44.0], [41.0, 44.1], [41.1, 44.1])], [survey(0, [42.0, 45.0], [42.0, 45.1], [42.1, 45.1])]);
    const kinds = properties(features, HANDLE_KIND_PROPERTY);
    expect(features.features.length).toBe(6);
    expect(kinds.filter((it) => it === HANDLE_KIND_FENCE).length).toBe(3);
    expect(kinds.filter((it) => it === HANDLE_KIND_SURVEY).length).toBe(3);
  });

  it('nothing to edit means no handles', () => {
    expect(vertexHandleFeatures([], []).features.length).toBe(0);
  });

  it('a circle gets a centre handle and an edge handle', () => {
    const features = vertexHandleFeatures([], [], [FenceCircle({ index: 2, inclusion: true, centre: at(41.0, 44.0), radius: 150.0 })]);
    expect(properties(features, HANDLE_KIND_PROPERTY)).toEqual([HANDLE_KIND_CIRCLE, HANDLE_KIND_FENCE_CIRCLE_RADIUS]);
    expect(property(features, 0, POLYGON_INDEX_PROPERTY)).toBe(2);
    expect(property(features, 0, VERTEX_INDEX_PROPERTY)).toBe(0);
  });

  it('all three kinds of handle coexist and stay distinguishable', () => {
    const features = vertexHandleFeatures(
      [polygon(0, [41.0, 44.0], [41.0, 44.1], [41.1, 44.1])],
      [survey(0, [42.0, 45.0], [42.0, 45.1], [42.1, 45.1])],
      [FenceCircle({ index: 0, inclusion: true, centre: at(43.0, 46.0), radius: 150.0 })],
    );
    const kinds = properties(features, HANDLE_KIND_PROPERTY);
    expect(features.features.length).toBe(8);
    expect(kinds.filter((it) => it === HANDLE_KIND_FENCE).length).toBe(3);
    expect(kinds.filter((it) => it === HANDLE_KIND_SURVEY).length).toBe(3);
    expect(kinds.filter((it) => it === HANDLE_KIND_CIRCLE).length).toBe(1);
    expect(kinds.filter((it) => it === HANDLE_KIND_FENCE_CIRCLE_RADIUS).length).toBe(1);
  });
});

describe('FenceCircleRadiusDragTest', () => {
  const centre = at(47.0, 8.0);

  it('the edge handle sits east of the centre at the radius, as QGCMapCircleVisuals places its drag handle', () => {
    const circle = FenceCircle({ index: 3, inclusion: true, centre, radius: 100.0 });
    const edge = circleEdge(circle);
    near(metresBetween(centre, edge), 100.0, 0.5);
    expect(edge.longitude > centre.longitude).toBe(true);
    const handle = vertexHandleFeatures([], [], [circle]).features.find((it) => it.properties?.[HANDLE_KIND_PROPERTY] === HANDLE_KIND_FENCE_CIRCLE_RADIUS);
    const kind = handle?.properties?.[HANDLE_KIND_PROPERTY];
    const owner = handle?.properties?.[POLYGON_INDEX_PROPERTY];
    const vertex = handle?.properties?.[VERTEX_INDEX_PROPERTY];
    expect(handleHit(typeof kind === 'string' ? kind : null, typeof owner === 'number' ? owner : NaN, typeof vertex === 'number' ? vertex : NaN)).toEqual({ kind: 'CircleRadius', index: 3 });
  });

  it('dragging sets the distance from the centre, in the units the radius is shown in', () => {
    const feet = FenceCircle({ index: 0, inclusion: true, centre, radius: 328.084, radiusMetres: 100.0, radiusUnits: 'ft' });
    near(draggedCircleRadius(feet, pointAt(centre, 200.0, 90.0)), 656.168, 0.5);
  });

  it('a drag past the fence limits stops at them', () => {
    const bounded = FenceCircle({ index: 0, inclusion: true, centre, radius: 100.0, radiusMinimum: 10.0, radiusMaximum: 500.0 });
    near(draggedCircleRadius(bounded, pointAt(centre, 1.0, 90.0)), 10.0, 1e-9);
    near(draggedCircleRadius(bounded, pointAt(centre, 900.0, 90.0)), 500.0, 1e-9);
    const unbounded = FenceCircle({ index: 0, inclusion: true, centre, radius: 100.0 });
    near(draggedCircleRadius(unbounded, centre), 0.1, 1e-9);
  });
});

describe('LoiterHandleTest', () => {
  const loiter = MissionItem({ index: 2, sequence: 2, latitude: 47.0, longitude: 8.0, command: '', selected: false, loiterRadius: 80.0 });
  const waypoint = MissionItem({ index: 1, sequence: 1, latitude: 47.1, longitude: 8.0, command: '', selected: false });

  it('the arrows run with the turn, the way QGCMapCircleVisuals points them', () => {
    expect(loiterRotationArrows([waypoint, loiter]).map((it) => it.bearing)).toEqual([90.0, 270.0]);
    expect(loiterRotationArrows([{ ...loiter, loiterRadius: -80.0 }]).map((it) => it.bearing)).toEqual([270.0, 90.0]);
  });

  it('only the current loiter can be dragged or flipped, every loiter shows its arrows', () => {
    const kinds = (selected: number | null) => loiterHandleFeatures([waypoint, loiter], selected).map((it) => it.properties?.[HANDLE_KIND_PROPERTY]);
    expect(kinds(null)).toEqual([]);
    expect(loiterRotationArrows([waypoint, loiter]).length).toBe(2);
    expect(kinds(2)).toEqual([HANDLE_KIND_LOITER_ROTATION, HANDLE_KIND_LOITER_ROTATION, HANDLE_KIND_LOITER_RADIUS]);
    expect(handleHit(HANDLE_KIND_LOITER_RADIUS, 2, 0)).toEqual({ kind: 'LoiterRadius', index: 2 });
    expect(handleHit(HANDLE_KIND_LOITER_ROTATION, 2, 1)).toEqual({ kind: 'LoiterRotation', index: 2 });
  });

  it('dragging the radius keeps the direction it turns', () => {
    const to = pointAt(at(47.0, 8.0), 150.0, 90.0);
    near(draggedLoiterRadius(loiter, to), 150.0, 0.5);
    near(draggedLoiterRadius({ ...loiter, loiterRadius: -80.0 }, to), -150.0, 0.5);
  });
});

describe('ClickMarkerTest', () => {
  it('the marker sits on the tapped point and nowhere once the menu closes', () => {
    expect(clickMarkerFeatures(at(47.0, 8.0)).features.length).toBe(1);
    expect(clickMarkerFeatures(null).features.length).toBe(0);
    expect(clickMarkerFeatures(at(NaN, 8.0)).features.length).toBe(0);
  });
});

describe('CollisionLegTest', () => {
  it('legs the core says hit terrain are drawn red, and nothing else', () => {
    const view = parsed(
      '{"collisionLegs":[{"from":{"latitude":47.0,"longitude":8.0},"to":{"latitude":47.01,"longitude":8.0}},{"from":{"latitude":0,"longitude":0},"to":{"latitude":47.0,"longitude":8.0}}]}',
    );
    expect(collisionLegs(view)).toEqual([[at(47.0, 8.0), at(47.01, 8.0)]]);
    expect(collisionLegFeatures(collisionLegs(view)).features.length).toBe(1);
    expect(collisionLegs(null)).toEqual([]);
  });
});

describe('MarkerLabelTest', () => {
  it('a lettered item shows its first letter and its name beside it, like MissionItemIndicator', () => {
    expect(waypointLabel(1, false, 'Takeoff')).toBe('T');
    expect(sideLabel(false, 'Takeoff')).toBe('Takeoff');
    expect(waypointLabel(4, false, '')).toBe('4');
    expect(sideLabel(false, '')).toBe('');
    expect(waypointLabel(5, false, 'S')).toBe('S');
    expect(sideLabel(false, 'S')).toBe('');
    expect(sideLabel(true, 'Loiter')).toBe('');
  });

  it('a waypoint followed by a gripper release is marked as the drop point', () => {
    const waypoint = (index: number) => MissionItem({ index, sequence: index, latitude: 47 + index, longitude: 8, command: 'Waypoint', selected: false, commandId: 16, specifiesCoordinate: true });
    const drop = (index: number) => MissionItem({ index, sequence: index, latitude: NaN, longitude: NaN, command: 'Gripper Action', selected: false, commandId: 211, dropsPayload: true, placed: false, specifiesCoordinate: false });
    const items = [waypoint(1), drop(2), waypoint(3), MissionItem({ index: 4, sequence: 4, latitude: NaN, longitude: NaN, command: 'Change Speed', selected: false, commandId: 178, placed: false }), waypoint(5)];
    expect(items.map((it) => dropsAt(items, it))).toEqual([true, false, false, false, false]);
    const sides = missionFeatures(withDrops(items).filter((it) => it.placed)).features.map((it) => [it.properties?.[WAYPOINT_ID_PROPERTY], it.properties?.[WAYPOINT_SIDE_LABEL_PROPERTY]]);
    expect(sides.find(([id]) => id === 1)?.[1]).toBe(DROP_SIDE);
    expect(sides.find(([id]) => id === 3)?.[1]).toBe('');
  });

  it('a bay servo drop marks its waypoint, and a servo that drops nothing does not', () => {
    const waypoint = (index: number) => MissionItem({ index, sequence: index, latitude: 47 + index, longitude: 8, command: 'Waypoint', selected: false, commandId: 16, specifiesCoordinate: true });
    const servo = (index: number, dropsPayload: boolean) => MissionItem({ index, sequence: index, latitude: NaN, longitude: NaN, command: dropsPayload ? 'Drop bay 1' : 'Set Servo', selected: false, commandId: 183, dropsPayload, placed: false });
    const items = [waypoint(1), servo(2, true), waypoint(3), servo(4, false), waypoint(5)];
    expect(items.map((it) => dropsAt(items, it))).toEqual([true, false, false, false, false]);
  });
});

describe('MidpointTest', () => {
  const ring = EditableShape({
    path: 'plan.geoFenceController.polygons.0',
    midpoints: [at(41.7, 44.825), at(41.705, 44.83), at(41.71, 44.825), at(41.705, 44.82)],
    splitInvokable: 'splitPolygonSegment',
    canRemoveVertex: true,
  });
  const square = FencePolygon({ index: 0, inclusion: true, vertices: [at(41.7, 44.82), at(41.7, 44.83), at(41.71, 44.83), at(41.71, 44.82)], editable: ring });
  const corridor = EditableShape({ path: 'plan.missionController.visualItems.2.corridorPolyline', midpoints: [at(41.7, 44.825), at(41.705, 44.83)], splitInvokable: 'splitSegment', canRemoveVertex: true });
  const midpoint: MapHit = { kind: 'Midpoint', path: ring.path, invokable: ring.splitInvokable, segment: 2 };

  it('every segment offers somewhere to add a corner', () => {
    const features = midpointFeatures([ring]);
    expect(features.features.length).toBe(4);
    expect(properties(features, VERTEX_INDEX_PROPERTY)).toEqual([0, 1, 2, 3]);
  });

  it('a survey edge is split by its own invokable, not the fence one', () => {
    const features = midpointFeatures([ring, corridor]);
    expect(features.features.length).toBe(6);
    expect(properties(features, SPLIT_INVOKABLE_PROPERTY)).toEqual(['splitPolygonSegment', 'splitPolygonSegment', 'splitPolygonSegment', 'splitPolygonSegment', 'splitSegment', 'splitSegment']);
    expect([...new Set(properties(features, SHAPE_PATH_PROPERTY))]).toEqual([ring.path, corridor.path]);
  });

  it('a shape the core gave no midpoints for offers none', () => {
    expect(midpointFeatures([{ ...ring, midpoints: [] }]).features.length).toBe(0);
    expect(midpointFeatures([null]).features.length).toBe(0);
    expect(midpointFeatures([]).features.length).toBe(0);
  });

  it('a shape the core named no invokable for is not offered', () => {
    expect(midpointFeatures([{ ...ring, splitInvokable: '' }]).features.length).toBe(0);
  });

  it('a midpoint is a button, so it never writes a position', async () => {
    expect(await writeMove(midpoint, 41.0, 44.0, [], [])).toBe(false);
  });

  it('a midpoint is never a lasting selection', () => {
    expect(selectionSurvives(midpoint, [], [square], [], [], [])).toBe(false);
  });

  it('the leg into the selected item splits at its middle', () => {
    const item = (index: number, latitude: number, longitude: number) => MissionItem({ index, sequence: index, latitude, longitude, command: 'Waypoint', selected: false, altitude: 50.0 });
    const items = [item(1, 47.0, 8.0), item(2, 47.2, 8.4)];
    const split = legSplit(items, 2);
    if (split === null) throw new Error('split');
    near(metresBetween(at(47.0, 8.0), split), metresBetween(split, at(47.2, 8.4)), 0.5);
    near(split.latitude, 47.1, 0.01);
    near(azimuthBetween(at(0.0, 0.0), at(0.0, 1.0)), 90.0, 1e-9);
    expect(legSplit(items, 1)).toBeNull();
    expect(legSplit(items, null)).toBeNull();
  });

  it("the split skips home and standalone items and starts at a pattern's exit, as MissionController does", () => {
    const item = (index: number, latitude: number, routed: boolean, exit: TrackPoint | null) =>
      MissionItem({ index, sequence: index, latitude, longitude: 8.0, command: 'Waypoint', selected: false, altitude: 50.0, exit, routed });
    const home = item(0, 46.0, true, null);
    expect(legSplit([home, item(1, 47.0, true, null)], 1)).toBeNull();
    const roi = item(2, 47.5, false, null);
    expect(legSplit([home, item(1, 47.0, true, null), roi], 2)).toBeNull();
    const after = legSplit([home, item(1, 47.0, true, null), roi, item(3, 48.0, true, null)], 3);
    near(after?.latitude, 47.5, 0.01);
    const survey = item(1, 47.0, true, at(47.4, 8.0));
    near(legSplit([home, survey, item(2, 48.0, true, null)], 2)?.latitude, 47.7, 0.01);
  });
});

describe('CorridorAreaTest', () => {
  it('a corridor shades its width outline like TransectStyleMapVisuals', () => {
    const point = (latitude: number) => at(latitude, 8.0);
    const corridor = Survey({ index: 1, area: [point(47.0), point(47.1)], transects: [], cameraShots: 0, kind: 'corridor', shape: SHAPE_LINE, property: 'corridorPolyline', outline: [point(47.0), point(47.05), point(47.1)] });
    const survey = Survey({ index: 2, area: [point(46.0), point(46.1), point(46.2)], transects: [], cameraShots: 0, kind: 'survey', shape: SHAPE_AREA, property: 'surveyAreaPolygon' });
    expect(shadedArea(corridor)).toEqual(corridor.outline);
    expect(shadedArea(survey)).toEqual(survey.area);
    expect(surveyAreaFeatures([corridor, survey]).features.length).toBe(2);
    const tinted = surveyAreaFeatures([{ ...survey, collides: true }]);
    expect(tinted.features.length).toBe(1);
    expect(property(tinted, 0, SURVEY_COLLISION)).toBe(true);
    expect(collidingItems(parsed('{"collidingItems":[2]}'))).toEqual(new Set([2]));
  });
});

describe('StructureScanRouteTest', () => {
  const survey = ({ transects = [], loop = [] }: { transects?: readonly TrackPoint[]; loop?: readonly TrackPoint[] } = {}) =>
    Survey({ index: 0, area: [], transects, cameraShots: 0, kind: 'structure', shape: 'shape', property: 'property', editable: null, flightLoop: loop, layers: 3 });
  const square = [at(41.0, 44.0), at(41.0, 44.1), at(41.1, 44.1)];

  it('a structure scan flies a closed loop, so the route returns to where it started', () => {
    const route = flownRoute(survey({ loop: square }));
    expect(route.length).toBe(square.length + 1);
    expect(route[0]).toEqual(route[route.length - 1]);
  });

  it('a loop already closed is not closed twice', () => {
    const closed = [...square, ...square.slice(0, 1)];
    expect(flownRoute(survey({ loop: closed }))).toEqual(closed);
  });

  it('a survey mows, so its transects are the route and are left open', () => {
    expect(flownRoute(survey({ transects: square }))).toEqual(square);
  });

  it('an item with neither draws no route rather than a degenerate one', () => {
    expect(flownRoute(survey())).toEqual([]);
    expect(flownRoute(survey({ loop: square.slice(0, 1) }))).toEqual([]);
  });
});

describe('StructureScanLayersTest', () => {
  const survey = (layers: number, span = '') =>
    Survey({ index: 0, area: [], transects: [], cameraShots: 0, kind: 'structure', shape: 'shape', property: 'property', editable: null, flightLoop: [], layers, layerSpanText: span });

  it('stacked circuits are said in words, because on a map they sit exactly on each other', () => {
    expect(layersText(survey(3))).toBe('3 layers, one drawn');
  });

  it("the span is the core's sentence, so a feet rig does not read metres", () => {
    expect(layersText(survey(3, '12.0 m to 42.0 m'))).toBe('3 layers, 12.0 m to 42.0 m, one drawn');
    expect(layersText(survey(3, '39 ft to 138 ft'))).toBe('3 layers, 39 ft to 138 ft, one drawn');
  });

  it('a span the core withheld leaves the count alone rather than inventing one', () => {
    expect(layersText(survey(3))).toBe('3 layers, one drawn');
  });

  it('a single layer says nothing, because the loop drawn is the whole mission', () => {
    expect(layersText(survey(1))).toBeNull();
    expect(layersText(survey(0))).toBeNull();
    expect(layersText(null)).toBeNull();
  });
});

describe('PatternNameTest', () => {
  const item = (index: number, name: string) => MissionItem({ index, sequence: index, latitude: 41.0, longitude: 44.0, command: name, selected: false });

  it('a structure scan is not called a survey, because the core already named it', () => {
    const items = [item(2, 'Structure Scan'), item(3, 'Corridor Scan')];
    expect(patternName(2, items)).toBe('Structure Scan');
    expect(patternName(3, items)).toBe('Corridor Scan');
  });

  it('an item the list does not hold falls back to a word that is true of all three', () => {
    expect(patternName(9, [])).toBe('pattern');
    expect(patternName(2, [item(2, '')])).toBe('pattern');
  });
});

describe('CircleModeTest', () => {
  const centre = at(47.0, 8.0);
  const ring = circleRing(centre, 100.0, 36);
  const fence = FencePolygon({ index: 0, inclusion: true, vertices: ring });

  it("a circle's radius is the distance from its centre to a vertex", () => {
    near(circleRadius(ring), 100.0, 0.5);
    expect(circleRadius([])).toBeNull();
  });

  it('setting the radius redraws the ring around the same centre', () => {
    const wider = circleAround(ring, 250.0);
    if (wider === null) throw new Error('no circle');
    near(circleRadius(wider), 250.0, 0.5);
    near(polygonCentre(wider)?.latitude, centre.latitude, 1e-6);
    expect(circleAround(ring, 0.0)).toBeNull();
  });

  it('only a circled shape gets a radius handle, and loses its corners', () => {
    expect(radiusHandleFeatures([fence], [], new Set()).length).toBe(0);
    expect(radiusHandleFeatures([fence], [], new Set([fencePath(0)])).length).toBe(1);
  });

  it('a target finds its own vertices', () => {
    expect(shapeVertices(ShapeTarget({ path: fencePath(0), line: false }), [fence], [])).toEqual(ring);
    expect(shapeVertices(ShapeTarget({ path: fencePath(3), line: false }), [fence], [])).toEqual([]);
  });

  it("a circle's centre is edited as its shape centre, like Edit Center Position", () => {
    const found = shapeCentreHit(ShapeTarget({ path: fencePath(0), line: false }), [fence], []);
    if (found === null) throw new Error('no centre');
    const [hit, middle] = found;
    expect(hit).toEqual({ kind: 'ShapeCentre', fence: true, owner: 0 });
    near(middle.latitude, centre.latitude, 1e-6);
    near(middle.longitude, centre.longitude, 1e-6);
    expect(positionTitle(hit)).toBe('Edit center position');
    expect(positionTitle({ kind: 'FenceVertex', polygon: 0, vertex: 1 })).toBe('Edit vertex position');
    expect(shapeCentreHit(ShapeTarget({ path: fencePath(3), line: false }), [fence], [])).toBeNull();
  });
});

describe('DragStepTest', () => {
  it('a drag step dispatched before the drop never lands after it', async () => {
    const midpoint: MapHit = { kind: 'Midpoint', path: '', invokable: '', segment: 0 };
    const before = moveGeneration();
    await writeMove(midpoint, 0.0, 0.0, [], []);
    expect(await writeDragStep(before, midpoint, 0.0, 0.0, [], [], [], [])).toBe(true);
    expect(await writeDragStep(moveGeneration(), midpoint, 0.0, 0.0, [], [], [], [])).toBe(false);
  });
});

describe('LiveCircleTest', () => {
  it('circle mode follows the shape at that path, not just its index', () => {
    const ring = circleAround(circleRing(at(47.0, 8.0), 100.0, 16), 100.0);
    if (ring === null) throw new Error('no circle');
    const square = [at(47.0, 8.0), at(47.001, 8.0), at(47.001, 8.001), at(47.0, 8.001)];
    const chosen = new Set([fencePath(0)]);
    expect(liveCircles(chosen, [FencePolygon({ index: 0, inclusion: true, vertices: ring })], [])).toEqual(chosen);
    expect(liveCircles(chosen, [FencePolygon({ index: 0, inclusion: true, vertices: square })], [])).toEqual(new Set());
    expect(liveCircles(chosen, [], [])).toEqual(new Set());
  });
});

describe('ShotPointsTest', () => {
  const video = (points: string): Json => parsed(`{"kind":"object","class":"Video","available":true,"decoding":true,"shotPoints":${points}}`);

  it('every photo the vehicle reported gets a place on the map', () => {
    const points = shotPoints(video('[{"latitude":47.397,"longitude":8.545},{"latitude":47.398,"longitude":8.546}]'));
    expect(points.length).toBe(2);
    near(points[0]?.latitude, 47.397, 1e-9);
    near(points[1]?.longitude, 8.546, 1e-9);
  });

  it('a vehicle that has taken no photos draws nothing', () => {
    expect(shotPoints(video('[]'))).toEqual([]);
    expect(shotPoints(parsed('{"kind":"object"}'))).toEqual([]);
    expect(shotPoints(null)).toEqual([]);
  });

  it('a point without a usable coordinate is dropped rather than plotted at zero', () => {
    const points = shotPoints(video('[{"latitude":47.397,"longitude":8.545},{"longitude":8.546},{"latitude":null,"longitude":null}]'));
    expect(points.length).toBe(1);
    expect((points[0]?.latitude ?? 0) > 47.0).toBe(true);
  });

  it('the features handed to the map carry longitude first, as GeoJSON wants it', () => {
    const collection = shotFeatures([at(47.397, 8.545)]);
    const json = JSON.stringify(collection);
    expect(json).toContain('8.545');
    expect(collection.features.length).toBe(1);
    expect(json.indexOf('8.545') < json.indexOf('47.397')).toBe(true);
  });
});
