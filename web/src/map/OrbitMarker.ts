import type { Map as MapLibreMap } from 'maplibre-gl';
import { type Json, optBoolean, optDouble, optJSONObject } from '@/bridge/Json';
import { coerceAtLeast } from '@/kotlin/Std';
import { circleRing, pointAt } from './FenceBridge';
import { MapBridge } from './MapBridge';
import { MINIMUM_CIRCLE_RADIUS_METRES } from './MissionLayers';
import { metresBetween } from './TerrainProfile';
import { type TrackPoint, featureCollection, geoJsonSource, lineFeature, pointFeature, setGeoJson } from './VehicleMap';

export const ORBIT_VIEW = 'view.orbit';
const ORBIT_SOURCE = 'aircast-orbit';
const ORBIT_RING_SOURCE = 'aircast-orbit-ring';
const ORBIT_LAYER = 'aircast-orbit-layer';
const ORBIT_LABEL_LAYER = 'aircast-orbit-label';
const ORBIT_RING_LAYER = 'aircast-orbit-ring-layer';
const ORBIT_COLOUR = '#FFFFFF';
const ORBIT_ARROW_SOURCE = 'aircast-orbit-arrows';
const ORBIT_ARROW_LAYER = 'aircast-orbit-arrow-layer';
const ARROW_BEARING = 'bearing';
const ORBIT_HANDLE_SOURCE = 'aircast-orbit-handles';
const ORBIT_HANDLE_LAYER = 'aircast-orbit-handle-layer';

export type OrbitCircle = { readonly centre: TrackPoint; readonly radiusMetres: number; readonly clockwise: boolean };

export const OrbitCircle = (fields: { centre: TrackPoint; radiusMetres: number; clockwise?: boolean }): OrbitCircle => ({ clockwise: true, ...fields });

export const orbitArrows = (orbit: OrbitCircle | null): readonly (readonly [TrackPoint, number])[] =>
  orbit === null
    ? []
    : ([[0.0, 90.0], [180.0, 270.0]] as const).map(([around, travel]) => [pointAt(orbit.centre, orbit.radiusMetres, around), orbit.clockwise ? travel : (travel + 180.0) % 360.0] as const);

export const orbitCircle = (view: Json | null): OrbitCircle | null => {
  if (view === null || !optBoolean(view, 'orbiting')) return null;
  const centre = optJSONObject(view, 'centre');
  if (centre === null) return null;
  const radius = optDouble(view, 'radiusMetres');
  if (Number.isNaN(radius) || radius <= 0) return null;
  return OrbitCircle({ centre: { latitude: optDouble(centre, 'latitude'), longitude: optDouble(centre, 'longitude') }, radiusMetres: radius, clockwise: optBoolean(view, 'clockwise', true) });
};

export const orbitRing = (orbit: OrbitCircle | null): readonly TrackPoint[] => {
  const ring = orbit === null ? [] : circleRing(orbit.centre, orbit.radiusMetres);
  const first = ring[0];
  return first === undefined ? [] : [...ring, first];
};

export const orbitRadiusHandle = (orbit: OrbitCircle | null): TrackPoint | null => (orbit === null ? null : pointAt(orbit.centre, orbit.radiusMetres, 90.0));

export const orbitHandles = (preview: OrbitCircle | null): readonly TrackPoint[] => [preview?.centre ?? null, orbitRadiusHandle(preview)].filter((it) => it !== null);

export const draggedOrbitRadius = (orbit: OrbitCircle, to: TrackPoint): number => coerceAtLeast(metresBetween(orbit.centre, to), MINIMUM_CIRCLE_RADIUS_METRES);

export const OrbitBridge = {
  read: async (): Promise<OrbitCircle | null> => orbitCircle(await MapBridge.read(ORBIT_VIEW)),
};

export const installOrbitLayer = (style: MapLibreMap): void => {
  if (style.getSource(ORBIT_SOURCE) !== undefined) return;
  style.addSource(ORBIT_SOURCE, geoJsonSource());
  style.addSource(ORBIT_RING_SOURCE, geoJsonSource());
  style.addSource(ORBIT_ARROW_SOURCE, geoJsonSource());
  style.addSource(ORBIT_HANDLE_SOURCE, geoJsonSource());
  style.addLayer({ id: ORBIT_RING_LAYER, type: 'line', source: ORBIT_RING_SOURCE, paint: { 'line-color': ORBIT_COLOUR, 'line-width': 2 } });
  style.addLayer({
    id: ORBIT_ARROW_LAYER,
    type: 'symbol',
    source: ORBIT_ARROW_SOURCE,
    layout: {
      'text-field': '▲',
      'text-font': ['Noto Sans Regular'],
      'text-size': 14,
      'text-rotate': ['get', ARROW_BEARING],
      'text-rotation-alignment': 'map',
      'text-allow-overlap': true,
      'text-ignore-placement': true,
    },
    paint: { 'text-color': ORBIT_COLOUR },
  });
  style.addLayer({ id: ORBIT_LAYER, type: 'circle', source: ORBIT_SOURCE, paint: { 'circle-color': ORBIT_COLOUR, 'circle-radius': 6 } });
  style.addLayer({
    id: ORBIT_LABEL_LAYER,
    type: 'symbol',
    source: ORBIT_SOURCE,
    layout: {
      'text-field': 'Orbit',
      'text-size': 12,
      'text-offset': [0, 1.4],
      'text-anchor': 'top',
      'text-allow-overlap': true,
      'text-ignore-placement': true,
    },
    paint: { 'text-color': '#FFFFFF' },
  });
  style.addLayer({
    id: ORBIT_HANDLE_LAYER,
    type: 'circle',
    source: ORBIT_HANDLE_SOURCE,
    paint: { 'circle-color': '#FFFFFF', 'circle-radius': 9, 'circle-stroke-color': '#000000', 'circle-stroke-width': 2 },
  });
};

export const renderOrbit = (style: MapLibreMap, active: OrbitCircle | null, gotoShown: boolean, preview: OrbitCircle | null = null): void => {
  const orbit = preview ?? active;
  setGeoJson(style, ORBIT_HANDLE_SOURCE, featureCollection(orbitHandles(preview).map((it) => pointFeature(it.longitude, it.latitude))));
  setGeoJson(style, ORBIT_SOURCE, featureCollection(orbit !== null && (preview !== null || !gotoShown) ? [pointFeature(orbit.centre.longitude, orbit.centre.latitude)] : []));
  setGeoJson(style, ORBIT_ARROW_SOURCE, featureCollection(orbitArrows(orbit).map(([at, bearing]) => pointFeature(at.longitude, at.latitude, { [ARROW_BEARING]: bearing }))));
  const ring = orbitRing(orbit);
  setGeoJson(style, ORBIT_RING_SOURCE, featureCollection(ring.length === 0 ? [] : [lineFeature(ring)]));
};
