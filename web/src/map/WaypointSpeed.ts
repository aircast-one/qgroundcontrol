import { type Json, isNull, optBoolean, optDouble, optJSONObject, optText } from '@/bridge/Json';

export type WaypointSpeed = { specified: boolean; value: number | null; units: string; path: string; specifyPath: string };

export const waypointSpeed = (view: Json | null): WaypointSpeed | null => {
  const section = optJSONObject(view, 'speedSection');
  if (section === null || !optBoolean(section, 'available')) return null;
  return {
    specified: optBoolean(section, 'specified'),
    value: isNull(section, 'value') ? null : optDouble(section, 'value'),
    units: optText(section, 'units'),
    path: optText(section, 'path'),
    specifyPath: optText(section, 'specifyPath'),
  };
};

export type WaypointHold = { seconds: number; units: string; path: string };

export const waypointHold = (view: Json | null): WaypointHold | null => {
  const hold = optJSONObject(view, 'hold');
  if (hold === null) return null;
  const read = { seconds: optDouble(hold, 'value', 0.0), units: optText(hold, 'units'), path: optText(hold, 'path') };
  return read.path.trim() === '' ? null : read;
};
