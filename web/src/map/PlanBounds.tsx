import { type TrackPoint, isPlottable } from './VehicleMap';
import type { MissionItem } from './PlanBridge';
import { type FenceCircle, type FencePolygon, type RallyPoint, circlesAsPolygons } from './FenceBridge';
import type { Survey } from './SurveyBridge';

export type PlanBounds = {
  south: number;
  west: number;
  north: number;
  east: number;
  centre: TrackPoint;
  longitudeSpan: number;
  unwrappedEast: number;
  spanDegrees: number;
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
  const gaps = sorted.map((here, index) => [(index === sorted.length - 1 ? sorted[0]! + 360.0 : sorted[index + 1]!) - here, index] as const);
  const widest = gaps.reduce<readonly [number, number] | null>((best, gap) => (best === null || gap[0] > best[0] ? gap : best), null);
  if (widest === null) return [0.0, 0.0];
  return [sorted[(widest[1] + 1) % sorted.length]!, sorted[widest[1]]!];
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

export const planBounds = (points: readonly TrackPoint[]): PlanBounds | null => {
  const usable = points.filter((it) => isPlottable(it.latitude, it.longitude));
  if (usable.length === 0) return null;
  const [west, east] = longitudeArc(usable.map((it) => it.longitude));
  return PlanBounds({
    south: Math.min(...usable.map((it) => it.latitude)),
    west,
    north: Math.max(...usable.map((it) => it.latitude)),
    east,
  });
};

export const clearWindow = (corners: readonly TrackPoint[]): readonly TrackPoint[] => {
  const north = Math.max(...corners.map((it) => it.latitude));
  const south = Math.min(...corners.map((it) => it.latitude));
  const west = Math.min(...corners.map((it) => it.longitude));
  const east = Math.max(...corners.map((it) => it.longitude));
  return [
    { latitude: north, longitude: west },
    { latitude: north, longitude: east },
    { latitude: south, longitude: east },
    { latitude: south, longitude: west },
  ];
};
