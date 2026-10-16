import { type Json, optBoolean, optDouble, optJSONObject } from '@/bridge/Json';
import type { Fact, FactSlider } from '@/bridge/Qgc';
import type { TrackPoint } from '@/map/VehicleMap';

const HOLD_LABEL = 'Hold';

export type SpeedSection = { specified: boolean; value: number | null; units: string; path: string; specifyPath: string; slider: FactSlider | null };

export const speedSectionOf = (_served: Json | null): SpeedSection | null => null;

export const withoutHeroFields = (fields: readonly Fact[], view: Json | null): readonly Fact[] =>
  optJSONObject(view, 'hold') === null ? fields : fields.filter((it) => it.description !== HOLD_LABEL);

export const geoOf = (view: Json | null): readonly [number, number] | null =>
  view !== null && optBoolean(view, 'valid') ? [optDouble(view, 'latitude'), optDouble(view, 'longitude')] : null;

export const vehicleAltitudePath = (altitudeMode: number | null): string | null =>
  altitudeMode === 1 ? 'vehicle.altitudeRelative' : altitudeMode === 2 ? 'vehicle.altitudeAMSL' : altitudeMode === 3 || altitudeMode === 4 ? 'vehicle.altitudeAboveTerr' : null;

export const EditPositionDialog = (_props: {
  at: TrackPoint;
  onDismiss: () => void;
  title?: string;
  confirm?: string;
  vehicleConfirm?: string | null;
  altitudeMode?: number | null;
  onAltitude?: ((metres: number) => void) | null;
  onMove: (latitude: number, longitude: number) => void;
}) => null;
