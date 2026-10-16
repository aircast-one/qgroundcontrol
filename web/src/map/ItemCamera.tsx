import type { Json } from '@/bridge/Json';
import type { TrackPoint } from './VehicleMap';

export const VideoBridge = {
  read: async (): Promise<Json | null> => null,
};

export const shotPoints = (_view: Json | null): readonly TrackPoint[] => [];
