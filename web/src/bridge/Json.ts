export type Json = { [key: string]: unknown };
export type JsonArray = unknown[];

export const isJson = (value: unknown): value is Json => typeof value === 'object' && value !== null && !Array.isArray(value);

export const parseJson = (text: string | null | undefined): Json | null => {
  if (!text) return null;
  try {
    const parsed: unknown = JSON.parse(text);
    return isJson(parsed) ? parsed : null;
  } catch {
    return null;
  }
};

export const has = (json: Json | null | undefined, key: string): boolean => json !== null && json !== undefined && key in json;

export const isNull = (json: Json | null | undefined, key: string): boolean => json?.[key] === null || json?.[key] === undefined;

export const opt = (json: Json | null | undefined, key: string): unknown => json?.[key] ?? null;

export const optText = (json: Json | null | undefined, key: string): string => {
  const value = json?.[key];
  if (value === null || value === undefined) return '';
  return typeof value === 'string' ? value : typeof value === 'object' ? JSON.stringify(value) : String(value);
};

export const optArrayText = (array: JsonArray | null | undefined, index: number): string => {
  const value = array?.[index];
  if (value === null || value === undefined) return '';
  return typeof value === 'string' ? value : typeof value === 'object' ? JSON.stringify(value) : String(value);
};

export const optString = (json: Json | null | undefined, key: string, fallback = ''): string => {
  const value = json?.[key];
  if (value === undefined) return fallback;
  return value === null ? 'null' : typeof value === 'string' ? value : typeof value === 'object' ? JSON.stringify(value) : String(value);
};

export const optBoolean = (json: Json | null | undefined, key: string, fallback = false): boolean => {
  const value = json?.[key];
  if (typeof value === 'boolean') return value;
  if (typeof value === 'string' && value.toLowerCase() === 'true') return true;
  if (typeof value === 'string' && value.toLowerCase() === 'false') return false;
  return fallback;
};

const numeric = (value: unknown): number => {
  if (typeof value === 'number') return value;
  if (typeof value === 'string' && value.trim() !== '') return Number(value);
  return NaN;
};

export const optDouble = (json: Json | null | undefined, key: string, fallback = NaN): number => {
  const value = numeric(json?.[key]);
  return Number.isNaN(value) ? fallback : value;
};

export const optInt = (json: Json | null | undefined, key: string, fallback = 0): number => {
  const value = numeric(json?.[key]);
  return Number.isFinite(value) ? Math.trunc(value) : fallback;
};

export const optLong = optInt;

export const optJSONObject = (json: Json | null | undefined, key: string): Json | null => {
  const value = json?.[key];
  return isJson(value) ? value : null;
};

export const optJSONArray = (json: Json | null | undefined, key: string): JsonArray | null => {
  const value = json?.[key];
  return Array.isArray(value) ? value : null;
};

export const arrayJSONObject = (array: JsonArray | null | undefined, index: number): Json | null => {
  const value = array?.[index];
  return isJson(value) ? value : null;
};

export const jsonObjects = (array: JsonArray | null | undefined): Json[] => (array ?? []).filter(isJson);

export const jsonStrings = (array: JsonArray | null | undefined): string[] => (array ?? []).map((_, index) => optArrayText(array, index));
