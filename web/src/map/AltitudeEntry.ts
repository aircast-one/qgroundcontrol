import { format } from '@/kotlin/Std';

export const RALLY_ALTITUDE_DECIMALS = 2;
export const SURFACE_DISTANCE_DECIMALS = 2;
export const SURFACE_DISTANCE_MINIMUM = 0.1;

const DOUBLE_LITERAL = /^[+-]?(NaN|Infinity|(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?[fFdD]?)$/;

export const toDoubleOrNull = (text: string): number | null => {
  const literal = text.replace(/^[\x00-\x20]+|[\x00-\x20]+$/g, '');
  return DOUBLE_LITERAL.test(literal) ? Number(literal.replace(/[fFdD]$/, '')) : null;
};

export const typedNumber = (text: string): number | null => {
  const typed = text.trim();
  const commas = [...typed].filter((char) => char === ',').length;
  if (commas > 1 || (typed.includes(',') && typed.includes('.'))) return null;
  const number = toDoubleOrNull(typed.replace(',', '.'));
  return number !== null && Number.isFinite(number) ? number : null;
};

export const parsedAltitude = (text: string): number | null => typedNumber(text);

export const parsedSurfaceDistance = (text: string, metresPerShown: number): number | null => {
  const typed = typedNumber(text);
  return typed !== null && typed * metresPerShown >= SURFACE_DISTANCE_MINIMUM - 1e-9 ? typed : null;
};

export const metresPerUnit = (units: string): number => (units.trim() === 'ft' ? 0.3048 : 1.0);

export const altitudeFieldText = (altitude: number, decimals: number): string => {
  if (Number.isNaN(altitude)) return '';
  const shown = format(`%.${decimals}f`, altitude);
  const trimmed = shown.includes('.') ? shown.replace(/0+$/, '').replace(/\.$/, '') : shown;
  return trimmed === '-0' ? '0' : trimmed;
};

export const LATITUDE_LIMIT = 90.0;
export const LONGITUDE_LIMIT = 180.0;

export const parsedCoordinate = (text: string, limit: number): number | null => {
  const typed = typedNumber(text);
  return typed !== null && typed >= -limit && typed <= limit ? typed : null;
};
