import { type FenceCircle, type FencePolygon, type RallyPoint, circlesAsPolygons } from './FenceBridge';
import type { MissionItem } from './PlanBridge';
import type { Survey } from './SurveyBridge';
import { type TrackPoint, isPlottable } from './VehicleMap';

export type PlanBounds = {
  readonly south: number;
  readonly west: number;
  readonly north: number;
  readonly east: number;
  readonly centre: TrackPoint;
  readonly longitudeSpan: number;
  readonly unwrappedEast: number;
  readonly spanDegrees: number;
};

export const normaliseLongitude = (degrees: number): number => {
  const wrapped = ((((degrees + 180.0) % 360.0) + 360.0) % 360.0) - 180.0;
  return wrapped === -180.0 ? 180.0 : wrapped;
};

export const PlanBounds = (fields: { south: number; west: number; north: number; east: number }): PlanBounds => {
  const longitudeSpan = fields.east >= fields.west ? fields.east - fields.west : fields.east - fields.west + 360.0;
  return {
    ...fields,
    longitudeSpan,
    centre: { latitude: (fields.south + fields.north) / 2, longitude: normaliseLongitude(fields.west + longitudeSpan / 2) },
    unwrappedEast: fields.west + longitudeSpan,
    spanDegrees: Math.max(fields.north - fields.south, longitudeSpan),
  };
};

export const longitudeArc = (longitudes: readonly number[]): readonly [number, number] => {
  const sorted = longitudes.map(normaliseLongitude).toSorted((a, b) => a - b);
  const widest = sorted
    .map((here, index) => [(index === sorted.length - 1 ? (sorted[0] ?? 0) + 360.0 : (sorted[index + 1] ?? 0)) - here, index] as const)
    .reduce<readonly [number, number] | null>((best, gap) => (best === null || gap[0] > best[0] ? gap : best), null);
  if (widest === null) return [0.0, 0.0];
  return [sorted[(widest[1] + 1) % sorted.length] ?? 0, sorted[widest[1]] ?? 0];
};

export const planPoints = (
  items: readonly MissionItem[] = [],
  polygons: readonly FencePolygon[] = [],
  circles: readonly FenceCircle[] = [],
  rally: readonly RallyPoint[] = [],
  surveys: readonly Survey[] = [],
): readonly TrackPoint[] => [
  ...items.map((it) => ({ latitude: it.latitude, longitude: it.longitude })),
  ...polygons.flatMap((it) => it.vertices),
  ...circlesAsPolygons(circles).flatMap((it) => it.vertices),
  ...rally.map((it) => ({ latitude: it.latitude, longitude: it.longitude })),
  ...surveys.flatMap((it) => [...it.area, ...it.transects]),
];

export const fitPoints = (plan: readonly TrackPoint[], latitude: number, longitude: number): readonly TrackPoint[] =>
  plan.length > 0 ? plan : isPlottable(latitude, longitude) ? [{ latitude, longitude }] : [];

const least = (values: readonly number[]): number => values.reduce((a, b) => Math.min(a, b), Infinity);

const most = (values: readonly number[]): number => values.reduce((a, b) => Math.max(a, b), -Infinity);

export const planBounds = (points: readonly TrackPoint[]): PlanBounds | null => {
  const usable = points.filter((it) => isPlottable(it.latitude, it.longitude));
  if (usable.length === 0) return null;
  const [west, east] = longitudeArc(usable.map((it) => it.longitude));
  return PlanBounds({
    south: least(usable.map((it) => it.latitude)),
    west,
    north: most(usable.map((it) => it.latitude)),
    east,
  });
};

export const planIsDrawn = (
  items: readonly MissionItem[],
  surveys: readonly Survey[],
  polygons: readonly FencePolygon[],
  circles: readonly FenceCircle[],
  rally: readonly RallyPoint[],
): boolean => [items, surveys, polygons, circles, rally].some((it) => it.length > 0);

export const fitsPlanOnEntry = (firstRead: boolean, planRead: boolean, planIsDrawn: boolean): boolean => firstRead && planRead && planIsDrawn;

export const centersOnVehicleAtEntry = (alreadyCentred: boolean, vehicleKnown: boolean, fitsRequested: number): boolean => !alreadyCentred && vehicleKnown && fitsRequested === 0;

export const planEntryPoint = (vehicle: TrackPoint | null, operator: TrackPoint | null, lastViewed: TrackPoint | null): TrackPoint | null => vehicle ?? operator ?? lastViewed;

export const stillFirstRead = (firstRead: boolean, planRead: boolean): boolean => firstRead && !planRead;

export const clearWindow = (corners: readonly TrackPoint[]): readonly TrackPoint[] => {
  const north = most(corners.map((it) => it.latitude));
  const south = least(corners.map((it) => it.latitude));
  const west = least(corners.map((it) => it.longitude));
  const east = most(corners.map((it) => it.longitude));
  return [
    { latitude: north, longitude: west },
    { latitude: north, longitude: east },
    { latitude: south, longitude: east },
    { latitude: south, longitude: west },
  ];
};
