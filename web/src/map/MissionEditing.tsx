import type { Map as MapLibreMap } from 'maplibre-gl';

const HIT_RADIUS_PX = 44;
export const TAP_SLOP_PX = 20;

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

export const withinHit = (dx: number, dy: number): boolean => Math.abs(dx) <= HIT_RADIUS_PX && Math.abs(dy) <= HIT_RADIUS_PX;

export const withinTap = (dx: number, dy: number): boolean => Math.abs(dx) <= TAP_SLOP_PX && Math.abs(dy) <= TAP_SLOP_PX;

export const hitTest = (_map: MapLibreMap, _x: number, _y: number): MapHit | null => null;

export const attachMissionEditing = (
  _mapView: HTMLElement,
  _map: MapLibreMap,
  _onAdd: (latitude: number, longitude: number) => void,
  _onMove: (hit: MapHit, latitude: number, longitude: number) => void,
  _onSelected: (hit: MapHit | null) => void = () => undefined,
  _onMoved: (hit: MapHit, latitude: number, longitude: number) => void = () => undefined,
  _onDragging: (hit: MapHit | null) => void = () => undefined,
  _canDrag: (hit: MapHit) => boolean = () => true,
  _onBlankTap?: (latitude: number, longitude: number) => void,
): void => undefined;
