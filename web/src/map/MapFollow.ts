import { mutableStateOf } from '@/kotlin/State';
import { metresBetween } from './TerrainProfile';
import type { TrackPoint } from './VehicleMap';

export const mapFollowing = mutableStateOf(true);

export type ScreenPoint = { readonly x: number; readonly y: number };

export type ScreenRect = { readonly left: number; readonly top: number; readonly right: number; readonly bottom: number };

export type MapWindow = { readonly centre: TrackPoint; readonly shows: (at: TrackPoint) => boolean };

export const mapWindow = mutableStateOf<MapWindow | null>(null);

export const FOLLOW_VEHICLE = 'Follow the vehicle';

const COMPASS_POINTS: readonly string[] = ['north', 'north-east', 'east', 'south-east', 'south', 'south-west', 'west', 'north-west'];

export const bearingDegrees = (from: TrackPoint, to: TrackPoint): number => {
  const radians = Math.PI / 180;
  const [fromLat, toLat, deltaLon] = [from.latitude * radians, to.latitude * radians, (to.longitude - from.longitude) * radians];
  const y = Math.sin(deltaLon) * Math.cos(toLat);
  const x = Math.cos(fromLat) * Math.sin(toLat) - Math.sin(fromLat) * Math.cos(toLat) * Math.cos(deltaLon);
  return ((Math.atan2(y, x) / radians) + 360) % 360;
};

export const nearestLongitude = (centre: number, longitude: number): number => centre + ((((longitude - centre) % 360) + 540) % 360) - 180;

export const inWindow = (project: (at: TrackPoint) => ScreenPoint, rect: ScreenRect, centre: TrackPoint, at: TrackPoint): boolean => {
  const { x, y } = project({ latitude: at.latitude, longitude: nearestLongitude(centre.longitude, at.longitude) });
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
};

export const projectedWindow = (project: (at: TrackPoint) => ScreenPoint, rect: ScreenRect, centre: TrackPoint): MapWindow => ({
  centre,
  shows: (at) => inWindow(project, rect, centre, at),
});

export type VehicleOffScreen = { readonly bearing: number; readonly text: string };

export const vehicleOffScreen = (seen: MapWindow | null, vehicle: TrackPoint): VehicleOffScreen | null => {
  if (seen === null || seen.shows(vehicle)) return null;
  const bearing = bearingDegrees(seen.centre, vehicle);
  const metres = metresBetween(seen.centre, vehicle);
  const away = metres >= 1000 ? `${(metres / 1000).toFixed(1)} km` : `${Math.round(metres)} m`;
  return { bearing, text: `Vehicle ${away} ${COMPASS_POINTS[Math.round(bearing / 45) % COMPASS_POINTS.length] ?? ''} of the map center` };
};
