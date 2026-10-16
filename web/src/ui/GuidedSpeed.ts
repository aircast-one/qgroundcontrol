import { type Json, isNull, optBoolean, optDouble, optString, optText } from '@/bridge/Json';
import { format } from '@/kotlin/Std';

export const GUIDED_SPEED = 'view.guidedSpeed';

export type GuidedSpeed = {
  readonly label: string;
  readonly unit: string;
  readonly command: string | null;
  readonly initial: number | null;
  readonly minimum: number | null;
  readonly maximum: number | null;
  readonly sentence: string;
  readonly targetMetersSecond: number;
};

export const guidedSpeedPath = (target: number): string => `${GUIDED_SPEED}(${format('%.2f', target)})`;

const numberOrNull = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

const textOrNull = (json: Json, key: string): string | null => {
  if (isNull(json, key)) return null;
  const text = optString(json, key);
  return text.trim() === '' ? null : text;
};

export const guidedSpeed = (view: Json | null): GuidedSpeed | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  return {
    label: textOrNull(view, 'label') ?? 'Speed',
    unit: optText(view, 'unit'),
    command: textOrNull(view, 'command'),
    initial: numberOrNull(view, 'initial'),
    minimum: numberOrNull(view, 'minimum'),
    maximum: numberOrNull(view, 'maximum'),
    sentence: isNull(view, 'sentence') ? '' : optText(view, 'sentence'),
    targetMetersSecond: optDouble(view, 'targetMetersSecond', 0.0),
  };
};
