import type { TrackPoint } from './VehicleMap';
import type { FencePolygon } from './FenceBridge';
import type { LandingLabel } from './LandingPattern';
import type { MapHit } from './MissionEditing';
import type { Survey } from './SurveyBridge';

export type EditableShape = {
  path: string;
  midpoints: readonly TrackPoint[];
  splitInvokable: string;
  canRemoveVertex: boolean;
  edgeLengths: readonly string[];
  distanceUnit: string;
  metresPerUnit: number;
  caption: string;
  circleCaption: string;
};

export const edgeLabels = (_hit: MapHit | null, _fences: readonly FencePolygon[], _surveys: readonly Survey[]): readonly LandingLabel[] => [];
