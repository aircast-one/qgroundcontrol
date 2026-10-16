import { type Json, arrayJSONObject, has, isNull, optBoolean, optDouble, optInt, optJSONArray } from '@/bridge/Json';
import { type TrackPoint, isPlottable } from './VehicleMap';

export const TRACK_VIEW = 'view.track';
export const TRACK_TAIL_VIEW = 'view.track(tail)';

export type TrackReading = {
  readonly points: readonly TrackPoint[];
  readonly count: number;
  readonly available: boolean;
  readonly vehicleId: number | null;
  readonly generation: number;
  readonly from: number;
};

export const TrackReading = (fields: { points: readonly TrackPoint[]; count: number; available: boolean; vehicleId?: number | null; generation?: number; from?: number }): TrackReading => ({
  vehicleId: null,
  generation: 0,
  from: 0,
  ...fields,
});

export const trackReading = (view: Json | null): TrackReading => {
  const listed = optJSONArray(view, 'points');
  return TrackReading({
    points: (listed ?? []).map((_, index) => {
      const point = arrayJSONObject(listed, index);
      return { latitude: optDouble(point, 'latitude', NaN), longitude: optDouble(point, 'longitude', NaN) };
    }),
    count: optInt(view, 'count'),
    available: optBoolean(view, 'available'),
    vehicleId: view !== null && has(view, 'vehicleId') && !isNull(view, 'vehicleId') ? optInt(view, 'vehicleId') : null,
    generation: optInt(view, 'generation'),
    from: optInt(view, 'from'),
  });
};

export const mergedTrack = (held: TrackReading | null, tail: TrackReading): TrackReading | null => {
  if (tail.from === 0) return tail;
  if (held === null || held.from !== 0 || held.vehicleId !== tail.vehicleId || held.generation !== tail.generation || held.points.length < tail.from) return null;
  return { ...tail, points: [...held.points.slice(0, tail.from), ...tail.points], from: 0 };
};

export const plottedTrack = (reading: TrackReading): readonly TrackPoint[] => reading.points.filter((it) => isPlottable(it.latitude, it.longitude));

export const trackDraws = (reading: TrackReading): boolean => reading.available && plottedTrack(reading).length > 1;
