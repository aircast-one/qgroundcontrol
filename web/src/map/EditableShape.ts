import { isJson, jsonStrings, optBoolean, optDouble, optJSONArray, optText } from '@/bridge/Json';
import { type FencePolygon, coordinate } from './FenceBridge';
import type { LandingLabel } from './LandingPattern';
import { MapBridge } from './MapBridge';
import type { MapHit } from './MissionEditing';
import type { Survey } from './SurveyBridge';
import type { TrackPoint } from './VehicleMap';

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

export const EditableShape = (fields: {
  path: string;
  midpoints: readonly TrackPoint[];
  splitInvokable: string;
  canRemoveVertex: boolean;
  edgeLengths?: readonly string[];
  distanceUnit?: string;
  metresPerUnit?: number;
  caption?: string;
  circleCaption?: string;
}): EditableShape => ({ edgeLengths: [], distanceUnit: 'm', metresPerUnit: 1.0, caption: '', circleCaption: '', ...fields });

export const editableShape = async (path: string, shape = ''): Promise<EditableShape | null> => {
  const call = shape.trim() === '' ? `view.polygon(${path})` : `view.polygon(${path},${shape})`;
  const view = await MapBridge.read(call);
  if (view === null || optText(view, 'class') !== 'EditablePolygon') return null;
  const metresPerUnit = optDouble(view, 'horizontalMetresPerUnit', 1.0);
  const unit = optText(view, 'horizontalUnit');
  return EditableShape({
    path,
    midpoints: (optJSONArray(view, 'midpoints') ?? []).flatMap((entry) => {
      const at = coordinate(isJson(entry) ? entry : null);
      return at === null ? [] : [at];
    }),
    splitInvokable: optText(view, 'splitInvokable'),
    canRemoveVertex: optBoolean(view, 'canRemoveVertex'),
    edgeLengths: jsonStrings(optJSONArray(view, 'edgeLengths')),
    distanceUnit: unit.trim() === '' ? 'm' : unit,
    metresPerUnit: Number.isFinite(metresPerUnit) && metresPerUnit > 0 ? metresPerUnit : 1.0,
    caption: optText(view, 'caption'),
    circleCaption: optText(view, 'circleCaption'),
  });
};

export const edgeLabels = (hit: MapHit | null, fences: readonly FencePolygon[], surveys: readonly Survey[]): readonly LandingLabel[] => {
  const shape =
    hit?.kind === 'FenceVertex'
      ? (fences.find((it) => it.index === hit.polygon)?.editable ?? null)
      : hit?.kind === 'SurveyVertex'
        ? (surveys.find((it) => it.index === hit.item)?.editable ?? null)
        : null;
  return shape === null
    ? []
    : shape.midpoints.flatMap((at, index) => {
        const text = shape.edgeLengths[index];
        return text === undefined ? [] : [{ at, text }];
      });
};
