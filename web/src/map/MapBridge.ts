import { useEffect, useMemo, useSyncExternalStore } from 'react';
import { call, onCoreEvent } from '@/core/client';
import { type Json, optBoolean, optDouble, optJSONArray, optString, parseJson } from '@/bridge/Json';
import { mutableStateOf } from '@/kotlin/State';
import { type TrackPoint, isPlottable } from './VehicleMap';

const CLIENT = 'map';

const registry = { watched: new Map<string, number>() as ReadonlyMap<string, number>, started: false };

const values = mutableStateOf<ReadonlyMap<string, Json>>(new Map());

const bridgeReady = mutableStateOf(false);

const withValue = (held: ReadonlyMap<string, Json>, path: string, json: Json): ReadonlyMap<string, Json> => new Map(held).set(path, json);

const withoutValue = (held: ReadonlyMap<string, Json>, path: string): ReadonlyMap<string, Json> => new Map([...held].filter(([key]) => key !== path));

const resend = (): boolean => {
  try {
    MapBridge.sendWatch([...registry.watched.keys()].join(','));
    bridgeReady.value = true;
    return true;
  } catch {
    return false;
  }
};

export const MapBridge = {
  values,

  bridgeReady,

  sendWatch: (csv: string): void => {
    void call('watchClient', CLIENT, csv);
  },

  start(): void {
    try {
      if (!registry.started) {
        onCoreEvent((path, json) => {
          if (registry.watched.has(path)) values.value = withValue(values.value, path, parseJson(json) ?? {});
        });
        registry.started = true;
      }
      bridgeReady.value = true;
    } catch {
      return;
    }
  },

  watch(path: string): void {
    const held = registry.watched.get(path) ?? 0;
    registry.watched = new Map(registry.watched).set(path, held + 1);
    if (held === 0 && !resend()) {
      registry.watched = new Map([...registry.watched].filter(([key]) => key !== path));
      bridgeReady.value = false;
    }
  },

  unwatch(path: string): void {
    const held = registry.watched.get(path);
    if (held === undefined) return;
    if (held === 1) {
      registry.watched = new Map([...registry.watched].filter(([key]) => key !== path));
      resend();
      values.value = withoutValue(values.value, path);
      return;
    }
    registry.watched = new Map(registry.watched).set(path, held - 1);
  },

  watchedPathsForTest(): ReadonlySet<string> {
    return new Set(registry.watched.keys());
  },

  forgetWatchesForTest(): void {
    registry.watched = new Map();
  },

  async seed(path: string): Promise<void> {
    if (values.value.has(path)) return;
    const json = await MapBridge.read(path);
    if (json === null || values.value.has(path)) return;
    values.value = withValue(values.value, path, json);
  },

  async read(path: string): Promise<Json | null> {
    try {
      return parseJson(await call<string>('get', path));
    } catch {
      return null;
    }
  },

  markReachable(): void {
    bridgeReady.value = true;
  },
};

export const useMapPath = (path: string): Json | null => {
  useEffect(() => {
    MapBridge.watch(path);
    return () => MapBridge.unwatch(path);
  }, [path]);
  useEffect(() => {
    void MapBridge.seed(path);
  }, [path]);
  return useSyncExternalStore(values.subscribe, () => values.value.get(path) ?? null);
};

const toDoubleOrNull = (text: string): number | null => {
  const parsed = Number(text);
  return text.trim() === '' || Number.isNaN(parsed) ? null : parsed;
};

const toIntOrNull = (text: string): number | null => (/^[+-]?\d+$/.test(text) ? Number.parseInt(text, 10) : null);

export const useMapDouble = (path: string): number => {
  const value = useMapPath(path)?.value;
  return typeof value === 'number' ? value : typeof value === 'string' ? (toDoubleOrNull(value) ?? NaN) : NaN;
};

export const useMapString = (path: string): string => optString(useMapPath(path), 'value', '');

export const useMapInt = (path: string, fallback = -1): number => {
  const value = useMapPath(path)?.value;
  return typeof value === 'number' ? Math.trunc(value) : typeof value === 'string' ? (toIntOrNull(value) ?? fallback) : fallback;
};

export const useMapCount = (path: string): number => optJSONArray(useMapPath(path), 'elements')?.length ?? 0;

export const coordinateOf = (json: Json | null): TrackPoint | null => {
  if (json === null || !optBoolean(json, 'valid')) return null;
  const latitude = optDouble(json, 'latitude', NaN);
  const longitude = optDouble(json, 'longitude', NaN);
  return isPlottable(latitude, longitude) ? { latitude, longitude } : null;
};

export const FLY_STATE_VIEW = 'view.flyState';

export const useMapCoordinate = (path: string): TrackPoint | null => {
  const json = useMapPath(path);
  return useMemo(() => coordinateOf(json), [json]);
};

export const useMapViewFlag = (path: string, field: string): boolean => optBoolean(useMapPath(path), field);

export const useMapBool = (path: string): boolean => {
  const value = useMapPath(path)?.value;
  return value === true || value === 'true';
};
