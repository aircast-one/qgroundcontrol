import type { Map as MapLibreMap } from 'maplibre-gl';
import type { TrackPoint } from './VehicleMap';
import { PLAN_ITEMS } from './PlanBridge';
import type { Survey } from './SurveyBridge';
import { FENCE_POLYGONS } from './FenceBridge';

export const fencePath = (index: number): string => `${FENCE_POLYGONS}.${index}`;

export const surveyPath = (survey: Survey): string => `${PLAN_ITEMS}.${survey.index}.${survey.property}`;

export const installTraceLayer = (_style: MapLibreMap): void => undefined;

export const renderTrace = (_style: MapLibreMap, _points: readonly TrackPoint[], _line: boolean): void => undefined;
