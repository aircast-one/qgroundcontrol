import { type Json, type JsonArray, isJson, isNull, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { format } from '@/kotlin/Std';
import { typedNumber } from './AltitudeEntry';
import { coordinateJson, coordinateJsonAt, invokeOk, setOk, settingJson } from './BridgeCalls';
import { type EditableShape, editableShape } from './EditableShape';
import { MapBridge } from './MapBridge';
import type { MapHit } from './MissionEditing';
import { type TrackPoint, isPlottable } from './VehicleMap';
import { TAP } from '@/kotlin/Platform';

export const FENCE_ROOT = 'plan.geoFenceController';
export const RALLY_ROOT = 'plan.rallyPointController';

export const FENCE_POLYGONS = `${FENCE_ROOT}.polygons`;
export const FENCE_CIRCLES = `${FENCE_ROOT}.circles`;
export const RALLY_POINTS = `${RALLY_ROOT}.points`;
export const FENCES_VIEW = 'view.fences';
export const FLY_FENCES_VIEW = 'view.flyFences';

export type FencePolygon = {
  index: number;
  inclusion: boolean;
  vertices: readonly TrackPoint[];
  editable: EditableShape | null;
  kindText: string;
  detailText: string;
};

export const FencePolygon = (fields: { index: number; inclusion: boolean; vertices: readonly TrackPoint[]; editable?: EditableShape | null; kindText?: string; detailText?: string }): FencePolygon => ({
  editable: null,
  kindText: '',
  detailText: '',
  ...fields,
});

export type FenceRow = { index: number; circle: boolean; title: string; detail: string; inclusion: boolean | null; radius: string };

export const FenceRow = (fields: { index: number; circle: boolean; title: string; detail: string; inclusion?: boolean | null; radius?: string }): FenceRow => ({ inclusion: null, radius: '', ...fields });

export const inclusionText = (inclusion: boolean): string => (inclusion ? 'Inclusion' : 'Exclusion');

const orBlank = (text: string, fallback: string): string => (text.trim() === '' ? fallback : text);

export const fenceRows = (polygons: readonly FencePolygon[], circles: readonly FenceCircle[]): readonly FenceRow[] => [
  ...polygons.map((it) => FenceRow({ index: it.index, circle: false, title: orBlank(it.kindText, `Polygon ${it.index + 1}`), detail: inclusionText(it.inclusion), inclusion: it.inclusion })),
  ...circles.map((it) =>
    FenceRow({
      index: it.index,
      circle: true,
      title: orBlank(it.kindText, `Circle ${it.index + 1}`),
      detail: inclusionText(it.inclusion),
      radius: [trimmedRadius(it.radius), it.radiusUnits].filter((part) => part.trim() !== '').join(' '),
    }),
  ),
];

export const fenceHeading = (row: FenceRow, previous: FenceRow | null): string | null =>
  previous === null || previous.circle !== row.circle ? (row.circle ? 'Circular fences' : 'Polygon fences') : null;

export const fenceRowHit = (row: FenceRow): MapHit => (row.circle ? { kind: 'Circle', index: row.index } : { kind: 'FenceVertex', polygon: row.index, vertex: 0 });

export const rowSelected = (row: FenceRow, selected: MapHit | null): boolean => {
  switch (selected?.kind) {
    case 'Circle':
    case 'CircleCentre':
      return row.circle && selected.index === row.index;
    case 'FenceVertex':
      return !row.circle && selected.polygon === row.index;
    default:
      return false;
  }
};

export const fenceSelectionAfterRemove = (row: FenceRow, selected: MapHit | null): MapHit | null => {
  const moved = (index: number, rebuild: (index: number) => MapHit): MapHit | null => (index === row.index ? null : index > row.index ? rebuild(index - 1) : selected);
  switch (selected?.kind) {
    case 'FenceVertex':
      return row.circle ? selected : moved(selected.polygon, (it) => ({ kind: 'FenceVertex', polygon: it, vertex: selected.vertex }));
    case 'Circle':
      return !row.circle ? selected : moved(selected.index, (it) => ({ kind: 'Circle', index: it }));
    case 'CircleCentre':
      return !row.circle ? selected : moved(selected.index, (it) => ({ kind: 'CircleCentre', index: it }));
    default:
      return selected;
  }
};

export const rallyAfterRemove = (removed: number, countBefore: number): MapHit | null => {
  const last = countBefore - 2;
  return last >= 0 ? { kind: 'Rally', index: Math.min(removed, last) } : null;
};

export const NO_GEOFENCE = 'No geofence – keep the vehicle inside a boundary, or out of an area.';
export const NO_RALLY_POINTS = `No rally points – alternate landing points for Return to Launch. ${TAP} the map to place one.`;
export const GEOFENCE_NOT_SUPPORTED = 'Not supported – this vehicle does not support geofence.';
export const RALLY_NOT_SUPPORTED = 'Not supported – this vehicle does not support rally points.';

export const rallyRows = (points: readonly RallyPoint[]): readonly FenceRow[] =>
  points.map((point) => {
    const height = Number.isFinite(point.altitude) ? `${plainAltitude(point.altitude)} ${orBlank(point.altitudeUnits, 'm')}` : null;
    const where = format('%.6f, %.6f', point.latitude, point.longitude);
    return FenceRow({ index: point.index, circle: false, title: `Rally ${point.index + 1}`, detail: [height, where].filter((it) => it !== null).join(' · ') });
  });

const plainAltitude = (value: number): string => (value === Math.floor(value) ? String(Math.trunc(value)) : format('%.1f', value));

export type FenceCircle = {
  index: number;
  inclusion: boolean;
  centre: TrackPoint;
  radius: number;
  detailText: string;
  kindText: string;
  radiusMinimum: number | null;
  radiusMaximum: number | null;
  radiusMetres: number;
  radiusUnits: string;
};

export const FenceCircle = (fields: {
  index: number;
  inclusion: boolean;
  centre: TrackPoint;
  radius: number;
  detailText?: string;
  kindText?: string;
  radiusMinimum?: number | null;
  radiusMaximum?: number | null;
  radiusMetres?: number;
  radiusUnits?: string;
}): FenceCircle => ({ detailText: '', kindText: '', radiusMinimum: null, radiusMaximum: null, radiusMetres: fields.radius, radiusUnits: '', ...fields });

export type RallyPoint = {
  index: number;
  latitude: number;
  longitude: number;
  altitudeMetres: number;
  altitude: number;
  altitudeUnits: string;
  altitudePath: string;
};

export const RallyPoint = (fields: { index: number; latitude: number; longitude: number; altitudeMetres?: number; altitude?: number; altitudeUnits?: string; altitudePath?: string }): RallyPoint => ({
  altitudeMetres: 0.0,
  altitude: NaN,
  altitudeUnits: '',
  altitudePath: '',
  ...fields,
});

export const coordinate = (json: Json | null): TrackPoint | null => {
  if (json === null) return null;
  const latitude = optDouble(json, 'latitude', NaN);
  const longitude = optDouble(json, 'longitude', NaN);
  return isPlottable(latitude, longitude) ? { latitude, longitude } : null;
};

const listed = (json: Json | null, key: string): JsonArray | null => optJSONArray(json, key);

const objectAt = (list: JsonArray, index: number): Json | null => {
  const entry = list[index];
  return isJson(entry) ? entry : null;
};

const objects = (list: JsonArray): readonly (readonly [Json, number])[] =>
  list.flatMap((_, index) => {
    const element = objectAt(list, index);
    return element === null ? [] : [[element, index] as const];
  });

export const FENCE_POLYGON_MINIMUM = 3;

export const cornerRemovable = (polygon: FencePolygon | null): boolean => polygon?.editable?.canRemoveVertex === true;

export const fencePolygons = async (json: Json | null): Promise<readonly FencePolygon[]> => {
  const list = listed(json, 'polygons');
  if (list === null) return [];
  const drawn = objects(list).flatMap(([element, index]) => {
    const corners = optJSONArray(element, 'vertices');
    if (corners === null) return [];
    const vertices = corners.flatMap((_, at) => {
      const point = coordinate(objectAt(corners, at));
      return point === null ? [] : [point];
    });
    return vertices.length < FENCE_POLYGON_MINIMUM ? [] : [{ element, at: optInt(element, 'index', index), vertices }];
  });
  return Promise.all(
    drawn.map(async ({ element, at, vertices }) =>
      FencePolygon({
        index: at,
        inclusion: optBoolean(element, 'inclusion', true),
        vertices,
        editable: await editableShape(`${FENCE_POLYGONS}.${at}`),
        kindText: optText(element, 'kindText'),
        detailText: optText(element, 'detailText'),
      }),
    ),
  );
};

export type FirmwareFence = { radiusMetres: number; radiusText: string; centre: TrackPoint | null };

export const firmwareFence = (json: Json | null): FirmwareFence | null => {
  const served = optJSONObject(json, 'firmwareFence');
  if (served === null) return null;
  const radius = optDouble(served, 'radiusMetres', NaN);
  if (Number.isNaN(radius) || radius <= 0.0) return null;
  return { radiusMetres: radius, radiusText: optText(served, 'radiusText'), centre: coordinate(optJSONObject(served, 'centre')) };
};

const bound = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isFinite(value) && value > 0.0 ? value : null;
};

export const CIRCLE_STEP = 1.5;

export const grownRadius = (circle: FenceCircle): number | null => {
  const wanted = circle.radius * CIRCLE_STEP;
  const ceiling = circle.radiusMaximum;
  if (ceiling === null) return wanted;
  return circle.radius >= ceiling ? null : Math.min(wanted, ceiling);
};

export const trimmedRadius = (radius: number): string => format('%.1f', radius).replace(/0+$/, '').replace(/\.+$/, '');

export const typedRadius = (text: string, circle: FenceCircle): number | null => {
  const wanted = typedNumber(text);
  if (wanted === null || wanted <= 0.0) return null;
  if (circle.radiusMinimum !== null && wanted < circle.radiusMinimum) return null;
  return circle.radiusMaximum !== null && wanted > circle.radiusMaximum ? null : wanted;
};

export const shrunkRadius = (circle: FenceCircle): number | null => {
  const wanted = circle.radius / CIRCLE_STEP;
  const floor = circle.radiusMinimum;
  if (floor === null) return wanted > 0.0 ? wanted : null;
  return circle.radius <= floor ? null : Math.max(wanted, floor);
};

export const fenceCircles = (json: Json | null): readonly FenceCircle[] => {
  const list = listed(json, 'circles');
  if (list === null) return [];
  return objects(list).flatMap(([element, index]) => {
    const centre = coordinate(optJSONObject(element, 'centre'));
    if (centre === null) return [];
    const radius = optDouble(element, 'radius', NaN);
    if (Number.isNaN(radius) || radius <= 0.0) return [];
    const metres = optDouble(element, 'radiusMetres', radius);
    return [
      FenceCircle({
        index: optInt(element, 'index', index),
        inclusion: optBoolean(element, 'inclusion', true),
        centre,
        radius,
        detailText: optText(element, 'detailText'),
        kindText: optText(element, 'kindText'),
        radiusMinimum: bound(element, 'radiusMinimum'),
        radiusMaximum: bound(element, 'radiusMaximum'),
        radiusMetres: Number.isFinite(metres) ? metres : radius,
        radiusUnits: optText(element, 'radiusUnits'),
      }),
    ];
  });
};

export const rallyPoints = (json: Json | null): readonly RallyPoint[] => {
  const list = listed(json, 'rallyPoints');
  if (list === null) return [];
  return objects(list).flatMap(([element, index]) => {
    const point = coordinate(element);
    if (point === null) return [];
    const metres = optDouble(element, 'altitudeMetres', 0.0);
    return [
      RallyPoint({
        index: optInt(element, 'index', index),
        latitude: point.latitude,
        longitude: point.longitude,
        altitudeMetres: Number.isFinite(metres) ? metres : 0.0,
        altitude: optDouble(element, 'altitude', NaN),
        altitudeUnits: optText(element, 'altitudeUnits'),
        altitudePath: optText(element, 'altitudePath'),
      }),
    ];
  });
};

export const rallyMovePayload = (latitude: number, longitude: number, altitudeMetres: number): string => settingJson(coordinateJson(latitude, longitude, altitudeMetres));

export const rallyAltitudeFor = (rally: readonly RallyPoint[], index: number): number => rally.find((it) => it.index === index)?.altitudeMetres ?? 0.0;

export const BREACH_RETURN_PATH = `${FENCE_ROOT}.breachReturnPoint`;

export type BreachReturn = { point: TrackPoint; altitude: number | null; units: string; altitudePath: string };

export const breachReturn = (json: Json | null): BreachReturn | null => {
  const point = coordinate(optJSONObject(json, 'breachReturnPoint'));
  if (point === null) return null;
  const altitude = optJSONObject(json, 'breachReturnAltitude');
  const value = altitude === null || isNull(altitude, 'value') ? NaN : optDouble(altitude, 'value');
  return {
    point,
    altitude: Number.isFinite(value) ? value : null,
    units: optText(altitude, 'units'),
    altitudePath: optText(altitude, 'path'),
  };
};

export const FenceBridge = {
  read: (): Promise<Json | null> => MapBridge.read(FENCES_VIEW),

  readFlown: (): Promise<Json | null> => MapBridge.read(FLY_FENCES_VIEW),

  addInclusionPolygon: (topLeft: TrackPoint, bottomRight: TrackPoint): Promise<boolean> =>
    invokeOk(`${FENCE_ROOT}.addInclusionPolygon`, `[${coordinateJsonAt(topLeft)}, ${coordinateJsonAt(bottomRight)}]`),

  addInclusionCircle: (topLeft: TrackPoint, bottomRight: TrackPoint): Promise<boolean> =>
    invokeOk(`${FENCE_ROOT}.addInclusionCircle`, `[${coordinateJsonAt(topLeft)}, ${coordinateJsonAt(bottomRight)}]`),

  addRallyPoint: (latitude: number, longitude: number): Promise<boolean> => invokeOk(`${RALLY_ROOT}.addPoint`, `[${coordinateJson(latitude, longitude)}]`),

  moveRallyPoint: (index: number, latitude: number, longitude: number, altitudeMetres: number): Promise<boolean> =>
    setOk(`${RALLY_POINTS}.${index}.coordinate`, rallyMovePayload(latitude, longitude, altitudeMetres)),

  moveCircle: (index: number, latitude: number, longitude: number): Promise<boolean> => setOk(`${FENCE_CIRCLES}.${index}.center`, settingJson(coordinateJson(latitude, longitude))),

  setCircleRadius: (index: number, shown: number): Promise<boolean> => setOk(`${FENCE_CIRCLES}.${index}.radius`, settingJson(`${shown}`)),

  deletePolygon: (index: number): Promise<boolean> => invokeOk(`${FENCE_ROOT}.deletePolygon`, `[${index}]`),

  setBreachReturn: (at: TrackPoint): Promise<boolean> => setOk(BREACH_RETURN_PATH, settingJson(coordinateJsonAt(at))),

  clearBreachReturn: (): Promise<boolean> => setOk(BREACH_RETURN_PATH, settingJson('null')),

  setBreachAltitude: (path: string, shown: number): Promise<boolean> => setOk(path, settingJson(`${shown}`)),

  deleteCircle: (index: number): Promise<boolean> => invokeOk(`${FENCE_ROOT}.deleteCircle`, `[${index}]`),

  setRallyAltitude: (path: string, shown: number): Promise<boolean> => setOk(path, settingJson(`${shown}`)),

  removeRallyPoint: (index: number): Promise<boolean> => invokeOk(`${RALLY_ROOT}.removePoint`, `["@${RALLY_POINTS}.${index}"]`),

  setPolygonInclusion: (index: number, inclusion: boolean): Promise<boolean> => setOk(`${FENCE_POLYGONS}.${index}.inclusion`, settingJson(String(inclusion))),

  removeVertex: (polygon: number, vertex: number): Promise<boolean> => invokeOk(`${FENCE_POLYGONS}.${polygon}.removeVertex`, `[${vertex}]`),

  adjustVertex: (polygon: number, vertex: number, latitude: number, longitude: number): Promise<boolean> =>
    invokeOk(`${FENCE_POLYGONS}.${polygon}.adjustVertex`, `[${vertex}, ${coordinateJson(latitude, longitude)}]`),
};

const EARTH_RADIUS_M = 6_371_000.0;
const CIRCLE_SEGMENTS = 48;
const RADIANS = Math.PI / 180;

export const circleRing = (centre: TrackPoint, radiusMetres: number, segments: number = CIRCLE_SEGMENTS): readonly TrackPoint[] =>
  radiusMetres <= 0.0 || segments < 3 ? [] : Array.from({ length: segments }, (_, step) => pointAt(centre, radiusMetres, (360.0 * step) / segments));

export const azimuthBetween = (from: TrackPoint, to: TrackPoint): number => {
  const fromLat = from.latitude * RADIANS;
  const toLat = to.latitude * RADIANS;
  const deltaLon = (to.longitude - from.longitude) * RADIANS;
  const y = Math.sin(deltaLon) * Math.cos(toLat);
  const x = Math.cos(fromLat) * Math.sin(toLat) - Math.sin(fromLat) * Math.cos(toLat) * Math.cos(deltaLon);
  return (Math.atan2(y, x) / RADIANS + 360) % 360;
};

export const pointAt = (centre: TrackPoint, metres: number, bearingDegrees: number): TrackPoint => {
  const angular = metres / EARTH_RADIUS_M;
  const lat = centre.latitude * RADIANS;
  const lon = centre.longitude * RADIANS;
  const bearing = bearingDegrees * RADIANS;
  const pointLat = Math.asin(Math.sin(lat) * Math.cos(angular) + Math.cos(lat) * Math.sin(angular) * Math.cos(bearing));
  const pointLon = lon + Math.atan2(Math.sin(bearing) * Math.sin(angular) * Math.cos(lat), Math.cos(angular) - Math.sin(lat) * Math.sin(pointLat));
  return { latitude: pointLat / RADIANS, longitude: pointLon / RADIANS };
};

export const circlesAsPolygons = (circles: readonly FenceCircle[]): readonly FencePolygon[] =>
  circles.flatMap((circle) => {
    const ring = circleRing(circle.centre, circle.radiusMetres);
    return ring.length < 3 ? [] : [FencePolygon({ index: circle.index, inclusion: circle.inclusion, vertices: ring, kindText: circle.kindText })];
  });
