import { type Json, optBoolean, optDouble, optText } from '@/bridge/Json';

const EQUATOR_METRES = 40_075_016.686;
const TILE_PIXELS = 512.0;

export type MapScale = { readonly text: string; readonly pixels: number };

export const metresPerPixel = (latitude: number, zoom: number): number => (EQUATOR_METRES * Math.cos((latitude * Math.PI) / 180)) / (TILE_PIXELS * 2.0 ** zoom);

export const metresAcross = (latitude: number, zoom: number, maxPixels: number): number | null => {
  const perPixel = metresPerPixel(latitude, zoom);
  if (!Number.isFinite(perPixel) || perPixel <= 0.0 || maxPixels <= 0.0) return null;
  return perPixel * maxPixels;
};

export const mapScaleBar = (view: Json | null, maxPixels: number): MapScale | null => {
  if (!optBoolean(view, 'available')) return null;
  const fraction = optDouble(view, 'fraction', NaN);
  if (!Number.isFinite(fraction) || fraction <= 0.0) return null;
  const text = optText(view, 'text');
  if (text.trim() === '') return null;
  return { text, pixels: fraction * maxPixels };
};
