import type { FeatureCollection } from 'geojson';
import type { Map as MapLibreMap, MapMouseEvent, MapTouchEvent } from 'maplibre-gl';
import { type Json, optBoolean, optDouble, optJSONObject, optText } from '@/bridge/Json';
import { coerceAtLeast, format } from '@/kotlin/Std';
import { pointAt, circleRing } from './FenceBridge';
import type { FlyMapEdits } from './FlyMapEdits';
import { MapBridge } from './MapBridge';
import { withinHit, withinTap } from './MissionEditing';
import { MINIMUM_CIRCLE_RADIUS_METRES } from './MissionLayers';
import { OrbitCircle, draggedOrbitRadius, orbitArrows, orbitRadiusHandle } from './OrbitMarker';
import { metresBetween } from './TerrainProfile';
import { type TrackPoint, featureCollection, geoJsonSource, isPlottable, lineFeature, pointFeature, setAllGesturesEnabled, setGeoJson } from './VehicleMap';

export const GOTO_MAP_CLICK_VIEW = 'view.mapClick';
const GOTO_SOURCE = 'aircast-goto';
const GOTO_RING_SOURCE = 'aircast-goto-ring';
const GOTO_LAYER = 'aircast-goto-layer';
const GOTO_LABEL_LAYER = 'aircast-goto-label';
const GOTO_RING_LAYER = 'aircast-goto-ring-layer';
const GOTO_RADIUS_SOURCE = 'aircast-goto-radius';
const GOTO_RADIUS_LAYER = 'aircast-goto-radius-layer';
const RADIUS_TEXT = 'text';
const GOTO_ARROW_SOURCE = 'aircast-goto-arrows';
const GOTO_ARROW_LAYER = 'aircast-goto-arrow-layer';
const ARROW_BEARING = 'bearing';
const GOTO_HANDLE_SOURCE = 'aircast-goto-handle';
const GOTO_HANDLE_LAYER = 'aircast-goto-handle-layer';
const GOTO_COLOUR = '#2E7D32';
const GOTO_PATH_SOURCE = 'aircast-goto-path';
const GOTO_PATH_LAYER = 'aircast-goto-path-layer';
const PATH_DASH = 2;
const PATH_GAP = 1.5;

export type GotoLocation = {
  readonly at: TrackPoint;
  readonly loiterRadiusMetres: number | null;
  readonly loiterRadiusText: string;
  readonly loiterClockwise: boolean;
  readonly arrived: boolean;
  readonly progressText: string;
};

export const GotoLocation = (fields: { at: TrackPoint; loiterRadiusMetres: number | null; loiterRadiusText?: string; loiterClockwise?: boolean; arrived?: boolean; progressText?: string }): GotoLocation => ({
  loiterRadiusText: '',
  loiterClockwise: true,
  arrived: false,
  progressText: '',
  ...fields,
});

export const previewGoto = (confirmed: GotoLocation | null, preview: TrackPoint | null): GotoLocation | null =>
  preview !== null ? GotoLocation({ at: preview, loiterRadiusMetres: null }) : confirmed !== null && !confirmed.arrived ? confirmed : null;

export const gotoPath = (from: TrackPoint | null, preview: TrackPoint | null): readonly TrackPoint[] => {
  const ends = [from !== null && isPlottable(from.latitude, from.longitude) ? from : null, preview].filter((it) => it !== null);
  return ends.length === 2 ? ends : [];
};

export const gotoLocation = (view: Json | null): GotoLocation | null => {
  const json = optJSONObject(view, 'gotoLocation');
  if (json === null) return null;
  const radius = optDouble(json, 'loiterRadiusMetres');
  const location = GotoLocation({
    at: { latitude: optDouble(json, 'latitude'), longitude: optDouble(json, 'longitude') },
    loiterRadiusMetres: !Number.isNaN(radius) && radius > 0 ? radius : null,
    loiterRadiusText: optText(json, 'loiterRadiusText'),
    loiterClockwise: optBoolean(json, 'loiterClockwise', true),
    arrived: optBoolean(json, 'arrived'),
    progressText: optText(json, 'progressText'),
  });
  return isPlottable(location.at.latitude, location.at.longitude) ? location : null;
};

export const gotoRing = (location: GotoLocation | null): readonly TrackPoint[] => {
  const ring = location?.loiterRadiusMetres == null ? [] : circleRing(location.at, location.loiterRadiusMetres);
  const first = ring[0];
  return first === undefined ? [] : [...ring, first];
};

export const gotoArrows = (location: GotoLocation | null): readonly (readonly [TrackPoint, number])[] =>
  orbitArrows(location?.loiterRadiusMetres == null ? null : OrbitCircle({ centre: location.at, radiusMetres: location.loiterRadiusMetres, clockwise: location.loiterClockwise }));

export type LoiterEdit = { readonly radiusMetres: number; readonly clockwise: boolean; readonly unit: string; readonly metresPerUnit: number };

export const loiterEditNumber = (edit: LoiterEdit): string => {
  const text = format('%.1f', edit.radiusMetres / edit.metresPerUnit);
  return text.endsWith('.0') ? text.slice(0, -2) : text;
};

export const editedGoto = (location: GotoLocation | null, edit: LoiterEdit | null): GotoLocation | null => {
  if (edit === null || location === null || location.loiterRadiusMetres === null) return location;
  return {
    ...location,
    loiterRadiusMetres: edit.radiusMetres,
    loiterClockwise: edit.clockwise,
    loiterRadiusText: [loiterEditNumber(edit), edit.unit].filter((it) => it.trim() !== '').join(' '),
  };
};

export const gotoRadiusHandle = (location: GotoLocation | null): TrackPoint | null =>
  location?.loiterRadiusMetres == null ? null : pointAt(location.at, location.loiterRadiusMetres, 90.0);

export const draggedGotoRadius = (location: GotoLocation, to: TrackPoint): number => coerceAtLeast(metresBetween(location.at, to), MINIMUM_CIRCLE_RADIUS_METRES);

type CircleGrab = 'None' | 'GotoRadius' | 'GotoFlip' | 'OrbitRadius' | 'OrbitCentre' | 'OrbitFlip';

const DRAGGING: ReadonlySet<CircleGrab> = new Set(['GotoRadius', 'OrbitRadius', 'OrbitCentre']);

export const attachGotoRadiusDrag = (_mapView: HTMLElement, map: MapLibreMap, edits: FlyMapEdits, shown: () => GotoLocation | null): (() => void) => {
  const touch = { grab: 'None' as CircleGrab, downX: 0, downY: 0, swallowsClick: false };
  const near = (at: TrackPoint | null, x: number, y: number): boolean => {
    if (at === null) return false;
    const point = map.project([at.longitude, at.latitude]);
    return withinHit(point.x - x, point.y - y);
  };
  const nearArrow = (arrows: readonly (readonly [TrackPoint, number])[], x: number, y: number): boolean => arrows.some(([at]) => near(at, x, y));
  const grabbed = (x: number, y: number): CircleGrab => {
    const orbit = edits.orbit.value;
    const goto = edits.gotoLoiter.value !== null ? shown() : null;
    if (near(orbitRadiusHandle(orbit), x, y)) return 'OrbitRadius';
    if (near(orbit?.centre ?? null, x, y)) return 'OrbitCentre';
    if (nearArrow(orbitArrows(orbit), x, y)) return 'OrbitFlip';
    if (near(gotoRadiusHandle(goto), x, y)) return 'GotoRadius';
    if (nearArrow(gotoArrows(goto), x, y)) return 'GotoFlip';
    return 'None';
  };
  const dragTo = (x: number, y: number) => {
    const lngLat = map.unproject([x, y]);
    const to = { latitude: lngLat.lat, longitude: lngLat.lng };
    const orbit = edits.orbit.value;
    const edit = edits.gotoLoiter.value;
    const goto = shown();
    if (touch.grab === 'OrbitRadius' && orbit !== null) edits.orbit.value = { ...orbit, radiusMetres: draggedOrbitRadius(orbit, to) };
    else if (touch.grab === 'OrbitCentre' && orbit !== null) edits.orbit.value = { ...orbit, centre: to };
    else if (touch.grab === 'GotoRadius' && edit !== null && goto !== null) edits.gotoLoiter.value = { ...edit, radiusMetres: draggedGotoRadius(goto, to) };
  };
  const flip = () => {
    const orbit = edits.orbit.value;
    const edit = edits.gotoLoiter.value;
    if (touch.grab === 'OrbitFlip' && orbit !== null) edits.orbit.value = { ...orbit, clockwise: !orbit.clockwise };
    else if (touch.grab === 'GotoFlip' && edit !== null) edits.gotoLoiter.value = { ...edit, clockwise: !edit.clockwise };
  };
  const dragging = () => DRAGGING.has(touch.grab);
  const down = (event: MapMouseEvent | MapTouchEvent) => {
    if (event.type === 'touchstart' && (event as MapTouchEvent).points.length !== 1) return;
    if (event.type === 'mousedown' && (event as MapMouseEvent).originalEvent.button !== 0) return;
    touch.swallowsClick = false;
    touch.downX = event.point.x;
    touch.downY = event.point.y;
    touch.grab = grabbed(event.point.x, event.point.y);
    if (touch.grab === 'None') return;
    event.preventDefault();
    if (dragging()) setAllGesturesEnabled(map, false);
  };
  const move = (event: MapMouseEvent | MapTouchEvent) => {
    if (dragging()) dragTo(event.point.x, event.point.y);
  };
  const release = (at: { readonly x: number; readonly y: number } | null) => {
    if (touch.grab === 'None') return;
    if (at !== null && withinTap(at.x - touch.downX, at.y - touch.downY)) flip();
    if (dragging()) setAllGesturesEnabled(map, true);
    touch.grab = 'None';
    touch.swallowsClick = true;
  };
  const lifted = (event: MapMouseEvent | MapTouchEvent) => {
    if (event.type === 'touchend' && touch.grab !== 'None') event.originalEvent.preventDefault();
    release(event.point);
  };
  const cancelled = () => release(null);
  const clicked = (event: MapMouseEvent) => {
    if (!touch.swallowsClick) return;
    touch.swallowsClick = false;
    event.preventDefault();
  };
  map.on('mousedown', down);
  map.on('touchstart', down);
  map.on('mousemove', move);
  map.on('touchmove', move);
  map.on('mouseup', lifted);
  map.on('touchend', lifted);
  map.on('touchcancel', cancelled);
  map.on('click', clicked);
  window.addEventListener('mouseup', cancelled);
  return () => {
    map.off('mousedown', down);
    map.off('touchstart', down);
    map.off('mousemove', move);
    map.off('touchmove', move);
    map.off('mouseup', lifted);
    map.off('touchend', lifted);
    map.off('touchcancel', cancelled);
    map.off('click', clicked);
    window.removeEventListener('mouseup', cancelled);
  };
};

export const GotoBridge = {
  read: async (): Promise<GotoLocation | null> => gotoLocation(await MapBridge.read(GOTO_MAP_CLICK_VIEW)),
};

export const installGotoLayer = (style: MapLibreMap): void => {
  if (style.getSource(GOTO_SOURCE) !== undefined) return;
  style.addSource(GOTO_SOURCE, geoJsonSource());
  style.addSource(GOTO_RING_SOURCE, geoJsonSource());
  style.addSource(GOTO_RADIUS_SOURCE, geoJsonSource());
  style.addSource(GOTO_ARROW_SOURCE, geoJsonSource());
  style.addSource(GOTO_HANDLE_SOURCE, geoJsonSource());
  style.addSource(GOTO_PATH_SOURCE, geoJsonSource());
  style.addLayer({ id: GOTO_PATH_LAYER, type: 'line', source: GOTO_PATH_SOURCE, paint: { 'line-color': GOTO_COLOUR, 'line-width': 3, 'line-dasharray': [PATH_DASH, PATH_GAP] } });
  style.addLayer({ id: GOTO_RING_LAYER, type: 'line', source: GOTO_RING_SOURCE, paint: { 'line-color': GOTO_COLOUR, 'line-width': 2 } });
  style.addLayer({
    id: GOTO_ARROW_LAYER,
    type: 'symbol',
    source: GOTO_ARROW_SOURCE,
    layout: {
      'text-field': '▲',
      'text-font': ['Noto Sans Regular'],
      'text-size': 14,
      'text-rotate': ['get', ARROW_BEARING],
      'text-rotation-alignment': 'map',
      'text-allow-overlap': true,
      'text-ignore-placement': true,
    },
    paint: { 'text-color': GOTO_COLOUR },
  });
  style.addLayer({
    id: GOTO_LAYER,
    type: 'circle',
    source: GOTO_SOURCE,
    paint: { 'circle-color': GOTO_COLOUR, 'circle-radius': 9, 'circle-stroke-color': '#FFFFFF', 'circle-stroke-width': 2 },
  });
  style.addLayer({
    id: GOTO_RADIUS_LAYER,
    type: 'symbol',
    source: GOTO_RADIUS_SOURCE,
    layout: {
      'text-field': ['get', RADIUS_TEXT],
      'text-size': 12,
      'text-anchor': 'bottom',
      'text-allow-overlap': true,
      'text-ignore-placement': true,
    },
    paint: { 'text-color': '#000000', 'text-halo-color': 'rgba(255, 255, 255, 0.5)', 'text-halo-width': 4 },
  });
  style.addLayer({
    id: GOTO_HANDLE_LAYER,
    type: 'circle',
    source: GOTO_HANDLE_SOURCE,
    paint: { 'circle-color': '#FFFFFF', 'circle-radius': 9, 'circle-stroke-color': GOTO_COLOUR, 'circle-stroke-width': 3 },
  });
  style.addLayer({
    id: GOTO_LABEL_LAYER,
    type: 'symbol',
    source: GOTO_SOURCE,
    layout: {
      'text-field': 'Go here',
      'text-size': 12,
      'text-offset': [0, 1.6],
      'text-anchor': 'top',
      'text-allow-overlap': true,
      'text-ignore-placement': true,
    },
    paint: { 'text-color': '#FFFFFF' },
  });
};

export const renderGotoPath = (style: MapLibreMap, path: readonly TrackPoint[]): void =>
  setGeoJson(style, GOTO_PATH_SOURCE, featureCollection(path.length >= 2 ? [lineFeature(path)] : []));

export const renderGoto = (style: MapLibreMap, location: GotoLocation | null, editing = false): void => {
  const handle = gotoRadiusHandle(editing ? location : null);
  setGeoJson(style, GOTO_HANDLE_SOURCE, featureCollection(handle === null ? [] : [pointFeature(handle.longitude, handle.latitude)]));
  setGeoJson(style, GOTO_SOURCE, featureCollection(location === null ? [] : [pointFeature(location.at.longitude, location.at.latitude)]));
  const labelled = location !== null && location.loiterRadiusMetres !== null && location.loiterRadiusText.trim() !== '' ? location : null;
  const edge = labelled === null ? null : pointAt(labelled.at, labelled.loiterRadiusMetres ?? 0.0, 0.0);
  setGeoJson(style, GOTO_RADIUS_SOURCE, featureCollection(labelled === null || edge === null ? [] : [pointFeature(edge.longitude, edge.latitude, { [RADIUS_TEXT]: labelled.loiterRadiusText })]));
  setGeoJson(style, GOTO_ARROW_SOURCE, featureCollection(gotoArrows(location).map(([at, bearing]) => pointFeature(at.longitude, at.latitude, { [ARROW_BEARING]: bearing }))));
  const ring = gotoRing(location);
  setGeoJson(style, GOTO_RING_SOURCE, featureCollection(ring.length === 0 ? [] : [lineFeature(ring)]));
};

const CLICK_MARKER_SOURCE = 'aircast-click-marker';
const CLICK_MARKER_SHADOW_LAYER = 'aircast-click-marker-shadow';
const CLICK_MARKER_RING_LAYER = 'aircast-click-marker-ring';
const CLICK_MARKER_DOT_LAYER = 'aircast-click-marker-dot';
const CLICK_MARKER_RADIUS = 13;

export const installClickMarker = (style: MapLibreMap): void => {
  if (style.getSource(CLICK_MARKER_SOURCE) !== undefined) return;
  style.addSource(CLICK_MARKER_SOURCE, geoJsonSource());
  style.addLayer({
    id: CLICK_MARKER_SHADOW_LAYER,
    type: 'circle',
    source: CLICK_MARKER_SOURCE,
    paint: { 'circle-radius': CLICK_MARKER_RADIUS, 'circle-opacity': 0, 'circle-stroke-color': 'rgba(0, 0, 0, 0.6)', 'circle-stroke-width': 4 },
  });
  style.addLayer({
    id: CLICK_MARKER_RING_LAYER,
    type: 'circle',
    source: CLICK_MARKER_SOURCE,
    paint: { 'circle-radius': CLICK_MARKER_RADIUS - 1, 'circle-opacity': 0, 'circle-stroke-color': '#FFFFFF', 'circle-stroke-width': 2 },
  });
  style.addLayer({ id: CLICK_MARKER_DOT_LAYER, type: 'circle', source: CLICK_MARKER_SOURCE, paint: { 'circle-radius': 2, 'circle-color': '#FFFFFF' } });
};

export const clickMarkerFeatures = (at: TrackPoint | null): FeatureCollection =>
  featureCollection(at !== null && isPlottable(at.latitude, at.longitude) ? [pointFeature(at.longitude, at.latitude)] : []);

export const renderClickMarker = (style: MapLibreMap, at: TrackPoint | null): void => setGeoJson(style, CLICK_MARKER_SOURCE, clickMarkerFeatures(at));
