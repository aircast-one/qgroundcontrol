import type { FeatureCollection } from 'geojson';
import type { MissionItem } from './PlanBridge';
import type { TrackPoint } from './VehicleMap';

export type GlideSlopeHeights = { transition: string; midSlope: string; approach: string };

export type LandingLabel = { at: TrackPoint; text: string };

export type LandingPattern = {
  index: number;
  landing: TrackPoint | null;
  slopeStart: TrackPoint | null;
  finalApproach: TrackPoint | null;
  loiterRadiusMetres: number | null;
  loiterClockwise: boolean;
  loiterRadiusText: string;
  loiterToAltitude: boolean;
  heights: GlideSlopeHeights | null;
  collides: boolean;
  glideSlopeShown: boolean;
};

const none = (): FeatureCollection => ({ type: 'FeatureCollection', features: [] });

export const landingAreaFeatures = (_patterns: readonly LandingPattern[]): FeatureCollection => none();

export const landingLabelFeatures = (_patterns: readonly LandingPattern[], _selected: number | null, _items: readonly MissionItem[] = []): FeatureCollection => none();

export const landingPathFeatures = (_patterns: readonly LandingPattern[]): FeatureCollection => none();

export const landingLoiterFeatures = (_patterns: readonly LandingPattern[], _items: readonly MissionItem[] = []): FeatureCollection => none();
