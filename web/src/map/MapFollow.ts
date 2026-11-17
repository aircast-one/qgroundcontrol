import { mutableStateOf } from '@/kotlin/State';
import { metresBetween } from './TerrainProfile';
import type { TrackPoint } from './VehicleMap';

export const mapFollowing = mutableStateOf(true);

export const mapWindow = mutableStateOf<readonly TrackPoint[]>([]);

export const FOLLOW_VEHICLE = 'Follow the vehicle';

const COMPASS_POINTS: readonly string[] = ['north', 'north-east', 'east', 'south-east', 'south', 'south-west', 'west', 'north-west'];

export const bearingDegrees = (from: TrackPoint, to: TrackPoint): number => {
  const radians = Math.PI / 180;
  const [fromLat, toLat, deltaLon] = [from.latitude * radians, to.latitude * radians, (to.longitude - from.longitude) * radians];
  const y = Math.sin(deltaLon) * Math.cos(toLat);
  const x = Math.cos(fromLat) * Math.sin(toLat) - Math.sin(fromLat) * Math.cos(toLat) * Math.cos(deltaLon);
  return ((Math.atan2(y, x) / radians) + 360) % 360;
};

export const inWindow = (corners: readonly TrackPoint[], at: TrackPoint): boolean => {
  if (corners.length === 0) return true;
  const latitudes = corners.map((it) => it.latitude);
  const longitudes = corners.map((it) => it.longitude);
  return at.latitude <= Math.max(...latitudes) && at.latitude >= Math.min(...latitudes) && at.longitude <= Math.max(...longitudes) && at.longitude >= Math.min(...longitudes);
};

export type VehicleOffScreen = { readonly bearing: number; readonly text: string };

export const vehicleOffScreen = (corners: readonly TrackPoint[], vehicle: TrackPoint): VehicleOffScreen | null => {
  if (inWindow(corners, vehicle)) return null;
  const centre = { latitude: corners.reduce((sum, it) => sum + it.latitude, 0) / corners.length, longitude: corners.reduce((sum, it) => sum + it.longitude, 0) / corners.length };
  const bearing = bearingDegrees(centre, vehicle);
  const metres = metresBetween(centre, vehicle);
  const away = metres >= 1000 ? `${(metres / 1000).toFixed(1)} km` : `${Math.round(metres)} m`;
  return { bearing, text: `Vehicle ${away} ${COMPASS_POINTS[Math.round(bearing / 45) % COMPASS_POINTS.length] ?? ''} of the map center` };
};
