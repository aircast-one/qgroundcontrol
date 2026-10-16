import { type Json, isNull, optBoolean, optDouble, optJSONArray, optJSONObject, optText } from '@/bridge/Json';

export type ObstacleWarning = { label: string; close: boolean };

export const sensorSaidStale = (view: Json): boolean => !isNull(view, 'stale') && optBoolean(view, 'stale');

export const obstacleWarning = (view: Json | null): ObstacleWarning | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const nearest = optJSONObject(view, 'nearest');
  if (nearest === null || sensorSaidStale(nearest)) return null;
  const distance = optText(nearest, 'distanceText');
  if (distance.trim() === '') return null;
  const sector = optText(nearest, 'sectorText');
  return {
    label: [distance, sector].filter((it) => it.trim() !== '').join(' '),
    close: optBoolean(nearest, 'close'),
  };
};

export type ObstacleSample = { bearingDegrees: number; metres: number };

export type ObstacleRing = {
  samples: readonly ObstacleSample[];
  maxMetres: number;
  incrementDegrees: number;
  floorMetres: number;
  stale: boolean;
};

const positive = (value: number): number | null => (Number.isFinite(value) && value > 0 ? value : null);

export const obstacleRing = (view: Json | null): ObstacleRing | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const ring = optJSONArray(view, 'ringMetres');
  if (ring === null) return null;
  const increment = positive(optDouble(view, 'ringIncrement', NaN));
  if (increment === null) return null;
  const ceiling = positive(optDouble(view, 'rangeMaxMetres', NaN));
  if (ceiling === null) return null;
  const served = optDouble(view, 'ringOffset', 0);
  const offset = Number.isFinite(served) ? served : 0;
  const samples = ring.flatMap((entry, index) => {
    if (entry === null || entry === undefined) return [];
    const metres = typeof entry === 'number' ? entry : typeof entry === 'string' && entry.trim() !== '' ? Number(entry) : NaN;
    return Number.isFinite(metres) && metres >= 0 ? [{ bearingDegrees: offset + index * increment, metres }] : [];
  });
  const floor = positive(optDouble(view, 'rangeMinMetres', NaN)) ?? 0;
  return samples.length === 0 ? null : { samples, maxMetres: ceiling, incrementDegrees: increment, floorMetres: floor, stale: sensorSaidStale(view) };
};
