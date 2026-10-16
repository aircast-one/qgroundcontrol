import { type Json, optDouble } from '@/bridge/Json';
import { type TrackPoint, isPlottable } from './VehicleMap';
import type { EditableShape } from './EditableShape';
import { PLAN_ROOT } from './PlanBridge';

export const FENCE_ROOT = `${PLAN_ROOT}.geoFenceController`;
export const FENCE_POLYGONS = `${FENCE_ROOT}.polygons`;

const EARTH_RADIUS_M = 6_371_000.0;
const CIRCLE_SEGMENTS = 48;

export type FencePolygon = {
  index: number;
  inclusion: boolean;
  vertices: readonly TrackPoint[];
  editable: EditableShape | null;
  kindText: string;
  detailText: string;
};

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

export type RallyPoint = {
  index: number;
  latitude: number;
  longitude: number;
  altitudeMetres: number;
  altitude: number;
  altitudeUnits: string;
  altitudePath: string;
};

export type FirmwareFence = { radiusMetres: number; radiusText: string; centre: TrackPoint | null };

export type BreachReturn = { point: TrackPoint; altitude: number | null; units: string; altitudePath: string };

export const coordinate = (json: Json | null): TrackPoint | null => {
  if (json === null) return null;
  const latitude = optDouble(json, 'latitude', NaN);
  const longitude = optDouble(json, 'longitude', NaN);
  return isPlottable(latitude, longitude) ? { latitude, longitude } : null;
};

export const pointAt = (centre: TrackPoint, metres: number, bearingDegrees: number): TrackPoint => {
  const radians = Math.PI / 180;
  const angular = metres / EARTH_RADIUS_M;
  const lat = centre.latitude * radians;
  const lon = centre.longitude * radians;
  const bearing = bearingDegrees * radians;
  const pointLat = Math.asin(Math.sin(lat) * Math.cos(angular) + Math.cos(lat) * Math.sin(angular) * Math.cos(bearing));
  const pointLon = lon + Math.atan2(Math.sin(bearing) * Math.sin(angular) * Math.cos(lat), Math.cos(angular) - Math.sin(lat) * Math.sin(pointLat));
  return { latitude: pointLat / radians, longitude: pointLon / radians };
};

export const circleRing = (centre: TrackPoint, radiusMetres: number, segments: number = CIRCLE_SEGMENTS): readonly TrackPoint[] =>
  radiusMetres <= 0.0 || segments < 3 ? [] : Array.from({ length: segments }, (_, step) => pointAt(centre, radiusMetres, (360.0 * step) / segments));

export const circlesAsPolygons = (circles: readonly FenceCircle[]): readonly FencePolygon[] =>
  circles.flatMap((circle) => {
    const ring = circleRing(circle.centre, circle.radiusMetres);
    return ring.length < 3 ? [] : [{ index: circle.index, inclusion: circle.inclusion, vertices: ring, editable: null, kindText: circle.kindText, detailText: '' }];
  });

export const fencePolygons = (_json: Json | null): readonly FencePolygon[] => [];

export const fenceCircles = (_json: Json | null): readonly FenceCircle[] => [];

export const rallyPoints = (_json: Json | null): readonly RallyPoint[] => [];

export const firmwareFence = (_json: Json | null): FirmwareFence | null => null;

export const breachReturn = (_json: Json | null): BreachReturn | null => null;

export const FenceBridge = {
  read: async (): Promise<Json | null> => null,
  readFlown: async (): Promise<Json | null> => null,
};
