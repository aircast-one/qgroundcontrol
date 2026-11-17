import { getSharedPreferences } from '@/kotlin/Platform';
import { type TrackPoint, isPlottable } from './VehicleMap';

const CAMERA_STORE = 'fly-map-camera';

export type SavedCamera = { readonly centre: TrackPoint; readonly zoom: number };

const toDoubleOrNull = (text: string): number | null => {
  const parsed = Number(text);
  return text.trim() === '' || Number.isNaN(parsed) ? null : parsed;
};

export const readCamera = (): SavedCamera | null => {
  const parts = (getSharedPreferences(CAMERA_STORE).getString('camera', null) ?? '').split(',').flatMap((it) => {
    const parsed = toDoubleOrNull(it);
    return parsed === null ? [] : [parsed];
  });
  const [latitude, longitude, zoom] = parts;
  if (parts.length !== 3 || latitude === undefined || longitude === undefined || zoom === undefined) return null;
  return isPlottable(latitude, longitude) && zoom > 1.0 ? { centre: { latitude, longitude }, zoom } : null;
};

export const writeCamera = (camera: SavedCamera): void => {
  getSharedPreferences(CAMERA_STORE).putString('camera', `${camera.centre.latitude},${camera.centre.longitude},${camera.zoom}`);
};

const VEHICLE_ZOOM = 17.0;
const VEHICLE_DECIMALS = 4;

const pointOf = (text: string | null): TrackPoint | null => {
  const [latitude, longitude] = (text ?? '').split(',').map((it) => toDoubleOrNull(it));
  return latitude === undefined || longitude === undefined || latitude === null || longitude === null || !isPlottable(latitude, longitude) ? null : { latitude, longitude };
};

export const readLastVehicle = (): TrackPoint | null => pointOf(getSharedPreferences(CAMERA_STORE).getString('vehicle', null));

export const vehicleSpot = (at: TrackPoint): string => `${at.latitude.toFixed(VEHICLE_DECIMALS)},${at.longitude.toFixed(VEHICLE_DECIMALS)}`;

export const writeLastVehicle = (at: TrackPoint): void => {
  const spot = vehicleSpot(at);
  const store = getSharedPreferences(CAMERA_STORE);
  if (store.getString('vehicle', null) !== spot) store.putString('vehicle', spot);
};

export const launchCamera = (saved: SavedCamera | null, lastVehicle: TrackPoint | null): SavedCamera | null =>
  saved ?? (lastVehicle === null ? null : { centre: lastVehicle, zoom: VEHICLE_ZOOM });
