import type { TrackPoint } from './VehicleMap';

const EARTH_RADIUS_METRES = 6_371_000.0;

export const TERRAIN_VIEW = 'view.terrainProfile';

export const metresBetween = (from: TrackPoint, to: TrackPoint): number => {
  const radians = Math.PI / 180;
  const fromLat = from.latitude * radians;
  const toLat = to.latitude * radians;
  const deltaLat = toLat - fromLat;
  const deltaLon = (to.longitude - from.longitude) * radians;
  const a = Math.sin(deltaLat / 2) ** 2 + Math.cos(fromLat) * Math.cos(toLat) * Math.sin(deltaLon / 2) ** 2;
  return 2 * EARTH_RADIUS_METRES * Math.asin(Math.min(1.0, Math.sqrt(a)));
};
