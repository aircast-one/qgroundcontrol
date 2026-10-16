import type { Map as MapLibreMap } from 'maplibre-gl';
import type { TrackPoint } from './VehicleMap';
import type { MissionItem } from './PlanBridge';
import type { Survey } from './SurveyBridge';

export type TransectArrow = { at: TrackPoint; bearing: number };

export type TransectMarks = { arrows: readonly TransectArrow[]; stubs: readonly (readonly TrackPoint[])[] };

export const installTransectMarks = (_style: MapLibreMap): void => undefined;

export const renderTransectMarks = (_style: MapLibreMap, _surveys: readonly Survey[], _selected: number | null, _legArrows: readonly TransectArrow[] = []): void => undefined;

export const renderGimbalWedges = (_style: MapLibreMap, _items: readonly MissionItem[]): void => undefined;
