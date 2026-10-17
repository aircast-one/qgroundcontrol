import type { Feature } from 'geojson';
import type { Map as MapLibreMap } from 'maplibre-gl';
import { type Json, jsonObjects, optDouble, optJSONArray, optText } from '@/bridge/Json';
import { typedNumber } from './AltitudeEntry';
import { coordinateJsonAt, invokeOk } from './BridgeCalls';
import type { EditableShape } from './EditableShape';
import { type FencePolygon, FENCE_POLYGONS, azimuthBetween, circleRing, pointAt } from './FenceBridge';
import { MapBridge } from './MapBridge';
import type { MapHit } from './MissionEditing';
import { PLAN_ITEMS } from './PlanBridge';
import type { Survey } from './SurveyBridge';
import { metresBetween } from './TerrainProfile';
import { type TrackPoint, featureCollection, geoJsonSource, lineFeature, setGeoJson } from './VehicleMap';
import { stageFile, unstageAll } from '@/core/files';

const DEFAULT_SHAPE_FRACTION = 0.75;
const DEFAULT_SHAPE_MAX_METRES = 3000.0;
const DEFAULT_CIRCLE_SEGMENTS = 16;
const METRES_PER_DEGREE = 111_320.0;
const TRACE_SOURCE = 'aircast-polygon-trace';
const TRACE_LINE_LAYER = 'aircast-polygon-trace-line';
const TRACE_DOT_LAYER = 'aircast-polygon-trace-dots';
const TRACE_COLOUR = '#FFFFFF';
const SHAPE_FOLDER = 'cache';
const SHAPE_PREFIX = 'shape.';
const UNREADABLE = 'That file could not be read.';

const offset = (centre: TrackPoint, northMetres: number, eastMetres: number): TrackPoint => ({
  latitude: centre.latitude + northMetres / METRES_PER_DEGREE,
  longitude: centre.longitude + eastMetres / (METRES_PER_DEGREE * Math.cos((centre.latitude * Math.PI) / 180)),
});

const halfExtents = (view: readonly TrackPoint[]): readonly [number, number] | null => {
  const [topLeft, topRight, , bottomLeft] = view;
  if (view.length !== 4 || topLeft === undefined || topRight === undefined || bottomLeft === undefined) return null;
  const half = (metres: number) => Math.min(metres * DEFAULT_SHAPE_FRACTION, DEFAULT_SHAPE_MAX_METRES) / 2;
  return [half(metresBetween(topLeft, topRight)), half(metresBetween(topLeft, bottomLeft))];
};

const viewCentre = (view: readonly TrackPoint[]): TrackPoint => ({
  latitude: view.reduce((sum, it) => sum + it.latitude, 0) / view.length,
  longitude: view.reduce((sum, it) => sum + it.longitude, 0) / view.length,
});

export const defaultRectangle = (view: readonly TrackPoint[]): readonly TrackPoint[] => {
  const extents = halfExtents(view);
  if (extents === null) return [];
  const [halfWidth, halfHeight] = extents;
  const centre = viewCentre(view);
  return (
    [
      [halfHeight, -halfWidth],
      [halfHeight, halfWidth],
      [-halfHeight, halfWidth],
      [-halfHeight, -halfWidth],
    ] as const
  ).map(([north, east]) => offset(centre, north, east));
};

export const defaultCircle = (view: readonly TrackPoint[]): readonly TrackPoint[] => {
  const extents = halfExtents(view);
  return extents === null ? [] : circleRing(viewCentre(view), Math.min(extents[0], extents[1]), DEFAULT_CIRCLE_SEGMENTS);
};

export type ShapeTarget = { readonly path: string; readonly line: boolean; readonly minimum: number; readonly fileShape: string; readonly missing: string };

export const ShapeTarget = (fields: { path: string; line: boolean }): ShapeTarget => ({
  ...fields,
  minimum: fields.line ? 2 : 3,
  fileShape: fields.line ? 'polyline' : 'polygon',
  missing: fields.line ? 'No polylines found in file' : 'No polygons found in file',
});

export const shapeTarget = (fence: number | null, survey: Survey | null): ShapeTarget | null =>
  fence !== null
    ? ShapeTarget({ path: `${FENCE_POLYGONS}.${fence}`, line: false })
    : survey !== null
      ? ShapeTarget({ path: `${PLAN_ITEMS}.${survey.index}.${survey.property}`, line: survey.property === CORRIDOR_PROPERTY })
      : null;

export const defaultLine = (view: readonly TrackPoint[]): readonly TrackPoint[] => {
  const [topLeft, topRight, bottomRight, bottomLeft] = view;
  if (view.length !== 4 || topLeft === undefined || topRight === undefined || bottomRight === undefined || bottomLeft === undefined) return [];
  const top = { latitude: (topLeft.latitude + topRight.latitude) / 2, longitude: (topLeft.longitude + topRight.longitude) / 2 };
  const bottom = { latitude: (bottomLeft.latitude + bottomRight.latitude) / 2, longitude: (bottomLeft.longitude + bottomRight.longitude) / 2 };
  return [0.25, 0.75].map((fraction) => ({
    latitude: top.latitude + (bottom.latitude - top.latitude) * fraction,
    longitude: top.longitude + (bottom.longitude - top.longitude) * fraction,
  }));
};

export const polygonCentre = (vertices: readonly TrackPoint[]): TrackPoint | null => {
  const origin = vertices[0];
  if (vertices.length < 3 || origin === undefined) return null;
  const local = vertices.map((it) => [it.longitude - origin.longitude, it.latitude - origin.latitude] as const);
  const edges = local.map((from, index) => [from, local[(index + 1) % local.length] ?? from] as const);
  const cross = edges.map(([a, b]) => a[0] * b[1] - b[0] * a[1]);
  const area = cross.reduce((sum, it) => sum + it, 0) / 2;
  if (area === 0.0) return null;
  const x = edges.reduce((sum, [a, b], index) => sum + (a[0] + b[0]) * (cross[index] ?? 0), 0) / (6 * area);
  const y = edges.reduce((sum, [a, b], index) => sum + (a[1] + b[1]) * (cross[index] ?? 0), 0) / (6 * area);
  return { latitude: origin.latitude + y, longitude: origin.longitude + x };
};

export const shapeMovedTo = (vertices: readonly TrackPoint[], centre: TrackPoint): readonly TrackPoint[] | null => {
  const from = polygonCentre(vertices);
  if (from === null) return null;
  const distance = metresBetween(from, centre);
  const azimuth = azimuthBetween(from, centre);
  return vertices.map((it) => pointAt(it, distance, azimuth));
};

export const fencePath = (index: number): string => `${FENCE_POLYGONS}.${index}`;

export const surveyPath = (survey: Survey): string => `${PLAN_ITEMS}.${survey.index}.${survey.property}`;

export const circleRadius = (vertices: readonly TrackPoint[]): number | null => {
  const centre = polygonCentre(vertices);
  const first = vertices[0];
  return centre === null || first === undefined ? null : metresBetween(centre, first);
};

export const circleAround = (vertices: readonly TrackPoint[], radius: number): readonly TrackPoint[] | null => {
  const centre = polygonCentre(vertices);
  return centre !== null && radius > 0 ? circleRing(centre, radius, DEFAULT_CIRCLE_SEGMENTS) : null;
};

export const shapeVertices = (target: ShapeTarget, fences: readonly FencePolygon[], surveys: readonly Survey[]): readonly TrackPoint[] =>
  fences.find((it) => fencePath(it.index) === target.path)?.vertices ?? surveys.find((it) => surveyPath(it) === target.path)?.area ?? [];

export const shapeCentreHit = (target: ShapeTarget, fences: readonly FencePolygon[], surveys: readonly Survey[]): readonly [MapHit, TrackPoint] | null => {
  const centre = polygonCentre(shapeVertices(target, fences, surveys));
  if (centre === null) return null;
  const fence = fences.find((it) => fencePath(it.index) === target.path);
  const survey = surveys.find((it) => surveyPath(it) === target.path);
  const hit: MapHit | null =
    fence !== undefined ? { kind: 'ShapeCentre', fence: true, owner: fence.index } : survey !== undefined ? { kind: 'ShapeCentre', fence: false, owner: survey.index } : null;
  return hit === null ? null : [hit, centre];
};

export const positionTitle = (hit: MapHit): string => (hit.kind === 'ShapeCentre' ? 'Edit center position' : 'Edit vertex position');

export const shapeEditable = (target: ShapeTarget, fences: readonly FencePolygon[], surveys: readonly Survey[]): EditableShape | null =>
  fences.find((it) => fencePath(it.index) === target.path)?.editable ?? surveys.find((it) => surveyPath(it) === target.path)?.editable ?? null;

export const shapeCaption = (shape: EditableShape, circled: boolean): string => (circled && shape.circleCaption.trim() !== '' ? shape.circleCaption : shape.caption);

export const traceCaption = (count: number, minimum: number): string => (count >= minimum ? `${count} points` : `Click the map to add points · ${count} of ${minimum}`);

export const circleRadiusMetres = (typed: string, metresPerUnit: number): number | null => {
  const value = typedNumber(typed);
  return value !== null && value > 0 ? value * metresPerUnit : null;
};

const CIRCLE_TOLERANCE = 0.01;

export const isCircleShape = (vertices: readonly TrackPoint[]): boolean => {
  const centre = polygonCentre(vertices);
  const radius = circleRadius(vertices);
  if (centre === null || radius === null) return false;
  return vertices.length === DEFAULT_CIRCLE_SEGMENTS && radius > 0 && vertices.every((it) => Math.abs(metresBetween(centre, it) - radius) <= radius * CIRCLE_TOLERANCE);
};

export const liveCircles = (circled: ReadonlySet<string>, fences: readonly FencePolygon[], surveys: readonly Survey[]): ReadonlySet<string> =>
  new Set([...circled].filter((path) => isCircleShape(shapeVertices(ShapeTarget({ path, line: false }), fences, surveys))));

export const replaceShape = async (target: ShapeTarget, vertices: readonly TrackPoint[]): Promise<boolean> =>
  vertices.length >= target.minimum && (await invokeOk(`${target.path}.clear`)) && invokeOk(`${target.path}.appendVertices`, `[[${vertices.map(coordinateJsonAt).join(',')}]]`);

export const CORRIDOR_PROPERTY = 'corridorPolyline';

export const fileShape = (view: Json | null, target: ShapeTarget): readonly [readonly TrackPoint[], string] => {
  if (view === null) return [[], target.missing];
  if (optText(view, 'error').trim() !== '') return [[], optText(view, 'error')];
  if (optText(view, 'shape') !== target.fileShape) return [[], target.missing];
  return [jsonObjects(optJSONArray(view, 'points')).map((it) => ({ latitude: optDouble(it, 'latitude'), longitude: optDouble(it, 'longitude') })), ''];
};

export const extensionOf = (name: string): string => {
  const dot = name.lastIndexOf('.');
  return dot < 0 ? '' : name.slice(dot + 1).toLowerCase();
};

export const mainShapeExtension = (names: readonly string[]): string | null => {
  const extensions = names.map(extensionOf);
  return ['shp', 'kml'].find((it) => extensions.includes(it)) ?? extensions[0] ?? null;
};

const stagedShapes = async (named: readonly (readonly [File, string])[]): Promise<boolean> => {
  await unstageAll(`/${SHAPE_FOLDER}/${SHAPE_PREFIX}`);
  return named.reduce<Promise<boolean>>(async (copied, [file, extension]) => (await copied) && stageFile(`/${SHAPE_FOLDER}/${SHAPE_PREFIX}${extension}`, file), Promise.resolve(true));
};

export const importShapeFiles = async (files: readonly File[], target: ShapeTarget): Promise<string | null> => {
  const named = files.map((file) => [file, extensionOf(file.name)] as const);
  const extension = mainShapeExtension(named.map(([, it]) => `x.${it}`));
  if (extension === null) return UNREADABLE;
  const copied = await stagedShapes(named);
  const staged = `/${SHAPE_FOLDER}/${SHAPE_PREFIX}${extension}`;
  const view = `view.${target.line ? 'lineFile' : 'areaFile'}(${staged})`;
  const [vertices, error] = copied ? fileShape(await MapBridge.read(view), target) : ([[], UNREADABLE] as const);
  if (error.trim() !== '') return error;
  return (await replaceShape(target, vertices)) ? null : target.missing;
};

export const traceOutline = (points: readonly TrackPoint[], line = false): readonly TrackPoint[] => {
  const first = points[0];
  return !line && points.length >= 3 && first !== undefined ? [...points, first] : points;
};

export const installTraceLayer = (style: MapLibreMap): void => {
  if (style.getSource(TRACE_SOURCE) !== undefined) return;
  style.addSource(TRACE_SOURCE, geoJsonSource());
  style.addLayer({ id: TRACE_LINE_LAYER, type: 'line', source: TRACE_SOURCE, paint: { 'line-color': TRACE_COLOUR, 'line-width': 2 } });
  style.addLayer({ id: TRACE_DOT_LAYER, type: 'circle', source: TRACE_SOURCE, paint: { 'circle-color': TRACE_COLOUR, 'circle-radius': 4 } });
};

export const renderTrace = (style: MapLibreMap, points: readonly TrackPoint[], line: boolean): void => {
  const outline = traceOutline(points, line);
  const dots: Feature = { type: 'Feature', geometry: { type: 'MultiPoint', coordinates: outline.map((it) => [it.longitude, it.latitude]) }, properties: {} };
  setGeoJson(style, TRACE_SOURCE, featureCollection([...(outline.length >= 2 ? [lineFeature(outline)] : []), ...(outline.length > 0 ? [dots] : [])]));
};
