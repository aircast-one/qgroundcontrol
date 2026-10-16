import type { FeatureCollection } from 'geojson';
import type { Map as MapLibreMap } from 'maplibre-gl';
import { type Json, isJson, isNull, optBoolean, optDouble, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { format } from '@/kotlin/Std';
import { MapBridge } from './MapBridge';
import { VEHICLE_LABEL_LAYER, addBitmap, drawnImage, featureCollection, geoJsonSource, isPlottable, pointFeature } from './VehicleMap';

export const TRAFFIC_SOURCE = 'traffic';
export const TRAFFIC_LAYER = 'traffic-aircraft';
const TRAFFIC_VIEW = 'view.adsbTraffic';
const ALERT_IMAGE = 'traffic-alert';
const AWARENESS_IMAGE = 'traffic-awareness';
const HEADING = 'heading';
const ALERT = 'alert';
const LABEL = 'label';
const AIRCRAFT_PX = 44;

export type TrafficMark = { readonly latitude: number; readonly longitude: number; readonly heading: number | null; readonly alert: boolean; readonly label: string };

const numberAt = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

export const trafficLabel = (altitude: number | null, unit: string, callsign: string): string =>
  altitude === null ? '' : format('%.0f %s\n%s', altitude, unit, callsign).trimEnd();

export const trafficMarks = (view: Json | null): readonly TrafficMark[] => {
  const contacts = optJSONArray(view, 'contacts');
  if (contacts === null) return [];
  const unit = optText(optJSONObject(view, 'units'), 'altitude');
  return contacts.flatMap((contact) => {
    if (!isJson(contact)) return [];
    const latitude = numberAt(contact, 'latitude');
    const longitude = numberAt(contact, 'longitude');
    if (latitude === null || longitude === null || !isPlottable(latitude, longitude)) return [];
    return [{
      latitude,
      longitude,
      heading: numberAt(contact, 'headingDegrees'),
      alert: optBoolean(contact, 'alert'),
      label: trafficLabel(numberAt(contact, 'altitude'), unit, optText(contact, 'callsign')),
    }];
  });
};

export const TrafficBridge = {
  read: async (): Promise<readonly TrafficMark[]> => trafficMarks(await MapBridge.read(TRAFFIC_VIEW)),
};

export const trafficFeatures = (marks: readonly TrafficMark[]): FeatureCollection =>
  featureCollection(marks.map((mark) => pointFeature(mark.longitude, mark.latitude, { [HEADING]: mark.heading ?? 0.0, [ALERT]: mark.alert, [LABEL]: mark.label })));

const AIRCRAFT_OUTLINE: readonly (readonly [number, number])[] = [
  [0.5, 0.06], [0.58, 0.42], [0.94, 0.58], [0.58, 0.6], [0.56, 0.82], [0.7, 0.92],
  [0.3, 0.92], [0.44, 0.82], [0.42, 0.6], [0.06, 0.58], [0.42, 0.42],
];

const aircraft = (fill: string): ImageData =>
  drawnImage(AIRCRAFT_PX, AIRCRAFT_PX, (canvas) => {
    const s = AIRCRAFT_PX;
    const path = new Path2D(`M${AIRCRAFT_OUTLINE.map(([x, y]) => `${s * x} ${s * y}`).join(' L')} Z`);
    canvas.strokeStyle = '#000000';
    canvas.lineWidth = 4;
    canvas.lineJoin = 'round';
    canvas.stroke(path);
    canvas.fillStyle = fill;
    canvas.fill(path);
  });

export const installTrafficLayer = (style: MapLibreMap): void => {
  if (style.getSource(TRAFFIC_SOURCE) !== undefined) return;
  addBitmap(style, ALERT_IMAGE, aircraft('#FF5252'));
  addBitmap(style, AWARENESS_IMAGE, aircraft('#FFFFFF'));
  style.addSource(TRAFFIC_SOURCE, geoJsonSource());
  style.addLayer(
    {
      id: TRAFFIC_LAYER,
      type: 'symbol',
      source: TRAFFIC_SOURCE,
      layout: {
        'icon-image': ['case', ['get', ALERT], ALERT_IMAGE, AWARENESS_IMAGE],
        'icon-rotate': ['get', HEADING],
        'icon-rotation-alignment': 'map',
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
        'text-field': ['get', LABEL],
        'text-size': 11,
        'text-anchor': 'top',
        'text-offset': [0, 1.6],
        'text-allow-overlap': false,
        'text-optional': true,
      },
      paint: { 'text-color': '#FFFFFF', 'text-halo-color': '#000000', 'text-halo-width': 1.5 },
    },
    style.getLayer(VEHICLE_LABEL_LAYER) !== undefined ? VEHICLE_LABEL_LAYER : undefined,
  );
};
