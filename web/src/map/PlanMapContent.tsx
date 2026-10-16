import { type Json, has, jsonStrings, optJSONArray, optText } from '@/bridge/Json';

export const MAP_TYPES_VIEW = 'view.mapTypes';

export type MapTypes = { current: string; types: readonly string[]; path: string };

export const mapTypes = (view: Json | null): MapTypes | null =>
  view !== null && has(view, 'types') ? { current: optText(view, 'current'), types: jsonStrings(optJSONArray(view, 'types')), path: optText(view, 'path') } : null;
