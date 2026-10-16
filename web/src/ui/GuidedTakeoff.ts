import { type Json, isNull, optBoolean, optDouble, optText } from '@/bridge/Json';
import { format } from '@/kotlin/Std';

export const GUIDED_TAKEOFF = 'view.guidedTakeoff';

export type GuidedTakeoff = {
  readonly label: string;
  readonly unit: string;
  readonly initial: number | null;
  readonly minimum: number | null;
  readonly maximum: number | null;
  readonly sentence: string;
  readonly targetMeters: number;
};

export const guidedTakeoffPath = (target: number): string => `${GUIDED_TAKEOFF}(${format('%.2f', target)})`;

const numberOrNull = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

export const guidedTakeoff = (view: Json | null): GuidedTakeoff | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  return {
    label: optText(view, 'label'),
    unit: optText(view, 'unit'),
    initial: numberOrNull(view, 'initial'),
    minimum: numberOrNull(view, 'minimum'),
    maximum: numberOrNull(view, 'maximum'),
    sentence: isNull(view, 'sentence') ? '' : optText(view, 'sentence'),
    targetMeters: optDouble(view, 'targetMeters', 0.0),
  };
};

export const takeoffRangeUsable = (takeoff: GuidedTakeoff | null): boolean =>
  takeoff !== null && takeoff.minimum !== null && takeoff.maximum !== null && takeoff.maximum > takeoff.minimum;
