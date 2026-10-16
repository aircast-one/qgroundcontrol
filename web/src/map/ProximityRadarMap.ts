import type { Map as MapLibreMap } from 'maplibre-gl';
import { type Json, isNull, jsonObjects, optBoolean, optDouble, optJSONArray } from '@/bridge/Json';
import { circleRing, pointAt } from './FenceBridge';
import type { VehicleChoice } from './VehiclePicker';
import { type TrackPoint, featureCollection, geoJsonSource, isPlottable, lineFeature, setGeoJson } from './VehicleMap';

const RADAR_SOURCE = 'aircast-proximity-radar';
const RADAR_LAYER = 'aircast-proximity-radar-layer';
const COLOUR_PROPERTY = 'colour';
const LIMIT_COLOUR = '#FFFFFF';
const SECTOR_COLOUR = '#FF0000';
const RADAR_OPACITY = 0.5;
const RADAR_LINE_WIDTH = 3;
const SECTOR_SWEEP_DEGREES = 45.0;
const ARC_STEPS = 9;
const LIMIT_SEGMENTS = 64;

export type RadarReading = { readonly maxMeters: number | null; readonly sectors: readonly (readonly [number, number])[] };

export const radarReading = (view: Json | null): RadarReading | null => {
  if (view === null || !optBoolean(view, 'shown')) return null;
  const maxMeters = optDouble(view, 'maxMeters');
  return {
    maxMeters: isNull(view, 'maxMeters') || !Number.isFinite(maxMeters) ? null : maxMeters,
    sectors: jsonObjects(optJSONArray(view, 'sectors'))
      .filter((sector) => !isNull(sector, 'meters'))
      .map((sector) => [optDouble(sector, 'bearing'), optDouble(sector, 'meters')] as const),
  };
};

export const radarLines = (centre: TrackPoint, heading: number, reading: RadarReading): readonly (readonly [string, readonly TrackPoint[]])[] => {
  const facing = Number.isNaN(heading) ? 0.0 : heading;
  const ring = reading.maxMeters === null ? null : circleRing(centre, reading.maxMeters, LIMIT_SEGMENTS);
  const first = ring?.[0];
  const limit = ring === null || first === undefined ? null : [...ring, first];
  const arcs = reading.sectors
    .filter(([, metres]) => metres > 0)
    .map(([bearing, metres]) => {
      const start = facing + bearing - SECTOR_SWEEP_DEGREES / 2;
      return [SECTOR_COLOUR, Array.from({ length: ARC_STEPS + 1 }, (_, step) => pointAt(centre, metres, start + (SECTOR_SWEEP_DEGREES * step) / ARC_STEPS))] as const;
    });
  return [...(limit === null ? [] : [[LIMIT_COLOUR, limit] as const]), ...arcs];
};

export const installProximityRadarLayer = (style: MapLibreMap): void => {
  if (style.getSource(RADAR_SOURCE) !== undefined) return;
  style.addSource(RADAR_SOURCE, geoJsonSource());
  style.addLayer({
    id: RADAR_LAYER,
    type: 'line',
    source: RADAR_SOURCE,
    paint: { 'line-color': ['get', COLOUR_PROPERTY], 'line-width': RADAR_LINE_WIDTH, 'line-opacity': RADAR_OPACITY },
  });
};

export type PlacedRadar = { readonly at: TrackPoint; readonly heading: number; readonly reading: RadarReading };

export const placedRadars = (fleet: readonly VehicleChoice[], active: TrackPoint, activeHeading: number): readonly PlacedRadar[] =>
  fleet
    .flatMap((vehicle): PlacedRadar[] => {
      const reading = vehicle.radar;
      if (reading === null) return [];
      return vehicle.active
        ? [{ at: active, heading: activeHeading, reading }]
        : [{ at: { latitude: vehicle.latitude, longitude: vehicle.longitude }, heading: vehicle.heading, reading }];
    })
    .filter((it) => isPlottable(it.at.latitude, it.at.longitude));

export const renderProximityRadars = (style: MapLibreMap, radars: readonly PlacedRadar[]): void => {
  const lines = radars.flatMap((it) => radarLines(it.at, it.heading, it.reading));
  setGeoJson(style, RADAR_SOURCE, featureCollection(lines.map(([colour, points]) => lineFeature(points, { [COLOUR_PROPERTY]: colour }))));
};
