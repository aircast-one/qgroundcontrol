import type { Map as MapLibreMap } from 'maplibre-gl';
import { type Json, optDouble, optJSONObject } from '@/bridge/Json';
import { MapBridge } from './MapBridge';
import { type TrackPoint, featureCollection, geoJsonSource, isPlottable, pointFeature, setGeoJson } from './VehicleMap';

export const ROI_GUIDED_VIEW = 'view.guidedActions';
const ROI_SOURCE = 'aircast-roi';
export const ROI_LAYER = 'aircast-roi-layer';
const ROI_LABEL_LAYER = 'aircast-roi-label';
const ROI_COLOUR = '#FF9500';

export const roiPoint = (view: Json | null): TrackPoint | null => {
  const roi = optJSONObject(view, 'roi');
  if (roi === null) return null;
  const point = { latitude: optDouble(roi, 'latitude'), longitude: optDouble(roi, 'longitude') };
  return isPlottable(point.latitude, point.longitude) ? point : null;
};

export const RoiBridge = {
  read: async (): Promise<TrackPoint | null> => roiPoint(await MapBridge.read(ROI_GUIDED_VIEW)),
};

export const installRoiLayer = (style: MapLibreMap): void => {
  if (style.getSource(ROI_SOURCE) !== undefined) return;
  style.addSource(ROI_SOURCE, geoJsonSource());
  style.addLayer({
    id: ROI_LAYER,
    type: 'circle',
    source: ROI_SOURCE,
    paint: { 'circle-color': ROI_COLOUR, 'circle-radius': 10, 'circle-stroke-color': '#FFFFFF', 'circle-stroke-width': 2 },
  });
  style.addLayer({
    id: ROI_LABEL_LAYER,
    type: 'symbol',
    source: ROI_SOURCE,
    layout: {
      'text-field': 'ROI here',
      'text-size': 12,
      'text-offset': [0, 1.6],
      'text-anchor': 'top',
      'text-allow-overlap': true,
      'text-ignore-placement': true,
    },
    paint: { 'text-color': '#FFFFFF' },
  });
};

export const renderRoi = (style: MapLibreMap, roi: TrackPoint | null): void =>
  setGeoJson(style, ROI_SOURCE, featureCollection(roi === null ? [] : [pointFeature(roi.longitude, roi.latitude)]));
