import { type Json, optBoolean, optDouble, optText } from '@/bridge/Json';
import type { TrackPoint } from '@/map/VehicleMap';

export const MAP_HOLD_HINT = 'Press and hold the map to fly there';
export const MAP_CLICK_PATH = 'view.mapClick';

export type MapHoldHint = { readonly name: 'Unseen' | 'Due' | 'Learned'; readonly ordinal: number };

export const MapHoldHint = {
  Unseen: { name: 'Unseen', ordinal: 0 },
  Due: { name: 'Due', ordinal: 1 },
  Learned: { name: 'Learned', ordinal: 2 },
} as const satisfies Record<string, MapHoldHint>;

export type MapPoint = { latitude: number; longitude: number };

export type LoiterOffer = { latitude: number; longitude: number; title: string; message: string; defaultRadius: number; clockwise: boolean };

export type OrbitDefaults = { radius: number; unit: string; metresPerUnit: number; clockwise: boolean };

export const orbitDefaults = (view: Json | null): OrbitDefaults => {
  const radius = optDouble(view, 'orbitDefaultRadius');
  const metresPerUnit = optDouble(view, 'orbitMetresPerUnit');
  return {
    radius: Number.isNaN(radius) ? 0.0 : radius,
    unit: optText(view, 'orbitRadiusUnit'),
    metresPerUnit: !Number.isNaN(metresPerUnit) && metresPerUnit > 0 ? metresPerUnit : 1.0,
    clockwise: view === null ? true : optBoolean(view, 'orbitClockwise', true),
  };
};

export const loiterOffer = (_view: Json | null): LoiterOffer | null => null;

export const mapClickUnits = (view: Json | null): OrbitDefaults => orbitDefaults(view);

export const MapClickMenu = (_props: { point: MapPoint; onDismiss: () => void }) => null;

export const RoiSheet = (_props: { at: TrackPoint; onDismiss: () => void }) => null;

export const SetWaypointSheet = (_props: { sequence: number; onDismiss: () => void }) => null;

export const LoiterRadiusPanel = (_props: { offer: LoiterOffer; units: OrbitDefaults; onRefused: (message: string) => void; onDone: () => void }) => null;
