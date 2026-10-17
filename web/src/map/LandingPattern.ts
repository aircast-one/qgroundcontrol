import type { Feature, FeatureCollection } from 'geojson';
import { type Json, optBoolean, optDouble, optJSONObject, optString, optText } from '@/bridge/Json';
import { azimuthBetween, circleRing, pointAt } from './FenceBridge';
import { LANDING_LABEL_TEXT, LANDING_SHAPE_KIND, TERRAIN_COLLISION } from './MissionLayers';
import { KIND_STRUCTURE, type MissionItem } from './PlanBridge';
import { metresBetween } from './TerrainProfile';
import { type TrackPoint, featureCollection, lineFeature, pointFeature } from './VehicleMap';

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

export const LandingPattern = (fields: {
  index: number;
  landing: TrackPoint | null;
  slopeStart: TrackPoint | null;
  finalApproach: TrackPoint | null;
  loiterRadiusMetres: number | null;
  loiterClockwise: boolean;
  loiterRadiusText?: string;
  loiterToAltitude?: boolean;
  heights?: GlideSlopeHeights | null;
  collides?: boolean;
  glideSlopeShown?: boolean;
}): LandingPattern => ({ loiterRadiusText: '', loiterToAltitude: false, heights: null, collides: false, glideSlopeShown: true, ...fields });

const place = (view: Json | null, key: string): TrackPoint | null => {
  const at = optJSONObject(view, key);
  if (at === null) return null;
  const latitude = optDouble(at, 'latitude', NaN);
  const longitude = optDouble(at, 'longitude', NaN);
  return Number.isNaN(latitude) || Number.isNaN(longitude) ? null : { latitude, longitude };
};

export type GlideSlopeHeights = { transition: string; midSlope: string; approach: string };

export type LandingLabel = { at: TrackPoint; text: string };

export const landingLabels = (pattern: LandingPattern): readonly LandingLabel[] => {
  if (!pattern.glideSlopeShown) return [];
  const landing = pattern.landing;
  const slopeStart = pattern.slopeStart;
  if (landing === null || slopeStart === null) return [];
  const bearing = azimuthBetween(landing, slopeStart);
  const side = bearing + (bearing > 180 ? -90 : 90);
  const transition = pointAt(landing, LANDING_LENGTH_M / 2, bearing);
  const top = (pattern.loiterToAltitude ? slopeStart : pattern.finalApproach) ?? slopeStart;
  const mid = pointAt(transition, metresBetween(transition, top) / 2, bearing);
  const heights = pattern.heights;
  return [
    { at: landing, text: 'Landing Area' },
    { at: pointAt(landing, LANDING_LENGTH_M / 2 + 2, bearing), text: 'Glide Slope' },
    ...(heights === null
      ? []
      : [
          { at: pointAt(transition, LANDING_WIDTH_M, side), text: heights.transition },
          { at: pointAt(mid, LANDING_WIDTH_M / 2, side), text: heights.midSlope },
          { at: slopeStart, text: heights.approach },
        ]),
  ];
};

export const structureScanLabels = (items: readonly MissionItem[]): readonly LandingLabel[] =>
  items
    .filter((it) => it.kind === KIND_STRUCTURE)
    .flatMap((item) => {
      const entry = { latitude: item.latitude, longitude: item.longitude };
      return [
        { at: entry, text: 'Entry' },
        { at: item.exit ?? entry, text: 'Exit' },
      ];
    });

export const landingLabelFeatures = (patterns: readonly LandingPattern[], selected: number | null, items: readonly MissionItem[] = []): FeatureCollection =>
  featureCollection(
    [...patterns.filter((it) => it.index === selected).flatMap(landingLabels), ...structureScanLabels(items)].map((label) =>
      pointFeature(label.at.longitude, label.at.latitude, { [LANDING_LABEL_TEXT]: label.text }),
    ),
  );

export const isLandingPattern = (view: Json | null): boolean => view !== null && optString(view, 'kind') !== 'null' && !('reason' in view);

export const landingPattern = (index: number, view: Json | null): LandingPattern | null => {
  if (view === null || !isLandingPattern(view)) return null;
  const heights = optJSONObject(view, 'heights');
  const radius = optDouble(view, 'loiterRadiusMetres', NaN);
  const pattern = LandingPattern({
    index,
    landing: place(view, 'landing'),
    slopeStart: place(view, 'slopeStart'),
    finalApproach: place(view, 'finalApproach'),
    loiterRadiusMetres: Number.isNaN(radius) ? null : radius,
    loiterClockwise: optBoolean(view, 'loiterClockwise'),
    loiterRadiusText: optText(view, 'loiterRadiusText'),
    loiterToAltitude: optBoolean(view, 'loiterToAltitude'),
    heights: heights === null ? null : { transition: optText(heights, 'transition'), midSlope: optText(heights, 'midSlope'), approach: optText(heights, 'approach') },
    glideSlopeShown: optBoolean(view, 'glideSlopeShown', true),
  });
  return pattern.landing !== null || pattern.slopeStart !== null || pattern.finalApproach !== null ? pattern : null;
};

export const landingText = (pattern: LandingPattern | null): string | null => {
  const radius = pattern?.loiterRadiusText ?? '';
  if (pattern === null || radius.trim() === '') return null;
  return `circles ${radius} ${pattern.loiterClockwise ? 'clockwise' : 'anticlockwise'}`;
};

export const approachPath = (pattern: LandingPattern): readonly TrackPoint[] =>
  [pattern.finalApproach, pattern.slopeStart, pattern.landing].filter((it): it is TrackPoint => it !== null);

export const landingPathFeatures = (patterns: readonly LandingPattern[]): FeatureCollection =>
  featureCollection(
    patterns
      .map(approachPath)
      .filter((it) => it.length >= 2)
      .map((path) => lineFeature(path)),
  );

const LANDING_WIDTH_M = 15.0;
const LANDING_LENGTH_M = 100.0;
export const LANDING_AREA_KIND = 'area';
export const GLIDE_SLOPE_KIND = 'slope';

const landingCorners = (landing: TrackPoint, bearing: number): readonly TrackPoint[] => {
  const angle = (Math.atan(LANDING_WIDTH_M / 2 / (LANDING_LENGTH_M / 2)) * 180) / Math.PI;
  const hypotenuse = LANDING_WIDTH_M / 2 / Math.sin((angle * Math.PI) / 180);
  return [bearing - angle, bearing + angle, bearing + (180 - angle), bearing - (180 - angle)].map((it) => pointAt(landing, hypotenuse, it));
};

export const landingArea = (pattern: LandingPattern): readonly TrackPoint[] | null => {
  const { landing, slopeStart } = pattern;
  return landing === null || slopeStart === null ? null : landingCorners(landing, azimuthBetween(landing, slopeStart));
};

export const glideSlope = (pattern: LandingPattern): readonly TrackPoint[] | null => {
  const { landing, slopeStart } = pattern;
  if (landing === null || slopeStart === null) return null;
  const top = pattern.loiterToAltitude ? slopeStart : pattern.finalApproach;
  return top === null ? null : [...landingCorners(landing, azimuthBetween(landing, slopeStart)).slice(0, 2), top];
};

const closedPolygon = (ring: readonly TrackPoint[], properties: Record<string, string | boolean>): Feature => ({
  type: 'Feature',
  geometry: { type: 'Polygon', coordinates: [[...ring, ...ring.slice(0, 1)].map((it) => [it.longitude, it.latitude])] },
  properties,
});

export const landingAreaFeatures = (patterns: readonly LandingPattern[]): FeatureCollection =>
  featureCollection(
    patterns
      .filter((it) => it.glideSlopeShown)
      .flatMap((pattern) => {
        const area = landingArea(pattern);
        const slope = glideSlope(pattern);
        return [
          ...(area === null ? [] : [[LANDING_AREA_KIND, area, false] as const]),
          ...(slope === null ? [] : [[GLIDE_SLOPE_KIND, slope, pattern.collides] as const]),
        ];
      })
      .map(([kind, ring, collides]) => closedPolygon(ring, { [LANDING_SHAPE_KIND]: kind, [TERRAIN_COLLISION]: collides })),
  );

export const loiterRings = (patterns: readonly LandingPattern[], items: readonly MissionItem[]): readonly (readonly [TrackPoint, number])[] =>
  loiterCircles(patterns, items).map(([centre, radius]) => [centre, radius] as const);

const loiterCircles = (patterns: readonly LandingPattern[], items: readonly MissionItem[]): readonly (readonly [TrackPoint, number, boolean])[] => [
  ...patterns
    .filter((it) => it.loiterToAltitude)
    .flatMap((pattern) => (pattern.finalApproach !== null && pattern.loiterRadiusMetres !== null ? [[pattern.finalApproach, pattern.loiterRadiusMetres, false] as const] : [])),
  ...items
    .filter((it) => Number.isFinite(it.loiterRadius) && it.loiterRadius !== 0.0)
    .map((it) => [{ latitude: it.latitude, longitude: it.longitude }, Math.abs(it.loiterRadius), it.terrainCollision] as const),
];

export const landingLoiterFeatures = (patterns: readonly LandingPattern[], items: readonly MissionItem[] = []): FeatureCollection =>
  featureCollection(
    loiterCircles(patterns, items).flatMap(([centre, radius, collides]) => {
      const ring = circleRing(centre, radius);
      return ring.length >= 3 ? [lineFeature([...ring, ...ring.slice(0, 1)], { [TERRAIN_COLLISION]: collides })] : [];
    }),
  );
