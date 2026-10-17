import type { LngLat, MapGeoJSONFeature, Map as MapLibreMap, MapMouseEvent, MapTouchEvent, PointLike } from 'maplibre-gl';
import {
  BREACH_LAYER,
  CIRCLE_INDEX_PROPERTY,
  FENCE_FILL_LAYER,
  FENCE_HANDLE_LAYER,
  HANDLE_KIND_CIRCLE,
  HANDLE_KIND_CIRCLE_RADIUS,
  HANDLE_KIND_FENCE,
  HANDLE_KIND_FENCE_CENTRE,
  HANDLE_KIND_FENCE_CIRCLE_RADIUS,
  HANDLE_KIND_LANDING,
  HANDLE_KIND_LOITER_RADIUS,
  HANDLE_KIND_LOITER_ROTATION,
  HANDLE_KIND_PROPERTY,
  HANDLE_KIND_SURVEY,
  HANDLE_KIND_SURVEY_CENTRE,
  MIDPOINT_LAYER,
  MISSION_DOT_LAYER,
  MISSION_LAYER,
  POLYGON_INDEX_PROPERTY,
  RALLY_INDEX_PROPERTY,
  RALLY_LAYER,
  SHAPE_PATH_PROPERTY,
  SPLIT_INVOKABLE_PROPERTY,
  VERTEX_INDEX_PROPERTY,
  WAYPOINT_EXIT_PROPERTY,
  WAYPOINT_ID_PROPERTY,
} from './MissionLayers';
import { setAllGesturesEnabled } from './VehicleMap';

const HIT_RADIUS_PX = 44;
const DRAG_WRITE_INTERVAL_MS = 120;
export const TAP_SLOP_PX = 20;
const LONG_PRESS_TIMEOUT_MS = 400;
const LONG_PRESS_ECHO_MS = 1000;

export type MapHit =
  | { readonly kind: 'Waypoint'; readonly index: number }
  | { readonly kind: 'FenceVertex'; readonly polygon: number; readonly vertex: number }
  | { readonly kind: 'SurveyVertex'; readonly item: number; readonly vertex: number }
  | { readonly kind: 'Rally'; readonly index: number }
  | { readonly kind: 'BreachReturn' }
  | { readonly kind: 'Circle'; readonly index: number }
  | { readonly kind: 'CircleCentre'; readonly index: number }
  | { readonly kind: 'CircleRadius'; readonly index: number }
  | { readonly kind: 'LandingPlace'; readonly index: number; readonly place: number }
  | { readonly kind: 'Midpoint'; readonly path: string; readonly invokable: string; readonly segment: number }
  | { readonly kind: 'ShapeCentre'; readonly fence: boolean; readonly owner: number }
  | { readonly kind: 'ShapeRadius'; readonly fence: boolean; readonly owner: number }
  | { readonly kind: 'LoiterRadius'; readonly index: number }
  | { readonly kind: 'LoiterRotation'; readonly index: number };

export const nearestIndex = (x: number, y: number, points: readonly (readonly [number, number] | null)[]): number | null =>
  points.reduce<{ index: number; distance: number } | null>((best, point, index) => {
    const distance = point === null ? Number.MAX_VALUE : (point[0] - x) ** 2 + (point[1] - y) ** 2;
    return best === null || distance < best.distance ? { index, distance } : best;
  }, null)?.index ?? null;

const nearest = (map: MapLibreMap, features: readonly MapGeoJSONFeature[], x: number, y: number): MapGeoJSONFeature | null => {
  const at = nearestIndex(
    x,
    y,
    features.map((feature) => {
      if (feature.geometry.type !== 'Point') return null;
      const [longitude = NaN, latitude = NaN] = feature.geometry.coordinates;
      const point = map.project([longitude, latitude]);
      return [point.x, point.y] as const;
    }),
  );
  return at === null ? null : (features[at] ?? null);
};

const numberProperty = (feature: MapGeoJSONFeature | null, key: string): number | null => {
  const value: unknown = feature?.properties[key];
  return typeof value === 'number' ? Math.trunc(value) : null;
};

const stringProperty = (feature: MapGeoJSONFeature | null, key: string): string | null => {
  const value: unknown = feature?.properties[key];
  return typeof value === 'string' ? value : null;
};

export const handleHit = (kind: string | null, owner: number, vertex: number): MapHit | null => {
  switch (kind) {
    case HANDLE_KIND_FENCE: return { kind: 'FenceVertex', polygon: owner, vertex };
    case HANDLE_KIND_SURVEY: return { kind: 'SurveyVertex', item: owner, vertex };
    case HANDLE_KIND_CIRCLE: return { kind: 'CircleCentre', index: owner };
    case HANDLE_KIND_FENCE_CIRCLE_RADIUS: return { kind: 'CircleRadius', index: owner };
    case HANDLE_KIND_LANDING: return { kind: 'LandingPlace', index: owner, place: vertex };
    case HANDLE_KIND_FENCE_CENTRE: return { kind: 'ShapeCentre', fence: true, owner };
    case HANDLE_KIND_SURVEY_CENTRE: return { kind: 'ShapeCentre', fence: false, owner };
    case HANDLE_KIND_CIRCLE_RADIUS: return { kind: 'ShapeRadius', fence: vertex === 0, owner };
    case HANDLE_KIND_LOITER_RADIUS: return { kind: 'LoiterRadius', index: owner };
    case HANDLE_KIND_LOITER_ROTATION: return { kind: 'LoiterRotation', index: owner };
    default: return null;
  }
};

const hitBox = (x: number, y: number): [PointLike, PointLike] => [
  [x - HIT_RADIUS_PX, y - HIT_RADIUS_PX],
  [x + HIT_RADIUS_PX, y + HIT_RADIUS_PX],
];

const rendered = (map: MapLibreMap, box: [PointLike, PointLike], ...layers: string[]): readonly MapGeoJSONFeature[] => {
  const present = layers.filter((layer) => map.getLayer(layer) !== undefined);
  return present.length === 0 ? [] : map.queryRenderedFeatures(box, { layers: present });
};

export const hitTest = (map: MapLibreMap, x: number, y: number): MapHit | null => {
  const box = hitBox(x, y);

  const midpoint = nearest(map, rendered(map, box, MIDPOINT_LAYER), x, y);
  const path = stringProperty(midpoint, SHAPE_PATH_PROPERTY);
  const invokable = stringProperty(midpoint, SPLIT_INVOKABLE_PROPERTY);
  const segment = numberProperty(midpoint, VERTEX_INDEX_PROPERTY);
  if (path !== null && path.trim() !== '' && invokable !== null && invokable.trim() !== '' && segment !== null) return { kind: 'Midpoint', path, invokable, segment };

  const handle = nearest(map, rendered(map, box, FENCE_HANDLE_LAYER), x, y);
  const owner = numberProperty(handle, POLYGON_INDEX_PROPERTY);
  const vertex = numberProperty(handle, VERTEX_INDEX_PROPERTY);
  const handled = owner === null || vertex === null ? null : handleHit(stringProperty(handle, HANDLE_KIND_PROPERTY), owner, vertex);
  if (handled !== null) return handled;

  if (rendered(map, box, BREACH_LAYER).length > 0) return { kind: 'BreachReturn' };

  const rally = numberProperty(nearest(map, rendered(map, box, RALLY_LAYER), x, y), RALLY_INDEX_PROPERTY);
  if (rally !== null) return { kind: 'Rally', index: rally };

  const circle = rendered(map, box, FENCE_FILL_LAYER)
    .map((it) => numberProperty(it, CIRCLE_INDEX_PROPERTY))
    .find((it) => it !== null);
  if (circle !== undefined && circle !== null) return { kind: 'Circle', index: circle };

  const waypoint = numberProperty(nearestMissionMarker(map, box, x, y), WAYPOINT_ID_PROPERTY);
  return waypoint === null ? null : { kind: 'Waypoint', index: waypoint };
};

const nearestMissionMarker = (map: MapLibreMap, box: [PointLike, PointLike], x: number, y: number): MapGeoJSONFeature | null => nearest(map, rendered(map, box, MISSION_DOT_LAYER, MISSION_LAYER), x, y);

const exitMarkerAt = (map: MapLibreMap, x: number, y: number): boolean => nearestMissionMarker(map, hitBox(x, y), x, y)?.properties[WAYPOINT_EXIT_PROPERTY] === true;

export const withinHit = (dx: number, dy: number): boolean => Math.abs(dx) <= HIT_RADIUS_PX && Math.abs(dy) <= HIT_RADIUS_PX;

export const withinTap = (dx: number, dy: number): boolean => Math.abs(dx) <= TAP_SLOP_PX && Math.abs(dy) <= TAP_SLOP_PX;

export const attachMissionEditing = (
  _mapView: HTMLElement,
  map: MapLibreMap,
  onAdd: (latitude: number, longitude: number) => void,
  onMove: (hit: MapHit, latitude: number, longitude: number) => void,
  onSelected: (hit: MapHit | null) => void = () => undefined,
  onMoved: (hit: MapHit, latitude: number, longitude: number) => void = () => undefined,
  onDragging: (hit: MapHit | null) => void = () => undefined,
  canDrag: (hit: MapHit) => boolean = () => true,
  onBlankTap: (latitude: number, longitude: number) => void = () => onSelected(null),
): (() => void) => {
  const gesture = {
    active: false,
    dragging: null as MapHit | null,
    tapped: null as MapHit | null,
    downX: 0,
    downY: 0,
    moved: false,
    lastWriteAt: 0,
    addedInGesture: false,
    swallowsClick: false,
    press: null as ReturnType<typeof setTimeout> | null,
    pressedAt: -LONG_PRESS_ECHO_MS,
  };
  const tappedOnly = (hit: MapHit | null, x: number, y: number): MapHit | null => (hit !== null && (!canDrag(hit) || (hit.kind === 'Waypoint' && exitMarkerAt(map, x, y))) ? hit : null);
  const cancelPress = () => {
    if (gesture.press !== null) clearTimeout(gesture.press);
    gesture.press = null;
  };
  const longClick = (tapped: MapHit | null, at: LngLat) => {
    gesture.addedInGesture = true;
    gesture.swallowsClick = true;
    gesture.pressedAt = performance.now();
    if (tapped !== null) onSelected(tapped);
    else onAdd(at.lat, at.lng);
  };
  const down = (event: MapMouseEvent | MapTouchEvent) => {
    cancelPress();
    if (event.type === 'touchstart' && (event as MapTouchEvent).points.length !== 1) return;
    if (event.type === 'mousedown' && (event as MapMouseEvent).originalEvent.button !== 0) return;
    const { x, y } = event.point;
    const hit = hitTest(map, x, y);
    gesture.active = true;
    gesture.downX = x;
    gesture.downY = y;
    gesture.moved = false;
    gesture.addedInGesture = false;
    gesture.swallowsClick = false;
    gesture.tapped = tappedOnly(hit, x, y);
    if (hit === null || gesture.tapped !== null) {
      setAllGesturesEnabled(map, true);
      const at = event.lngLat;
      const tapped = gesture.tapped;
      if (event.type === 'touchstart') gesture.press = setTimeout(() => {
        gesture.press = null;
        longClick(tapped, at);
      }, LONG_PRESS_TIMEOUT_MS);
      return;
    }
    gesture.dragging = hit;
    setAllGesturesEnabled(map, false);
    event.preventDefault();
  };
  const move = (event: MapMouseEvent | MapTouchEvent) => {
    const hit = gesture.dragging;
    const still = withinTap(event.point.x - gesture.downX, event.point.y - gesture.downY);
    if (hit === null) {
      if (!still) cancelPress();
      return;
    }
    if (hit.kind === 'Midpoint') return;
    if (!gesture.moved && !still) {
      gesture.moved = true;
      onDragging(hit);
    }
    const now = performance.now();
    if (gesture.moved && now - gesture.lastWriteAt >= DRAG_WRITE_INTERVAL_MS) {
      gesture.lastWriteAt = now;
      onMove(hit, event.lngLat.lat, event.lngLat.lng);
    }
  };
  const release = (x: number, y: number, at: LngLat, up: boolean) => {
    if (!gesture.active) return;
    gesture.active = false;
    cancelPress();
    const hit = gesture.dragging;
    gesture.dragging = null;
    setAllGesturesEnabled(map, true);
    if (gesture.moved) onDragging(null);
    const tap = gesture.tapped;
    gesture.tapped = null;
    const still = withinTap(x - gesture.downX, y - gesture.downY);
    if (hit === null) {
      if (tap !== null && !gesture.addedInGesture && up && still) onSelected(tap);
      return;
    }
    gesture.swallowsClick = true;
    if (gesture.moved) onMoved(hit, at.lat, at.lng);
    else if (hit.kind !== 'Midpoint' || still) onSelected(hit);
  };
  const lifted = (event: MapMouseEvent | MapTouchEvent) => release(event.point.x, event.point.y, event.lngLat, true);
  const cancelled = (event: MapTouchEvent) => release(event.point.x, event.point.y, event.lngLat, false);
  const outside = (event: MouseEvent) => {
    if (!gesture.active) return;
    const box = map.getCanvasContainer().getBoundingClientRect();
    const x = event.clientX - box.left;
    const y = event.clientY - box.top;
    release(x, y, map.unproject([x, y]), true);
  };
  const clicked = (event: MapMouseEvent) => {
    if (gesture.swallowsClick) {
      gesture.swallowsClick = false;
      return;
    }
    if (hitTest(map, event.point.x, event.point.y) !== null) return;
    event.preventDefault();
    onBlankTap(event.lngLat.lat, event.lngLat.lng);
  };
  const menu = (event: MapMouseEvent) => {
    event.originalEvent.preventDefault();
    if (performance.now() - gesture.pressedAt < LONG_PRESS_ECHO_MS) return;
    const { x, y } = event.point;
    const hit = hitTest(map, x, y);
    const tapped = tappedOnly(hit, x, y);
    if (hit !== null && tapped === null) return;
    longClick(tapped, event.lngLat);
  };
  map.on('mousedown', down);
  map.on('touchstart', down);
  map.on('mousemove', move);
  map.on('touchmove', move);
  map.on('mouseup', lifted);
  map.on('touchend', lifted);
  map.on('touchcancel', cancelled);
  map.on('click', clicked);
  map.on('contextmenu', menu);
  window.addEventListener('mouseup', outside);
  return () => {
    cancelPress();
    map.off('mousedown', down);
    map.off('touchstart', down);
    map.off('mousemove', move);
    map.off('touchmove', move);
    map.off('mouseup', lifted);
    map.off('touchend', lifted);
    map.off('touchcancel', cancelled);
    map.off('click', clicked);
    map.off('contextmenu', menu);
    window.removeEventListener('mouseup', outside);
  };
};
