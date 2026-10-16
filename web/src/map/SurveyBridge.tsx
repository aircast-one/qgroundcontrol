import type { Json } from '@/bridge/Json';
import type { TrackPoint } from './VehicleMap';
import type { EditableShape } from './EditableShape';

export type Survey = {
  index: number;
  area: readonly TrackPoint[];
  transects: readonly TrackPoint[];
  cameraShots: number;
  kind: string;
  shape: string;
  property: string;
  editable: EditableShape | null;
  flightLoop: readonly TrackPoint[];
  layers: number;
  layerSpanText: string;
  turnaround: boolean;
  outline: readonly TrackPoint[];
  collides: boolean;
};

export const SurveyBridge = {
  surveysFrom: (_json: Json | null): readonly Survey[] => [],
};
