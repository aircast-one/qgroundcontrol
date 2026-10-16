import type { FeatureCollection } from 'geojson';
import type { Map as MapLibreMap } from 'maplibre-gl';
import { type Json, jsonObjects, optBoolean, optDouble, optJSONArray } from '@/bridge/Json';
import { type TrackPoint, featureCollection, geoJsonSource, isPlottable, setGeoJson } from './VehicleMap';

export const SYNTHETIC_VIEW_PATH = 'view.syntheticView';
const BEAM_SOURCE = 'aircast-camera-beam';
const BEAM_FILL_LAYER = 'aircast-camera-beam-fill';
const BEAM_LINE_LAYER = 'aircast-camera-beam-line';
const BEAM_COLOUR = '#FFE9A6';
const BEAM_FILL_OPACITY = 0.28;
const BEAM_LINE_OPACITY = 0.6;
const BEAM_LINE_WIDTH = 1.5;
const POLYGON_POINTS = 4;

export const cameraBeam = (view: Json | null): readonly TrackPoint[] => {
  if (view === null || !optBoolean(view, 'available')) return [];
  return jsonObjects(optJSONArray(view, 'beam')).map((it) => ({ latitude: optDouble(it, 'latitude'), longitude: optDouble(it, 'longitude') }));
};

export const beamFeatures = (beam: readonly TrackPoint[]): FeatureCollection =>
  featureCollection(
    beam.length >= POLYGON_POINTS && beam.every((point) => isPlottable(point.latitude, point.longitude))
      ? [{ type: 'Feature', geometry: { type: 'Polygon', coordinates: [beam.map((it) => [it.longitude, it.latitude])] }, properties: {} }]
      : [],
  );

export const installCameraBeamLayer = (style: MapLibreMap): void => {
  if (style.getSource(BEAM_SOURCE) !== undefined) return;
  style.addSource(BEAM_SOURCE, geoJsonSource());
  style.addLayer({ id: BEAM_FILL_LAYER, type: 'fill', source: BEAM_SOURCE, paint: { 'fill-color': BEAM_COLOUR, 'fill-opacity': BEAM_FILL_OPACITY } });
  style.addLayer({ id: BEAM_LINE_LAYER, type: 'line', source: BEAM_SOURCE, paint: { 'line-color': BEAM_COLOUR, 'line-opacity': BEAM_LINE_OPACITY, 'line-width': BEAM_LINE_WIDTH } });
};

export const renderCameraBeam = (style: MapLibreMap, beam: readonly TrackPoint[]): void => setGeoJson(style, BEAM_SOURCE, beamFeatures(beam));
