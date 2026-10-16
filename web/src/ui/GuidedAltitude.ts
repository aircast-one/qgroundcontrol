import { type Json, isNull, optBoolean, optDouble, optText } from '@/bridge/Json';
import { format } from '@/kotlin/Std';

export const GUIDED_ALTITUDE = 'view.guidedAltitude';

export type GuidedAltitude = {
  readonly available: boolean;
  readonly label: string;
  readonly unit: string;
  readonly current: number | null;
  readonly minimum: number | null;
  readonly maximum: number | null;
  readonly sentence: string;
  readonly deltaMeters: number;
  readonly sends: boolean;
  readonly targetMeters: number | null;
  readonly currentMeters: number | null;
};

export const GuidedAltitude = (fields: Omit<GuidedAltitude, 'targetMeters' | 'currentMeters'> & { targetMeters?: number | null; currentMeters?: number | null }): GuidedAltitude => ({
  targetMeters: null,
  currentMeters: null,
  ...fields,
});

export const guidedAltitudePath = (target: number, pause = false): string =>
  `${GUIDED_ALTITUDE}(${format('%.2f', target)}${pause ? ',pause' : ''})`;

const numberOrNull = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

export const guidedAltitude = (view: Json | null): GuidedAltitude | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  return GuidedAltitude({
    available: true,
    label: optText(view, 'label'),
    unit: optText(view, 'unit'),
    current: numberOrNull(view, 'current'),
    minimum: numberOrNull(view, 'minimum'),
    maximum: numberOrNull(view, 'maximum'),
    sentence: isNull(view, 'sentence') ? '' : optText(view, 'sentence'),
    deltaMeters: optDouble(view, 'deltaMeters', 0.0),
    sends: optBoolean(view, 'sends'),
    targetMeters: numberOrNull(view, 'targetMeters'),
    currentMeters: numberOrNull(view, 'currentMeters'),
  });
};

export const altitudeDelta = (reading: GuidedAltitude): number | null => (reading.sends ? reading.deltaMeters : null);

export const rangeLabel = (minimum: number | null, maximum: number | null, unit: string): string | null => {
  if (minimum === null || maximum === null || maximum <= minimum) return null;
  const show = (value: number) => {
    const text = format('%.1f', value);
    return text === '-0.0' ? '0.0' : text;
  };
  return [`${show(minimum)} to ${show(maximum)}`, unit].filter((it) => it.trim() !== '').join(' ');
};
