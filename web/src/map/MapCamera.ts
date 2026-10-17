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
