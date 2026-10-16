import { type Json, isNull, optBoolean, optDouble } from '@/bridge/Json';
import { coordinate } from './FenceBridge';
import { MapBridge } from './MapBridge';
import type { TrackPoint } from './VehicleMap';

export const GCS_POSITION_VIEW = 'view.gcsPosition';

export const OperatorBridge = {
  read: (): Promise<Json | null> => MapBridge.read(GCS_POSITION_VIEW),
};

export const operatorHeading = (view: Json | null): number =>
  view !== null && optBoolean(view, 'usable') && !isNull(view, 'heading') ? optDouble(view, 'heading') : NaN;

export const operatorPoint = (view: Json | null): TrackPoint | null => (view !== null && optBoolean(view, 'usable') ? coordinate(view) : null);
