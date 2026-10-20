import type { FeatureCollection } from 'geojson';
import type { Map as MapLibreMap } from 'maplibre-gl';
import { type Json, jsonObjects, optBoolean, optDouble, optJSONArray } from '@/bridge/Json';
import { type TrackPoint, featureCollection, geoJsonSource, isPlottable, pointFeature, setGeoJson } from './VehicleMap';

const BOTTLE_SOURCE = 'aircast-dropped-bottles';
const BOTTLE_LAYER = 'aircast-dropped-bottles';
const BOTTLE_COLOUR = '#1565C0';

export const landedBottles = (view: Json | null): readonly TrackPoint[] =>
  jsonObjects(optJSONArray(view, 'bottles'))
    .filter((it) => optBoolean(it, 'landed'))
    .map((it) => ({ latitude: optDouble(it, 'latitude'), longitude: optDouble(it, 'longitude') }))
    .filter((it) => isPlottable(it.latitude, it.longitude));

export const bottleFeatures = (bottles: readonly TrackPoint[]): FeatureCollection => featureCollection(bottles.map((it) => pointFeature(it.longitude, it.latitude)));

export const installBottleLayer = (style: MapLibreMap): void => {
  if (style.getSource(BOTTLE_SOURCE) !== undefined) return;
  style.addSource(BOTTLE_SOURCE, geoJsonSource());
  style.addLayer({ id: BOTTLE_LAYER, type: 'circle', source: BOTTLE_SOURCE, paint: { 'circle-radius': 5, 'circle-color': BOTTLE_COLOUR, 'circle-stroke-width': 2, 'circle-stroke-color': '#FFFFFF' } });
};

export const renderBottles = (style: MapLibreMap, bottles: readonly TrackPoint[]): void => setGeoJson(style, BOTTLE_SOURCE, bottleFeatures(bottles));
