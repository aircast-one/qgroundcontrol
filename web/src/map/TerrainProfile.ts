import { type Json, isJson, isNull, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { coerceAtLeast } from '@/kotlin/Std';
import { type TrackPoint, isPlottable } from './VehicleMap';

const EARTH_RADIUS_METRES = 6_371_000.0;
const MIN_SPAN_METRES = 1.0;

export type Clearance = { collides: boolean; metres: number | null; text: string; complete: boolean };

export type ProfilePoint = { distance: number; terrain: number | null; planned: number; collision: boolean };

export const ProfilePoint = (fields: { distance: number; terrain: number | null; planned: number; collision?: boolean }): ProfilePoint => ({ collision: false, ...fields });

export type ProfileMarker = { sequence: number; distance: number; label: string; endDistance: number | null; lastSequence: number | null; pattern: string };

export const ProfileMarker = (fields: { sequence: number; distance: number; label: string; endDistance?: number | null; lastSequence?: number | null; pattern?: string }): ProfileMarker => ({
  endDistance: null,
  lastSequence: null,
  pattern: '',
  ...fields,
});

export type TerrainProfile = {
  readonly points: readonly ProfilePoint[];
  readonly clearance: Clearance | null;
  readonly lowestText: string;
  readonly highestText: string;
  readonly distanceText: string;
  readonly bandText: string;
  readonly markers: readonly ProfileMarker[];
  readonly band: readonly [number, number] | null;
  readonly heightHeader: string;
  readonly distanceTicks: readonly string[];
  readonly heightTicks: readonly string[];
  readonly distance: number;
  readonly lowest: number;
  readonly highest: number;
  readonly hasTerrain: boolean;
  readonly terrainCoverage: number;
  readonly span: number;
  readonly flat: boolean;
  readonly drawable: boolean;
};

const plannedExtreme = (points: readonly ProfilePoint[], pick: (a: number, b: number) => number): number | null =>
  points
    .filter((it) => Number.isFinite(it.planned))
    .map((it) => pick(it.terrain ?? it.planned, it.planned))
    .reduce<number | null>((best, value) => (best === null ? value : pick(best, value)), null);

const pairs = <T,>(list: readonly T[]): readonly (readonly [T, T])[] =>
  list.flatMap((from, index) => {
    const to = list[index + 1];
    return to === undefined ? [] : [[from, to] as const];
  });

export const TerrainProfile = (fields: {
  points: readonly ProfilePoint[];
  clearance?: Clearance | null;
  lowestText?: string;
  highestText?: string;
  distanceText?: string;
  bandText?: string;
  markers?: readonly ProfileMarker[];
  band?: readonly [number, number] | null;
  heightHeader?: string;
  distanceTicks?: readonly string[];
  heightTicks?: readonly string[];
}): TerrainProfile => {
  const points = fields.points;
  const band = fields.band ?? null;
  const distance = points.at(-1)?.distance ?? 0.0;
  const lowest = band?.[0] ?? plannedExtreme(points, Math.min) ?? 0.0;
  const highest = band?.[1] ?? plannedExtreme(points, Math.max) ?? 0.0;
  const covered = pairs(points)
    .filter(([from, to]) => from.terrain !== null && to.terrain !== null)
    .reduce((sum, [from, to]) => sum + (to.distance - from.distance), 0);
  return {
    clearance: null,
    lowestText: '',
    highestText: '',
    distanceText: '',
    bandText: '',
    markers: [],
    heightHeader: '',
    distanceTicks: [],
    heightTicks: [],
    ...fields,
    band,
    distance,
    lowest,
    highest,
    hasTerrain: points.filter((it) => it.terrain !== null).length >= 2,
    terrainCoverage: distance > 0.0 ? covered / distance : 0.0,
    span: coerceAtLeast(highest - lowest, MIN_SPAN_METRES),
    flat: highest - lowest < MIN_SPAN_METRES,
    drawable: points.length >= 2,
  };
};

export const metresBetween = (from: TrackPoint, to: TrackPoint): number => {
  const radians = Math.PI / 180;
  const fromLat = from.latitude * radians;
  const toLat = to.latitude * radians;
  const deltaLat = toLat - fromLat;
  const deltaLon = (to.longitude - from.longitude) * radians;
  const a = Math.sin(deltaLat / 2) ** 2 + Math.cos(fromLat) * Math.cos(toLat) * Math.sin(deltaLon / 2) ** 2;
  return 2 * EARTH_RADIUS_METRES * Math.asin(Math.min(1.0, Math.sqrt(a)));
};

export const TERRAIN_VIEW = 'view.terrainProfile';

export const clearanceOf = (view: Json | null): Clearance | null => {
  if (view === null) return null;
  const metres = optDouble(view, 'minClearanceMetres', NaN);
  return {
    collides: optBoolean(view, 'hasCollision'),
    metres: Number.isNaN(metres) ? null : metres,
    text: isNull(view, 'clearanceText') ? '' : optText(view, 'clearanceText'),
    complete: optBoolean(view, 'clearanceComplete'),
  };
};

export const terrainWarning = (clearance: Clearance | null): string | null => {
  if (clearance === null) return null;
  const metres = clearance.metres;
  if (!clearance.collides && !(metres !== null && metres < 0.0)) return null;
  return clearance.text.trim() !== '' ? `The route goes ${clearance.text} below the ground.` : 'The route goes below the ground.';
};

const finite = (value: number): number | null => (Number.isFinite(value) ? value : null);

export const terrainProfile = (view: Json | null): TerrainProfile => {
  const points = optJSONArray(view, 'points');
  if (view === null || points === null) return TerrainProfile({ points: [] });
  const low = finite(optDouble(view, 'minAltitudeMeters', NaN));
  const high = finite(optDouble(view, 'maxAltitudeMeters', NaN));
  return TerrainProfile({
    points: jsonObjects(points).map((point) => {
      const terrain = optDouble(point, 'terrainAltitude', NaN);
      return ProfilePoint({ distance: optDouble(point, 'distance', 0.0), terrain: Number.isNaN(terrain) ? null : terrain, planned: optDouble(point, 'missionAltitude', NaN), collision: optBoolean(point, 'collision') });
    }),
    markers: jsonObjects(optJSONArray(view, 'markers')).map((marker) => {
      const complex = optJSONObject(marker, 'complex');
      return ProfileMarker({
        sequence: optInt(marker, 'sequence'),
        distance: optDouble(marker, 'distance', 0.0),
        label: optText(marker, 'label'),
        endDistance: complex === null ? null : optDouble(complex, 'endDistance'),
        lastSequence: complex === null ? null : optInt(complex, 'lastSequence'),
        pattern: optText(complex, 'pattern'),
      });
    }),
    clearance: clearanceOf(view),
    lowestText: optText(view, 'lowestText'),
    highestText: optText(view, 'highestText'),
    distanceText: optText(view, 'distanceText'),
    bandText: optText(view, 'bandText'),
    band: low !== null && high !== null ? [low, high] : null,
    heightHeader: optText(view, 'heightHeader'),
    distanceTicks: jsonStrings(optJSONArray(view, 'distanceTicks')),
    heightTicks: jsonStrings(optJSONArray(view, 'heightTicks')),
  });
};

export const collidingItems = (view: Json | null, key = 'collidingItems'): ReadonlySet<number> =>
  new Set((optJSONArray(view, key) ?? []).map((value) => optInt({ value }, 'value', -1)).filter((it) => it >= 0));

const spot = (json: Json | null): TrackPoint | null => {
  if (json === null) return null;
  const at = { latitude: optDouble(json, 'latitude', NaN), longitude: optDouble(json, 'longitude', NaN) };
  return isPlottable(at.latitude, at.longitude) ? at : null;
};

export const collisionLegs = (view: Json | null): readonly (readonly [TrackPoint, TrackPoint])[] =>
  (optJSONArray(view, 'collisionLegs') ?? []).flatMap((leg) => {
    const from = spot(isJson(leg) ? optJSONObject(leg, 'from') : null);
    const to = spot(isJson(leg) ? optJSONObject(leg, 'to') : null);
    return from === null || to === null ? [] : [[from, to] as const];
  });
