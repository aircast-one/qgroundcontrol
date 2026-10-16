import { useEffect, useMemo, useRef, useSyncExternalStore } from 'react';
import { type Fact, Qgc } from './Qgc';
import { type Json, optArrayText } from './Json';

export const settingControl = (factPath: string): string => `view.control(${factPath})`;

export const useQgcPath = (path: string): Json | null => {
  useEffect(() => {
    Qgc.watch([path]);
    return () => Qgc.unwatch([path]);
  }, [path]);
  return useSyncExternalStore(
    (listener) => Qgc.subscribe(path, listener),
    () => Qgc.path(path),
  );
};

export const useQgcDerived = <T,>(path: string, read: (json: Json | null) => T): T => {
  const json = useQgcPath(path);
  return useMemo(() => read(json), [json, read]);
};

export const useQgcPathHoldingLast = (path: string): Json | null => {
  const current = useQgcPath(path);
  const last = useRef<Json | null>(null);
  const held = current ?? last.current;
  last.current = held;
  return held;
};

export const useQgcValue = (path: string): unknown => {
  const json = useQgcPath(path);
  return json?.value ?? null;
};

export const useQgcString = (path: string, fallback = ''): string => {
  const value = useQgcValue(path);
  return value === null || value === undefined ? fallback : typeof value === 'object' ? JSON.stringify(value) : String(value);
};

export const truthy = (value: unknown): boolean => {
  if (typeof value === 'boolean') return value;
  if (typeof value === 'number') return value !== 0;
  if (typeof value === 'string') {
    if (value.toLowerCase() === 'true') return true;
    const parsed = Number(value);
    return value.trim() !== '' && !Number.isNaN(parsed) && parsed !== 0;
  }
  return false;
};

export const useQgcBool = (path: string): boolean => truthy(useQgcValue(path));

export const useQgcDouble = (path: string, fallback = NaN): number => {
  const value = useQgcValue(path);
  if (typeof value === 'number') return value;
  if (typeof value === 'boolean') return value ? 1 : 0;
  if (typeof value === 'string') {
    const parsed = Number(value);
    return value.trim() === '' || Number.isNaN(parsed) ? fallback : parsed;
  }
  return fallback;
};

export const useQgcFacts = (groupPath: string): Fact[] => {
  const json = useQgcPath(groupPath);
  return useMemo(() => Qgc.facts(groupPath, json), [groupPath, json]);
};

export const useQgcStrings = (path: string): string[] => {
  const value = useQgcValue(path);
  return useMemo(() => (Array.isArray(value) ? value.map((_, index) => optArrayText(value, index)) : []), [value]);
};
