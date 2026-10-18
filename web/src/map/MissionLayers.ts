import type { Feature, FeatureCollection, GeoJsonProperties } from 'geojson';
import type { ExpressionSpecification, Map as MapLibreMap, SymbolLayerSpecification } from 'maplibre-gl';
import { coerceAtLeast, coerceAtMost } from '@/kotlin/Std';
import type { EditableShape } from './EditableShape';
import { type FenceCircle, type FencePolygon, type FirmwareFence, type RallyPoint, azimuthBetween, circleRing, pointAt } from './FenceBridge';
import type { LandingPattern } from './LandingPattern';
import { LANDING_AREA_KIND } from './LandingPattern';
import { HOME_ITEM } from './MissionKinds';
import { MAV_CMD_NAV_RETURN_TO_LAUNCH, type MissionItem } from './PlanBridge';
import { CORRIDOR_PROPERTY, circleRadius, fencePath, polygonCentre, surveyPath } from './PolygonTools';
import { SHAPE_AREA, SHAPE_LINE, type Survey } from './SurveyBridge';
import { metresBetween } from './TerrainProfile';
import { type TransectArrow, arrowOn } from './TransectMarks';
import { type TrackPoint, featureCollection, geoJsonSource, isPlottable, lineFeature, pointFeature, setGeoJson } from './VehicleMap';

export const CROWDED_ITEMS = 40;
export const CROWDED_RADIUS = 4.0;
export const MARKER_RADIUS = 13.0;
const CROWDED_STROKE = 0.5;
const MARKER_STROKE = 2.0;
const SELECTED_STROKE = 5.0;
export const WAYPOINT_RADIUS_PROPERTY = 'waypointRadius';
export const WAYPOINT_STROKE_PROPERTY = 'waypointStroke';

export const MISSION_SOURCE = 'aircast-mission';
export const MISSION_LAYER = 'aircast-mission-layer';
export const MISSION_DOT_LAYER = 'aircast-mission-dot-layer';
export const MISSION_PATH_SOURCE = 'aircast-mission-path';
export const LEG_LABEL_SOURCE = 'aircast-leg-labels';
export const LEG_LABEL_LAYER = 'aircast-leg-labels-layer';
export const LEG_LABEL_PROPERTY = 'leg';
export const MISSION_PATH_LAYER = 'aircast-mission-path-layer';
const COLLISION_LEG_SOURCE = 'aircast-collision-legs';
const COLLISION_LEG_LAYER = 'aircast-collision-leg-layer';
const COLLISION_COLOUR = '#FF0000';

const NOTO: string[] = ['Noto Sans Regular'];

const flagged = (property: string): ExpressionSpecification => ['boolean', ['get', property], false];

const zipWithNext = <T,>(list: readonly T[]): readonly (readonly [T, T])[] =>
  list.flatMap((from, at) => {
    const to = list[at + 1];
    return to === undefined ? [] : [[from, to] as const];
  });

const pointOf = (item: { readonly latitude: number; readonly longitude: number }): TrackPoint => ({ latitude: item.latitude, longitude: item.longitude });

const samePoint = (a: TrackPoint | undefined, b: TrackPoint | undefined): boolean => a !== undefined && b !== undefined && a.latitude === b.latitude && a.longitude === b.longitude;

const closedRing = (vertices: readonly TrackPoint[]): readonly TrackPoint[] =>
  vertices.length === 0 || samePoint(vertices[0], vertices[vertices.length - 1]) ? vertices : [...vertices, ...vertices.slice(0, 1)];

const ringFeature = (vertices: readonly TrackPoint[], properties: GeoJsonProperties = {}): Feature => ({
  type: 'Feature',
  geometry: { type: 'Polygon', coordinates: [closedRing(vertices).map((it) => [it.longitude, it.latitude])] },
  properties,
});

export const collisionLegFeatures = (legs: readonly (readonly [TrackPoint, TrackPoint])[]): FeatureCollection => featureCollection(legs.map(([from, to]) => lineFeature([from, to])));

export const renderCollisionLegs = (style: MapLibreMap, legs: readonly (readonly [TrackPoint, TrackPoint])[]): void => setGeoJson(style, COLLISION_LEG_SOURCE, collisionLegFeatures(legs));

export const WAYPOINT_ID_PROPERTY = 'waypointId';
export const WAYPOINT_EXIT_PROPERTY = 'waypointExit';
export const WAYPOINT_LABEL_PROPERTY = 'label';
export const WAYPOINT_SIDE_LABEL_PROPERTY = 'sideLabel';
const MISSION_SIDE_LABEL_LAYER = 'aircast-mission-side-label-layer';
export const WAYPOINT_COLOUR_PROPERTY = 'waypointColour';
export const WAYPOINT_SELECTED_PROPERTY = 'waypointSelected';

export const TAKEOFF_COLOUR = '#43A047';
export const LAND_COLOUR = '#E53935';
export const RETURN_COLOUR = '#5C6BC0';
export const LOITER_COLOUR = '#26A69A';
export const START_COLOUR = '#7E57C2';
export const WAYPOINT_COLOUR = '#FFB300';

export const MAV_CMD_NAV_LOITER_UNLIM = 17;
export const MAV_CMD_NAV_LOITER_TURNS = 18;
export const MAV_CMD_NAV_LOITER_TIME = 19;
export const MAV_CMD_DO_ORBIT = 34;

const LOITER_COMMANDS: ReadonlySet<number> = new Set([MAV_CMD_NAV_LOITER_UNLIM, MAV_CMD_NAV_LOITER_TURNS, MAV_CMD_NAV_LOITER_TIME, MAV_CMD_DO_ORBIT]);

export const waypointColour = (kind: string, commandId: number): string =>
  kind === 'settings'
    ? START_COLOUR
    : kind === 'takeoff'
      ? TAKEOFF_COLOUR
      : kind === 'land'
        ? LAND_COLOUR
        : commandId === MAV_CMD_NAV_RETURN_TO_LAUNCH
          ? RETURN_COLOUR
          : LOITER_COMMANDS.has(commandId)
            ? LOITER_COLOUR
            : WAYPOINT_COLOUR;

export const installMissionLayers = (style: MapLibreMap): void => {
  if (style.getSource(MISSION_PATH_SOURCE) === undefined) {
    style.addSource(MISSION_PATH_SOURCE, geoJsonSource());
    style.addLayer({ id: MISSION_PATH_LAYER, type: 'line', source: MISSION_PATH_SOURCE, paint: { 'line-color': '#FFB300', 'line-width': 3, 'line-dasharray': [2, 1.5] } });
  }
  if (style.getSource(LEG_LABEL_SOURCE) === undefined) {
    style.addSource(LEG_LABEL_SOURCE, geoJsonSource());
    style.addLayer({
      id: LEG_LABEL_LAYER,
      type: 'symbol',
      source: LEG_LABEL_SOURCE,
      layout: { 'text-field': ['get', LEG_LABEL_PROPERTY], 'text-font': NOTO, 'text-size': 11, 'text-offset': [0, 1.2], 'text-allow-overlap': false },
      paint: { 'text-color': '#FFFFFF', 'text-halo-color': '#37474F', 'text-halo-width': 2 },
    });
  }
  if (style.getSource(COLLISION_LEG_SOURCE) === undefined) {
    style.addSource(COLLISION_LEG_SOURCE, geoJsonSource());
    style.addLayer({ id: COLLISION_LEG_LAYER, type: 'line', source: COLLISION_LEG_SOURCE, paint: { 'line-color': COLLISION_COLOUR, 'line-width': 3 } });
  }
  if (style.getSource(MISSION_SOURCE) === undefined) {
    style.addSource(MISSION_SOURCE, geoJsonSource());
    style.addLayer({
      id: MISSION_DOT_LAYER,
      type: 'circle',
      source: MISSION_SOURCE,
      paint: {
        'circle-color': ['get', WAYPOINT_COLOUR_PROPERTY],
        'circle-radius': ['get', WAYPOINT_RADIUS_PROPERTY],
        'circle-stroke-color': ['case', flagged(WAYPOINT_SELECTED_PROPERTY), '#FFFFFF', '#37474F'],
        'circle-stroke-width': ['get', WAYPOINT_STROKE_PROPERTY],
      },
    });
    style.addLayer({
      id: MISSION_LAYER,
      type: 'symbol',
      source: MISSION_SOURCE,
      layout: { 'text-field': ['get', WAYPOINT_LABEL_PROPERTY], 'text-font': NOTO, 'text-size': 15, 'text-allow-overlap': false, 'text-ignore-placement': false },
      paint: { 'text-color': '#FFFFFF', 'text-halo-color': '#37474F', 'text-halo-width': 2.5 },
    });
    style.addLayer({
      id: MISSION_SIDE_LABEL_LAYER,
      type: 'symbol',
      source: MISSION_SOURCE,
      layout: { 'text-field': ['get', WAYPOINT_SIDE_LABEL_PROPERTY], 'text-font': NOTO, 'text-size': 13, 'text-anchor': 'left', 'text-offset': [1.2, 0], 'text-allow-overlap': true },
      paint: { 'text-color': '#FFFFFF', 'text-halo-color': '#37474F', 'text-halo-width': 2 },
    });
  }
};

export const crowded = (itemCount: number): boolean => itemCount > CROWDED_ITEMS;

export const legLabels = (items: readonly MissionItem[]): readonly (readonly [TrackPoint, string])[] =>
  crowded(items.length)
    ? []
    : zipWithNext(items.filter((it) => it.placed && (it.routed || it.index === HOME_ITEM)).toSorted((a, b) => a.index - b.index))
        .filter(([, to]) => !to.legBroken && to.distanceText.trim() !== '' && to.distance > 0.0)
        .map(([from, to]) => {
          const start = from.exit ?? pointOf(from);
          return [{ latitude: (start.latitude + to.latitude) / 2, longitude: (start.longitude + to.longitude) / 2 }, to.distanceText] as const;
        });

export const waypointLabel = (sequence: number, crowded: boolean, abbreviation: string = ''): string =>
  crowded ? '' : lettered(abbreviation) ? abbreviation.slice(0, 1) : sequence.toString();

export const sideLabel = (crowded: boolean, abbreviation: string): string => (!crowded && lettered(abbreviation) && abbreviation.length > 1 ? abbreviation : '');

const lettered = (abbreviation: string): boolean => {
  const first = abbreviation.charAt(0);
  return first !== '' && first > 'A' && first < 'z';
};

export const markerRadius = (crowded: boolean, selected: boolean): number => (crowded && !selected ? CROWDED_RADIUS : MARKER_RADIUS);

export const markerStroke = (crowded: boolean, selected: boolean): number => (selected ? SELECTED_STROKE : crowded ? CROWDED_STROKE : MARKER_STROKE);

type Marker = { item: MissionItem; at: TrackPoint; sequence: number; exit: boolean; side: string | null };

export const DROP_COMMAND = 211;
export const DROP_SIDE = 'Drop';

export const dropsAt = (items: readonly MissionItem[], item: MissionItem): boolean => {
  const start = items.findIndex((it) => it.index === item.index);
  if (start < 0) return false;
  const following = items.slice(start + 1);
  const next = following.findIndex((it) => it.specifiesCoordinate || it.complexPattern);
  return following.slice(0, next < 0 ? following.length : next).some((it) => it.commandId === DROP_COMMAND);
};

export const withDrops = (items: readonly MissionItem[]): readonly MissionItem[] => items.map((it) => ({ ...it, drops: dropsAt(items, it) }));

export const exitMarkers = (items: readonly MissionItem[]): readonly (readonly [MissionItem, TrackPoint])[] =>
  items.filter((it) => it.complexPattern).flatMap((item) => (item.exit === null ? [] : [[item, item.exit] as const]));

const landingMarkers = (item: MissionItem, pattern: LandingPattern): readonly Marker[] => [
  ...(pattern.finalApproach === null ? [] : [{ item, at: pattern.finalApproach, sequence: item.sequence, exit: false, side: pattern.loiterToAltitude ? 'Loiter' : 'Approach' }]),
  ...(pattern.landing === null ? [] : [{ item, at: pattern.landing, sequence: item.sequence + item.foldedCommands, exit: true, side: 'Land' }]),
];

export const HOME_SEAL = 'H';
const SAME_SPOT_DEGREES = 1e-6;

export const homeCovered = (items: readonly MissionItem[]): boolean => {
  const home = items.find((it) => it.index === HOME_ITEM && it.placed);
  if (home === undefined) return false;
  return items.some((it) => it.index !== HOME_ITEM && it.placed && Math.abs(it.latitude - home.latitude) < SAME_SPOT_DEGREES && Math.abs(it.longitude - home.longitude) < SAME_SPOT_DEGREES);
};

export const missionFeatures = (items: readonly MissionItem[], selectedIndex: number | null = null, landings: readonly LandingPattern[] = []): FeatureCollection => {
  const isCrowded = crowded(items.length);
  const patterns: ReadonlyMap<number, LandingPattern> = new Map(landings.map((it) => [it.index, it]));
  const covered = homeCovered(items);
  const plain = items.filter((it) => !patterns.has(it.index)).filter((it) => !(it.index === HOME_ITEM && covered));
  const markers: readonly Marker[] = [
    ...plain.map((item) => ({ item, at: pointOf(item), sequence: item.sequence, exit: false, side: item.drops ? DROP_SIDE : null })),
    ...exitMarkers(plain).map(([item, exit]) => ({ item, at: exit, sequence: item.sequence + item.foldedCommands, exit: true, side: null })),
    ...items.flatMap((item) => {
      const pattern = patterns.get(item.index);
      return pattern === undefined ? [] : landingMarkers(item, pattern);
    }),
  ];
  return featureCollection(
    markers.map(({ item, at, sequence, exit, side }) => {
      const letters = !exit && !item.complexPattern ? item.abbreviation : '';
      const selected = item.index === selectedIndex;
      return pointFeature(at.longitude, at.latitude, {
        [WAYPOINT_ID_PROPERTY]: item.index,
        [WAYPOINT_LABEL_PROPERTY]: item.index === HOME_ITEM && !exit && !isCrowded ? HOME_SEAL : waypointLabel(sequence, isCrowded, letters),
        [WAYPOINT_SIDE_LABEL_PROPERTY]: side !== null && !isCrowded ? side : sideLabel(isCrowded, letters),
        [WAYPOINT_RADIUS_PROPERTY]: markerRadius(isCrowded, selected),
        [WAYPOINT_STROKE_PROPERTY]: markerStroke(isCrowded, selected),
        [WAYPOINT_COLOUR_PROPERTY]: waypointColour(item.kind, item.commandId),
        [WAYPOINT_SELECTED_PROPERTY]: selected,
        [WAYPOINT_EXIT_PROPERTY]: exit,
      });
    }),
  );
};

const LEG_ARROW_QUARTER = 3;
const LEG_ARROW_SPACING = 5;

type ArrowWalk = { arrows: readonly boolean[]; count: number };

const missionRoute = (items: readonly MissionItem[], linkStartToHome: boolean): readonly MissionItem[] => {
  const flown = (linkStartToHome ? items : items.filter((it) => it.index !== 0)).filter((it) => it.routed && (it.index !== 0 || it.placed));
  const home = items.find((it) => it.index === 0 && it.closesRoute && it.placed);
  return home !== undefined && flown.some((it) => it.index !== 0) ? [...flown, { ...home, exit: null }] : flown;
};

const surveyRoute = (survey: Survey): readonly TrackPoint[] =>
  survey.transects.length >= 2 ? survey.transects : survey.flightLoop.length >= 2 ? closedLoop(survey.flightLoop) : [];

type FlownRoute = {
  (items: readonly MissionItem[], linkStartToHome: boolean): readonly MissionItem[];
  (survey: Survey): readonly TrackPoint[];
};

export const flownRoute: FlownRoute = ((first: readonly MissionItem[] | Survey, linkStartToHome: boolean = false) =>
  'transects' in first ? surveyRoute(first) : missionRoute(first, linkStartToHome)) as FlownRoute;

export const legArrows = (items: readonly MissionItem[], linkStartToHome: boolean): readonly TransectArrow[] => {
  const legs = zipWithNext(flownRoute(items, linkStartToHome));
  const walk = legs.reduce<ArrowWalk>((walked, [from, to], leg) => {
    if (to.index === 1) return { ...walked, arrows: [...walked.arrows, false] };
    const boundary = (leg === 0 && from.index === 0) || from.complexPattern || to.complexPattern;
    const spaced = !boundary && walked.count > LEG_ARROW_SPACING;
    return { arrows: [...walked.arrows, boundary || spaced], count: spaced ? 1 : walked.count + 1 };
  }, { arrows: [], count: 0 });
  const marked = walk.arrows.map((arrow, leg) => arrow || leg === legs.length - 1);
  return legs
    .filter(([, to], leg) => (marked[leg] ?? false) && !to.legBroken)
    .map(([from, to]) => arrowOn(from.exit ?? pointOf(from), pointOf(to), LEG_ARROW_QUARTER));
};

export const missionPath = (items: readonly MissionItem[], linkStartToHome: boolean): Feature | null => {
  const runs = flownRoute(items, linkStartToHome).reduce<readonly (readonly MissionItem[])[]>(
    (held, item) => (item.legBroken || held.length === 0 ? [...held, [item]] : [...held.slice(0, -1), [...(held[held.length - 1] ?? []), item]]),
    [],
  );
  const lines = runs
    .map((run) => run.flatMap((item) => [[item.longitude, item.latitude], ...(item.exit === null ? [] : [[item.exit.longitude, item.exit.latitude]])]))
    .filter((it) => it.length >= 2);
  return lines.length === 0 ? null : { type: 'Feature', geometry: { type: 'MultiLineString', coordinates: lines }, properties: {} };
};

export type OtherMission = { items: readonly MissionItem[]; linkStartToHome: boolean };

const OTHER_VEHICLE_WAYPOINT = -1;

export const renderMission = (
  style: MapLibreMap,
  items: readonly MissionItem[],
  linkStartToHome: boolean,
  selectedIndex: number | null = null,
  others: readonly OtherMission[] = [],
  landings: readonly LandingPattern[] = [],
  legs: boolean = false,
): void => {
  setGeoJson(style, LEG_LABEL_SOURCE, featureCollection((legs ? legLabels(items) : []).map(([at, text]) => pointFeature(at.longitude, at.latitude, { [LEG_LABEL_PROPERTY]: text }))));
  const otherMarkers = others.flatMap((other) => missionFeatures(other.items).features.map((it) => ({ ...it, properties: { ...it.properties, [WAYPOINT_ID_PROPERTY]: OTHER_VEHICLE_WAYPOINT } })));
  setGeoJson(style, MISSION_SOURCE, featureCollection([...missionFeatures(items, selectedIndex, landings).features, ...otherMarkers]));
  const paths = [missionPath(items, linkStartToHome), ...others.map((it) => missionPath(it.items, it.linkStartToHome))].filter((it): it is Feature => it !== null);
  setGeoJson(style, MISSION_PATH_SOURCE, featureCollection(paths));
};

export const FENCE_SOURCE = 'aircast-fence';
export const FENCE_FILL_LAYER = 'aircast-fence-fill';
export const FENCE_LINE_LAYER = 'aircast-fence-line';
export const FIRMWARE_FENCE_SOURCE = 'aircast-firmware-fence';
export const FIRMWARE_FENCE_LAYER = 'aircast-firmware-fence-line';
export const RALLY_SOURCE = 'aircast-rally';
export const RALLY_LAYER = 'aircast-rally-layer';
export const RALLY_LABEL_LAYER = 'aircast-rally-label-layer';

const fenceColour = (): ExpressionSpecification => ['case', flagged(KEEPS_IN_PROPERTY), KEEP_IN_COLOUR, KEEP_OUT_COLOUR];

export const SHOT_SOURCE = 'aircast-shots';
export const SHOT_LAYER = 'aircast-shots-dots';
export const SHOT_COLOUR = '#FFFFFF';

export const installShotLayer = (style: MapLibreMap): void => {
  if (style.getSource(SHOT_SOURCE) === undefined) style.addSource(SHOT_SOURCE, geoJsonSource());
  if (style.getLayer(SHOT_LAYER) === undefined) {
    style.addLayer({ id: SHOT_LAYER, type: 'circle', source: SHOT_SOURCE, paint: { 'circle-radius': 3, 'circle-color': SHOT_COLOUR, 'circle-stroke-width': 1, 'circle-stroke-color': '#37474F' } });
  }
};

export const shotFeatures = (points: readonly TrackPoint[]): FeatureCollection => featureCollection(points.map((it) => pointFeature(it.longitude, it.latitude)));

const lettering = (text: string): SymbolLayerSpecification['layout'] => ({ 'text-field': text, 'text-size': 12, 'text-allow-overlap': true, 'text-ignore-placement': true });

export const installFenceLayers = (style: MapLibreMap): void => {
  if (style.getSource(FENCE_SOURCE) === undefined) {
    style.addSource(FENCE_SOURCE, geoJsonSource());
    style.addLayer({ id: FENCE_FILL_LAYER, type: 'fill', source: FENCE_SOURCE, paint: { 'fill-color': fenceColour(), 'fill-opacity': 0.15 } });
    style.addLayer({ id: FENCE_LINE_LAYER, type: 'line', source: FENCE_SOURCE, paint: { 'line-color': fenceColour(), 'line-width': 2.5 } });
  }
  if (style.getSource(FIRMWARE_FENCE_SOURCE) === undefined) {
    style.addSource(FIRMWARE_FENCE_SOURCE, geoJsonSource());
    style.addLayer({ id: FIRMWARE_FENCE_LAYER, type: 'line', source: FIRMWARE_FENCE_SOURCE, paint: { 'line-color': FIRMWARE_FENCE_COLOUR, 'line-width': 2.0, 'line-dasharray': [3.0, 3.0] } });
  }
  if (style.getSource(GCS_SOURCE) === undefined) {
    style.addSource(GCS_SOURCE, geoJsonSource());
    style.addLayer({ id: GCS_LAYER, type: 'circle', source: GCS_SOURCE, paint: { 'circle-color': '#FFFFFF', 'circle-radius': 6, 'circle-stroke-color': '#1976D2', 'circle-stroke-width': 3 } });
    style.addLayer({
      id: GCS_HEADING_LAYER,
      type: 'symbol',
      source: GCS_SOURCE,
      filter: ['has', GCS_HEADING_PROPERTY],
      layout: {
        'text-field': '▲',
        'text-font': NOTO,
        'text-size': 12,
        'text-rotate': ['get', GCS_HEADING_PROPERTY],
        'text-rotation-alignment': 'map',
        'text-offset': [0, -1.1],
        'text-allow-overlap': true,
        'text-ignore-placement': true,
      },
      paint: { 'text-color': '#1976D2', 'text-opacity': 0.85 },
    });
  }
  if (style.getSource(BREACH_SOURCE) === undefined) {
    style.addSource(BREACH_SOURCE, geoJsonSource());
    style.addLayer({ id: BREACH_LAYER, type: 'circle', source: BREACH_SOURCE, paint: { 'circle-color': KEEP_IN_COLOUR, 'circle-radius': 10, 'circle-stroke-color': '#000000', 'circle-stroke-width': 2 } });
    style.addLayer({ id: BREACH_LABEL_LAYER, type: 'symbol', source: BREACH_SOURCE, layout: lettering('B'), paint: { 'text-color': '#000000' } });
  }
  if (style.getSource(RALLY_SOURCE) === undefined) {
    style.addSource(RALLY_SOURCE, geoJsonSource());
    style.addLayer({ id: RALLY_LAYER, type: 'circle', source: RALLY_SOURCE, paint: { 'circle-color': '#66BB6A', 'circle-radius': 10, 'circle-stroke-color': '#1B5E20', 'circle-stroke-width': 2 } });
    style.addLayer({ id: RALLY_LABEL_LAYER, type: 'symbol', source: RALLY_SOURCE, layout: lettering('R'), paint: { 'text-color': '#000000' } });
  }
};

export const BREACH_SOURCE = 'aircast-breach-return';
export const BREACH_LAYER = 'aircast-breach-return-layer';
export const BREACH_LABEL_LAYER = 'aircast-breach-return-label';

export const breachFeatures = (point: TrackPoint | null): FeatureCollection => featureCollection(point === null ? [] : [pointFeature(point.longitude, point.latitude)]);

export const GCS_SOURCE = 'aircast-gcs';
export const GCS_LAYER = 'aircast-gcs-layer';

export const GCS_HEADING_LAYER = 'aircast-gcs-heading-layer';
export const GCS_HEADING_PROPERTY = 'heading';

export const operatorFeatures = (point: TrackPoint | null, heading: number = NaN): FeatureCollection =>
  featureCollection(point === null ? [] : [pointFeature(point.longitude, point.latitude, Number.isNaN(heading) ? {} : { [GCS_HEADING_PROPERTY]: heading })]);

export const CIRCLE_INDEX_PROPERTY = 'circleIndex';
export const KEEPS_IN_PROPERTY = 'keepsIn';
export const KEEP_IN_COLOUR = '#FF9500';
export const KEEP_OUT_COLOUR = '#FF3B30';
export const FIRMWARE_FENCE_COLOUR = '#AF52DE';

export const fenceFeatures = (polygons: readonly FencePolygon[], circles: readonly FencePolygon[] = []): FeatureCollection =>
  featureCollection([
    ...polygons.map((polygon) => ringFeature(polygon.vertices, { [KEEPS_IN_PROPERTY]: polygon.inclusion })),
    ...circles.map((circle) => ringFeature(circle.vertices, { [CIRCLE_INDEX_PROPERTY]: circle.index, [KEEPS_IN_PROPERTY]: circle.inclusion })),
  ]);

export const RALLY_INDEX_PROPERTY = 'rallyIndex';

export const rallyFeatures = (points: readonly RallyPoint[]): FeatureCollection =>
  featureCollection(points.map((point) => pointFeature(point.longitude, point.latitude, { [RALLY_INDEX_PROPERTY]: point.index })));

export const firmwareFenceFeatures = (fence: FirmwareFence | null): FeatureCollection => {
  const centre = fence?.centre ?? null;
  return featureCollection(fence === null || centre === null ? [] : [ringFeature(circleRing(centre, fence.radiusMetres))]);
};

export const renderFences = (
  style: MapLibreMap,
  polygons: readonly FencePolygon[],
  rally: readonly RallyPoint[],
  circles: readonly FencePolygon[] = [],
  firmware: FirmwareFence | null = null,
  breach: TrackPoint | null = null,
): void => {
  setGeoJson(style, BREACH_SOURCE, breachFeatures(breach));
  setGeoJson(style, FENCE_SOURCE, fenceFeatures(polygons, circles));
  setGeoJson(style, RALLY_SOURCE, rallyFeatures(rally));
  setGeoJson(style, FIRMWARE_FENCE_SOURCE, firmwareFenceFeatures(firmware));
};

export const FENCE_HANDLE_SOURCE = 'aircast-fence-handles';
export const FENCE_HANDLE_LAYER = 'aircast-fence-handle-layer';

export const POLYGON_INDEX_PROPERTY = 'polygonIndex';
export const VERTEX_INDEX_PROPERTY = 'vertexIndex';
export const HANDLE_KIND_PROPERTY = 'handleKind';

export const HANDLE_KIND_FENCE = 'fence';
export const HANDLE_KIND_SURVEY = 'survey';
export const HANDLE_KIND_CIRCLE = 'circle';
export const HANDLE_KIND_LANDING = 'landing';
export const HANDLE_KIND_FENCE_CENTRE = 'fenceCentre';
export const HANDLE_KIND_SURVEY_CENTRE = 'surveyCentre';
export const HANDLE_KIND_CIRCLE_RADIUS = 'circleRadius';
export const HANDLE_KIND_FENCE_CIRCLE_RADIUS = 'fenceCircleRadius';
export const HANDLE_KIND_LOITER_RADIUS = 'loiterRadius';
export const HANDLE_KIND_LOITER_ROTATION = 'loiterRotation';

export const SHAPE_PATH_PROPERTY = 'shapePath';
export const SPLIT_INVOKABLE_PROPERTY = 'splitInvokable';
export const MIDPOINT_SOURCE = 'aircast-midpoints';
export const MIDPOINT_LAYER = 'aircast-midpoints-layer';

export const LANDING_PLACE_APPROACH = 0;
export const LANDING_PLACE_TOUCHDOWN = 1;

const tapOnlyHidden = (): ExpressionSpecification => ['match', ['get', HANDLE_KIND_PROPERTY], HANDLE_KIND_LOITER_ROTATION, 0, 1];

export const installFenceHandleLayer = (style: MapLibreMap): void => {
  if (style.getSource(FENCE_HANDLE_SOURCE) !== undefined) return;
  style.addSource(FENCE_HANDLE_SOURCE, geoJsonSource());
  style.addLayer({
    id: FENCE_HANDLE_LAYER,
    type: 'circle',
    source: FENCE_HANDLE_SOURCE,
    paint: { 'circle-color': '#FFFFFF', 'circle-radius': 7, 'circle-stroke-color': '#1565C0', 'circle-stroke-width': 3, 'circle-opacity': tapOnlyHidden(), 'circle-stroke-opacity': tapOnlyHidden() },
  });
};

const handleFeature = (at: TrackPoint, kind: string, owner: number, vertex: number): Feature =>
  pointFeature(at.longitude, at.latitude, { [HANDLE_KIND_PROPERTY]: kind, [POLYGON_INDEX_PROPERTY]: owner, [VERTEX_INDEX_PROPERTY]: vertex });

const handleFeatures = (kind: string, owner: number, vertices: readonly TrackPoint[]): readonly Feature[] => vertices.map((point, vertex) => handleFeature(point, kind, owner, vertex));

export const vertexHandleFeatures = (
  polygons: readonly FencePolygon[],
  surveys: readonly Survey[],
  circles: readonly FenceCircle[] = [],
  landings: readonly LandingPattern[] = [],
): FeatureCollection =>
  featureCollection([
    ...polygons.flatMap((it) => handleFeatures(HANDLE_KIND_FENCE, it.index, it.vertices)),
    ...surveys.flatMap((it) => handleFeatures(HANDLE_KIND_SURVEY, it.index, it.area)),
    ...circles.flatMap((it) => handleFeatures(HANDLE_KIND_CIRCLE, it.index, [it.centre])),
    ...circles.flatMap((it) => handleFeatures(HANDLE_KIND_FENCE_CIRCLE_RADIUS, it.index, [circleEdge(it)])),
    ...landings.flatMap(landingHandleFeatures),
  ]);

const landingHandleFeatures = (pattern: LandingPattern): readonly Feature[] =>
  [
    ...(pattern.finalApproach === null ? [] : [[LANDING_PLACE_APPROACH, pattern.finalApproach] as const]),
    ...(pattern.landing === null ? [] : [[LANDING_PLACE_TOUCHDOWN, pattern.landing] as const]),
  ].map(([place, at]) => handleFeature(at, HANDLE_KIND_LANDING, pattern.index, place));

export const renderVertexHandles = (
  style: MapLibreMap,
  polygons: readonly FencePolygon[],
  surveys: readonly Survey[],
  circles: readonly FenceCircle[] = [],
  landings: readonly LandingPattern[] = [],
  circled: ReadonlySet<string> = new Set(),
  loiterHandles: readonly Feature[] = [],
): void => {
  const cornered = polygons.map((it) => (circled.has(fencePath(it.index)) ? { ...it, vertices: [] } : it));
  const cornerSurveys = surveys.map((it) => (circled.has(surveyPath(it)) ? { ...it, area: [] } : it));
  setGeoJson(
    style,
    FENCE_HANDLE_SOURCE,
    featureCollection([
      ...vertexHandleFeatures(cornered, cornerSurveys, circles, landings).features,
      ...centreHandleFeatures(polygons, surveys),
      ...radiusHandleFeatures(polygons, surveys, circled),
      ...loiterHandles,
    ]),
  );
};

export const MINIMUM_CIRCLE_RADIUS_METRES = 0.1;

export const circleEdge = (circle: FenceCircle): TrackPoint => pointAt(circle.centre, circle.radiusMetres, 90.0);

export const draggedCircleRadius = (circle: FenceCircle, to: TrackPoint): number => {
  const shownPerMetre = circle.radiusMetres > 0.0 ? circle.radius / circle.radiusMetres : 1.0;
  const wanted = metresBetween(circle.centre, to) * shownPerMetre;
  return coerceAtMost(coerceAtLeast(wanted, circle.radiusMinimum ?? MINIMUM_CIRCLE_RADIUS_METRES * shownPerMetre), circle.radiusMaximum ?? Number.MAX_VALUE);
};

export const radiusHandleFeatures = (polygons: readonly FencePolygon[], surveys: readonly Survey[], circled: ReadonlySet<string>): readonly Feature[] => {
  const handle = (vertices: readonly TrackPoint[], owner: number, fence: boolean): readonly Feature[] => {
    const centre = polygonCentre(vertices);
    const radius = centre === null ? null : circleRadius(vertices);
    return centre === null || radius === null ? [] : [handleFeature(pointAt(centre, radius, 90.0), HANDLE_KIND_CIRCLE_RADIUS, owner, fence ? 0 : 1)];
  };
  return [
    ...polygons.filter((it) => circled.has(fencePath(it.index))).flatMap((it) => handle(it.vertices, it.index, true)),
    ...surveys.filter((it) => circled.has(surveyPath(it))).flatMap((it) => handle(it.area, it.index, false)),
  ];
};

const loiterArrowAzimuths: readonly number[] = [0.0, 180.0];

const loiterItems = (items: readonly MissionItem[]): readonly MissionItem[] =>
  items.filter((it) => Number.isFinite(it.loiterRadius) && it.loiterRadius !== 0.0 && isPlottable(it.latitude, it.longitude));

export const draggedLoiterRadius = (item: MissionItem, to: TrackPoint): number => {
  const metres = metresBetween(pointOf(item), to);
  return item.loiterRadius >= 0 ? metres : -metres;
};

export const loiterRotationArrows = (items: readonly MissionItem[]): readonly TransectArrow[] =>
  loiterItems(items).flatMap((item) =>
    loiterArrowAzimuths.map((azimuth) => ({ at: pointAt(pointOf(item), Math.abs(item.loiterRadius), azimuth), bearing: (azimuth + (item.loiterRadius >= 0 ? 90.0 : 270.0)) % 360.0 })),
  );

export const loiterHandleFeatures = (items: readonly MissionItem[], selected: number | null): readonly Feature[] => {
  const chosen = loiterItems(items).filter((it) => it.index === selected);
  return [
    ...chosen.flatMap((item) => loiterRotationArrows([item]).map((arrow, vertex) => handleFeature(arrow.at, HANDLE_KIND_LOITER_ROTATION, item.index, vertex))),
    ...chosen.map((item) => handleFeature(pointAt(pointOf(item), Math.abs(item.loiterRadius), 90.0), HANDLE_KIND_LOITER_RADIUS, item.index, 0)),
  ];
};

export const centreHandleFeatures = (polygons: readonly FencePolygon[], surveys: readonly Survey[]): readonly Feature[] => [
  ...polygons.flatMap((fence) => {
    const centre = polygonCentre(fence.vertices);
    return centre === null ? [] : handleFeatures(HANDLE_KIND_FENCE_CENTRE, fence.index, [centre]);
  }),
  ...surveys
    .filter((it) => it.property !== CORRIDOR_PROPERTY)
    .flatMap((area) => {
      const centre = polygonCentre(area.area);
      return centre === null ? [] : handleFeatures(HANDLE_KIND_SURVEY_CENTRE, area.index, [centre]);
    }),
];

export const SURVEY_AREA_SOURCE = 'aircast-survey-area';
export const SURVEY_AREA_LAYER = 'aircast-survey-area-layer';
export const SURVEY_COLLISION = 'collision';
export const TERRAIN_COLLISION = 'terrainCollision';
export const SURVEY_TRANSECT_SOURCE = 'aircast-survey-transects';
export const SURVEY_TRANSECT_LAYER = 'aircast-survey-transect-layer';
export const LANDING_PATH_SOURCE = 'aircast-landing-path';
export const LANDING_PATH_LAYER = 'aircast-landing-path-layer';
export const LANDING_LOITER_SOURCE = 'aircast-landing-loiter';
export const LANDING_AREA_SOURCE = 'aircast-landing-area';
export const LANDING_AREA_LAYER = 'aircast-landing-area-layer';
export const LANDING_SHAPE_KIND = 'kind';
export const LANDING_LABEL_SOURCE = 'aircast-landing-labels';
export const LANDING_LABEL_LAYER = 'aircast-landing-label-layer';
export const LANDING_LABEL_TEXT = 'text';
export const EDGE_LABEL_SOURCE = 'aircast-edge-labels';
export const EDGE_LABEL_LAYER = 'aircast-edge-label-layer';
export const LANDING_LOITER_LAYER = 'aircast-landing-loiter-layer';

export const SURVEY_LINE_SOURCE = 'aircast-survey-line';
export const SURVEY_LINE_LAYER = 'aircast-survey-line-layer';

export const installMidpointLayer = (style: MapLibreMap): void => {
  if (style.getSource(MIDPOINT_SOURCE) !== undefined) return;
  style.addSource(MIDPOINT_SOURCE, geoJsonSource());
  style.addLayer({
    id: MIDPOINT_LAYER,
    type: 'circle',
    source: MIDPOINT_SOURCE,
    paint: { 'circle-color': '#1565C0', 'circle-radius': 5, 'circle-stroke-color': '#FFFFFF', 'circle-stroke-width': 2, 'circle-opacity': 0.85 },
  });
};

export const midpointFeatures = (shapes: readonly (EditableShape | null)[]): FeatureCollection =>
  featureCollection(
    shapes
      .filter((it): it is EditableShape => it !== null && it.splitInvokable.trim() !== '')
      .flatMap((shape) =>
        shape.midpoints.map((at, segment) => pointFeature(at.longitude, at.latitude, { [SHAPE_PATH_PROPERTY]: shape.path, [SPLIT_INVOKABLE_PROPERTY]: shape.splitInvokable, [VERTEX_INDEX_PROPERTY]: segment })),
      ),
  );

export const MISSION_SPLIT_PATH = 'plan.missionController';
export const MISSION_SPLIT_INVOKABLE = 'insertSimpleMissionItem';

export const legSplit = (items: readonly MissionItem[], selected: number | null): TrackPoint | null => {
  const current = items.find((it) => it.index === selected);
  if (current === undefined || !current.routed) return null;
  const previous = items.filter((it) => it.index >= 1 && it.index < current.index && it.routed).reduce<MissionItem | null>((best, it) => (best === null || it.index > best.index ? it : best), null);
  if (previous === null) return null;
  const from = previous.exit ?? pointOf(previous);
  const to = pointOf(current);
  return pointAt(from, metresBetween(from, to) / 2, azimuthBetween(from, to));
};

export const renderMidpoints = (style: MapLibreMap, polygons: readonly FencePolygon[], surveys: readonly Survey[], items: readonly MissionItem[] = [], selected: number | null = null): void => {
  const split = legSplit(items, selected);
  const splitFeatures =
    split === null
      ? []
      : [pointFeature(split.longitude, split.latitude, { [SHAPE_PATH_PROPERTY]: MISSION_SPLIT_PATH, [SPLIT_INVOKABLE_PROPERTY]: MISSION_SPLIT_INVOKABLE, [VERTEX_INDEX_PROPERTY]: selected ?? 0 })];
  const shapes = midpointFeatures([...polygons.map((it) => it.editable), ...surveys.map((it) => it.editable)]).features;
  setGeoJson(style, MIDPOINT_SOURCE, featureCollection([...shapes, ...splitFeatures]));
};

const plainLabels = (id: string, source: string): SymbolLayerSpecification => ({
  id,
  type: 'symbol',
  source,
  layout: { 'text-field': ['get', LANDING_LABEL_TEXT], 'text-size': 12, 'text-allow-overlap': true, 'text-ignore-placement': true },
  paint: { 'text-color': '#FFFFFF', 'text-halo-color': '#000000', 'text-halo-width': 1.5 },
});

export const installLandingLayers = (style: MapLibreMap): void => {
  if (style.getSource(EDGE_LABEL_SOURCE) === undefined) {
    style.addSource(EDGE_LABEL_SOURCE, geoJsonSource());
    style.addLayer(plainLabels(EDGE_LABEL_LAYER, EDGE_LABEL_SOURCE));
  }
  if (style.getSource(LANDING_LABEL_SOURCE) === undefined) {
    style.addSource(LANDING_LABEL_SOURCE, geoJsonSource());
    style.addLayer(plainLabels(LANDING_LABEL_LAYER, LANDING_LABEL_SOURCE));
  }
  if (style.getSource(LANDING_AREA_SOURCE) === undefined) {
    style.addSource(LANDING_AREA_SOURCE, geoJsonSource());
    style.addLayer({
      id: LANDING_AREA_LAYER,
      type: 'fill',
      source: LANDING_AREA_SOURCE,
      paint: {
        'fill-color': ['case', flagged(TERRAIN_COLLISION), '#FF0000', ['match', ['get', LANDING_SHAPE_KIND], LANDING_AREA_KIND, '#00FF00', '#FFA500']],
        'fill-opacity': 0.5,
        'fill-outline-color': '#000000',
      },
    });
  }
  if (style.getSource(LANDING_LOITER_SOURCE) === undefined) {
    style.addSource(LANDING_LOITER_SOURCE, geoJsonSource());
    style.addLayer({ id: LANDING_LOITER_LAYER, type: 'line', source: LANDING_LOITER_SOURCE, paint: { 'line-color': ['case', flagged(TERRAIN_COLLISION), '#FF0000', '#26C6DA'], 'line-width': 2.5 } });
  }
  if (style.getSource(LANDING_PATH_SOURCE) === undefined) {
    style.addSource(LANDING_PATH_SOURCE, geoJsonSource());
    style.addLayer({ id: LANDING_PATH_LAYER, type: 'line', source: LANDING_PATH_SOURCE, paint: { 'line-color': '#26C6DA', 'line-width': 3, 'line-dasharray': [3, 2] } });
  }
};

export const installSurveyLayers = (style: MapLibreMap): void => {
  if (style.getSource(SURVEY_AREA_SOURCE) === undefined) {
    style.addSource(SURVEY_AREA_SOURCE, geoJsonSource());
    style.addLayer({
      id: SURVEY_AREA_LAYER,
      type: 'fill',
      source: SURVEY_AREA_SOURCE,
      paint: { 'fill-color': ['case', flagged(SURVEY_COLLISION), '#FF0000', '#AB47BC'], 'fill-opacity': 0.18, 'fill-outline-color': '#7B1FA2' },
    });
  }
  if (style.getSource(SURVEY_LINE_SOURCE) === undefined) {
    style.addSource(SURVEY_LINE_SOURCE, geoJsonSource());
    style.addLayer({ id: SURVEY_LINE_LAYER, type: 'line', source: SURVEY_LINE_SOURCE, paint: { 'line-color': '#7B1FA2', 'line-width': 3 } });
  }
  if (style.getSource(SURVEY_TRANSECT_SOURCE) === undefined) {
    style.addSource(SURVEY_TRANSECT_SOURCE, geoJsonSource());
    style.addLayer({ id: SURVEY_TRANSECT_LAYER, type: 'line', source: SURVEY_TRANSECT_SOURCE, paint: { 'line-color': '#E040FB', 'line-width': 2.5 } });
  }
};

export const shadedArea = (survey: Survey): readonly TrackPoint[] => (survey.shape === SHAPE_AREA ? survey.area : survey.outline);

export const surveyAreaFeatures = (surveys: readonly Survey[]): FeatureCollection =>
  featureCollection(
    surveys.flatMap((survey) => {
      const area = shadedArea(survey);
      return area.length < 3 ? [] : [ringFeature(area, { [SURVEY_COLLISION]: survey.collides })];
    }),
  );

export const surveyLineFeatures = (surveys: readonly Survey[]): FeatureCollection =>
  featureCollection(surveys.filter((it) => it.shape === SHAPE_LINE && it.area.length >= 2).map((survey) => lineFeature(survey.area)));

const closedLoop = (points: readonly TrackPoint[]): readonly TrackPoint[] => (samePoint(points[0], points[points.length - 1]) ? points : [...points, ...points.slice(0, 1)]);

export const surveyTransectFeatures = (surveys: readonly Survey[]): FeatureCollection =>
  featureCollection(
    surveys.flatMap((survey) => {
      const route = flownRoute(survey);
      return route.length < 2 ? [] : [lineFeature(route)];
    }),
  );

export const renderSurveys = (style: MapLibreMap, surveys: readonly Survey[]): void => {
  setGeoJson(style, SURVEY_AREA_SOURCE, surveyAreaFeatures(surveys));
  setGeoJson(style, SURVEY_LINE_SOURCE, surveyLineFeatures(surveys));
  setGeoJson(style, SURVEY_TRANSECT_SOURCE, surveyTransectFeatures(surveys));
};
