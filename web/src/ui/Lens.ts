import { type Json, isNull, jsonObjects, optBoolean, optDouble, optJSONArray, optText } from '@/bridge/Json';

export const LENS_VIEW = 'view.lens';
export const LENS_MEASURE = 'camera.measureLens';
export const LENS_FORGET = 'camera.forgetLens';
export const APPROXIMATE_AIM = 'Approximate aim · measure the lens in camera details';

export type LensReading = {
  summary: string;
  approximate: boolean;
  measuring: boolean;
  progress: string;
  error: string | null;
  canMeasure: boolean;
  reason: string | null;
  measuredZooms: readonly number[];
};

const text = (view: Json, key: string): string | null => {
  const value = isNull(view, key) ? '' : optText(view, key);
  return value.trim() === '' ? null : value;
};

export const lensReading = (view: Json | null): LensReading | null =>
  view === null || optText(view, 'class') !== 'Lens'
    ? null
    : {
        summary: optText(view, 'summary'),
        approximate: optBoolean(view, 'approximate'),
        measuring: optBoolean(view, 'measuring'),
        progress: optText(view, 'progress'),
        error: text(view, 'error'),
        canMeasure: optBoolean(view, 'canMeasure'),
        reason: text(view, 'reason'),
        measuredZooms: jsonObjects(optJSONArray(view, 'samples')).map((sample) => optDouble(sample, 'zoom', 0)),
      };

export const measuredText = (zooms: readonly number[]): string | null =>
  zooms.length === 0 ? null : `Measured at ${zooms.toSorted((a, b) => a - b).map((zoom) => `${Math.trunc(zoom)}%`).join(', ')} zoom`;
