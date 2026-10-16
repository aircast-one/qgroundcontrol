import type { Feature, FeatureCollection } from 'geojson';
import type { Map as MapLibreMap } from 'maplibre-gl';
import type { TrackPoint } from './VehicleMap';
import type { MissionItem } from './PlanBridge';
import type { FenceCircle, FencePolygon, FirmwareFence, RallyPoint } from './FenceBridge';
import type { Survey } from './SurveyBridge';
import type { LandingPattern } from './LandingPattern';
import type { TransectArrow } from './TransectMarks';

export const MINIMUM_CIRCLE_RADIUS_METRES = 0.1;
export const GCS_SOURCE = 'aircast-gcs';
export const SHOT_SOURCE = 'aircast-shots';
export const EDGE_LABEL_SOURCE = 'aircast-edge-labels';
export const LANDING_LABEL_TEXT = 'text';
export const LANDING_AREA_SOURCE = 'aircast-landing-area';
export const LANDING_LABEL_SOURCE = 'aircast-landing-labels';
export const LANDING_PATH_SOURCE = 'aircast-landing-path';
export const LANDING_LOITER_SOURCE = 'aircast-landing-loiter';
export const GCS_HEADING_PROPERTY = 'heading';

export type OtherMission = { items: readonly MissionItem[]; linkStartToHome: boolean };

const point = (at: TrackPoint, properties: Record<string, number> = {}): Feature => ({
  type: 'Feature',
  geometry: { type: 'Point', coordinates: [at.longitude, at.latitude] },
  properties,
});

export const operatorFeatures = (at: TrackPoint | null, heading: number = NaN): FeatureCollection => ({
  type: 'FeatureCollection',
  features: at === null ? [] : [point(at, Number.isNaN(heading) ? {} : { [GCS_HEADING_PROPERTY]: heading })],
});

export const shotFeatures = (points: readonly TrackPoint[]): FeatureCollection => ({ type: 'FeatureCollection', features: points.map((it) => point(it)) });

export const legArrows = (_items: readonly MissionItem[], _linkStartToHome: boolean): readonly TransectArrow[] => [];

export const loiterRotationArrows = (_items: readonly MissionItem[]): readonly TransectArrow[] => [];

export const loiterHandleFeatures = (_items: readonly MissionItem[], _selected: number | null): readonly Feature[] => [];

export const installSurveyLayers = (_style: MapLibreMap): void => undefined;

export const installLandingLayers = (_style: MapLibreMap): void => undefined;

export const installMidpointLayer = (_style: MapLibreMap): void => undefined;

export const installShotLayer = (_style: MapLibreMap): void => undefined;

export const installFenceLayers = (_style: MapLibreMap): void => undefined;

export const installMissionLayers = (_style: MapLibreMap): void => undefined;

export const installFenceHandleLayer = (_style: MapLibreMap): void => undefined;

export const renderCollisionLegs = (_style: MapLibreMap, _legs: readonly (readonly [TrackPoint, TrackPoint])[]): void => undefined;

export const renderFences = (
  _style: MapLibreMap,
  _polygons: readonly FencePolygon[],
  _rally: readonly RallyPoint[],
  _circles: readonly FencePolygon[] = [],
  _firmware: FirmwareFence | null = null,
  _breach: TrackPoint | null = null,
): void => undefined;

export const renderMidpoints = (_style: MapLibreMap, _polygons: readonly FencePolygon[], _surveys: readonly Survey[], _items: readonly MissionItem[] = [], _selected: number | null = null): void => undefined;

export const renderMission = (
  _style: MapLibreMap,
  _items: readonly MissionItem[],
  _linkStartToHome: boolean,
  _selectedIndex: number | null = null,
  _others: readonly OtherMission[] = [],
  _landings: readonly LandingPattern[] = [],
  _legs: boolean = false,
): void => undefined;

export const renderSurveys = (_style: MapLibreMap, _surveys: readonly Survey[]): void => undefined;

export const renderVertexHandles = (
  _style: MapLibreMap,
  _polygons: readonly FencePolygon[],
  _surveys: readonly Survey[],
  _circles: readonly FenceCircle[] = [],
  _landings: readonly LandingPattern[] = [],
  _circled: ReadonlySet<string> = new Set(),
  _loiterHandles: readonly Feature[] = [],
): void => undefined;
