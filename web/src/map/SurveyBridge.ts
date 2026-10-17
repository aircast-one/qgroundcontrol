import { type Json, type JsonArray, isJson, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { coerceIn } from '@/kotlin/Std';
import { coordinateJson, invokeOk, setOk, settingJson } from './BridgeCalls';
import { type EditableShape, editableShape } from './EditableShape';
import { MapBridge } from './MapBridge';
import { type MissionItem, PLAN_ITEMS, planItems } from './PlanBridge';
import { type TrackPoint, isPlottable } from './VehicleMap';

export const MISSION_CONTROLLER = 'plan.missionController';

export const SHAPE_AREA = 'area';
export const SHAPE_LINE = 'line';

export type Survey = {
  index: number;
  area: readonly TrackPoint[];
  transects: readonly TrackPoint[];
  cameraShots: number;
  kind: string;
  shape: string;
  property: string;
  editable: EditableShape | null;
  flightLoop: readonly TrackPoint[];
  layers: number;
  layerSpanText: string;
  turnaround: boolean;
  outline: readonly TrackPoint[];
  collides: boolean;
};

export const Survey = (fields: {
  index: number;
  area: readonly TrackPoint[];
  transects: readonly TrackPoint[];
  cameraShots: number;
  kind: string;
  shape: string;
  property: string;
  editable?: EditableShape | null;
  flightLoop?: readonly TrackPoint[];
  layers?: number;
  layerSpanText?: string;
  turnaround?: boolean;
  outline?: readonly TrackPoint[];
  collides?: boolean;
}): Survey => ({ editable: null, flightLoop: [], layers: 0, layerSpanText: '', turnaround: false, outline: [], collides: false, ...fields });

const points = (array: JsonArray | null): readonly TrackPoint[] =>
  (array ?? []).flatMap((point) => {
    if (!isJson(point)) return [];
    const latitude = optDouble(point, 'latitude', NaN);
    const longitude = optDouble(point, 'longitude', NaN);
    return isPlottable(latitude, longitude) ? [{ latitude, longitude }] : [];
  });

export const GRID_ANGLE_MAX = 359;

export const gridAngleShown = (raw: number): number => (Number.isNaN(raw) ? 0 : coerceIn(Math.round(((raw % 360.0) + 360.0) % 360.0), 0, GRID_ANGLE_MAX));

const altitudePath = (itemIndex: number): string => `${PLAN_ITEMS}.${itemIndex}.cameraCalc.distanceToSurface`;

export const SurveyBridge = {
  removeVertex: (survey: Survey, vertex: number): Promise<boolean> => invokeOk(`${PLAN_ITEMS}.${survey.index}.${survey.property}.removeVertex`, `[${vertex}]`),

  surveysFrom: async (json: Json | null): Promise<readonly Survey[]> => {
    const items = planItems(json);
    if (items === null) return [];
    const found = items.flatMap((entry, index) => {
      if (!isJson(entry)) return [];
      const geometry = optJSONObject(entry, 'geometry');
      if (geometry === null) return [];
      const shape = optText(geometry, 'shape');
      const property = optText(geometry, 'property');
      if (shape.trim() === '' || property.trim() === '') return [];
      const area = points(optJSONArray(geometry, 'vertices'));
      const transects = points(optJSONArray(geometry, 'transects'));
      const flightLoop = points(optJSONArray(geometry, 'flightLoop'));
      if (transects.length === 0 && flightLoop.length === 0 && area.length === 0) return [];
      return [
        Survey({
          index,
          area,
          transects,
          flightLoop,
          layers: optInt(geometry, 'layers', 0),
          layerSpanText: optText(geometry, 'layerSpanText'),
          turnaround: optBoolean(geometry, 'turnaround'),
          outline: points(optJSONArray(geometry, 'outline')),
          cameraShots: optInt(entry, 'cameraShots'),
          kind: optText(entry, 'kind'),
          shape,
          property,
        }),
      ];
    });
    return Promise.all(found.map(async (survey) => ({ ...survey, editable: await editableShape(`${PLAN_ITEMS}.${survey.index}.${survey.property}`, survey.shape) })));
  },

  gridAngle: async (itemIndex: number): Promise<number> => optDouble(await MapBridge.read(`view.control(${PLAN_ITEMS}.${itemIndex}.gridAngle)`), 'value', NaN),

  altitude: async (itemIndex: number): Promise<number> => optDouble(await MapBridge.read(`view.control(${altitudePath(itemIndex)})`), 'value', NaN),

  altitudeUnits: async (itemIndex: number): Promise<string> => optText(await MapBridge.read(`view.control(${altitudePath(itemIndex)})`), 'units'),

  setAltitude: (itemIndex: number, shown: number): Promise<boolean> => setOk(altitudePath(itemIndex), settingJson(`${shown}`)),

  setGridAngle: (itemIndex: number, degrees: number): Promise<boolean> => setOk(`${PLAN_ITEMS}.${itemIndex}.gridAngle`, settingJson(`${degrees}`)),

  adjustVertex: (survey: Survey, vertex: number, latitude: number, longitude: number): Promise<boolean> =>
    invokeOk(`${PLAN_ITEMS}.${survey.index}.${survey.property}.adjustVertex`, `[${vertex}, ${coordinateJson(latitude, longitude)}]`),
};

export const ABSENT = '—';

export type StructureStats = { layers: string; layerHeight: string; top: string; bottom: string };

export type SurveyStats = {
  areaText: string;
  warning: string;
  intervalText: string;
  footprintText: string;
  surfaceDistanceText: string;
  distanceText: string;
  photosText: string;
  structure: StructureStats | null;
};

export const SurveyStats = (fields: {
  areaText: string;
  warning: string;
  intervalText?: string;
  footprintText?: string;
  surfaceDistanceText?: string;
  distanceText?: string;
  photosText?: string;
  structure?: StructureStats | null;
}): SurveyStats => ({ intervalText: '', footprintText: '', surfaceDistanceText: '', distanceText: '', photosText: '', structure: null, ...fields });

export const statisticsRows = (stats: SurveyStats): readonly (readonly [string, string])[] => [
  ...(stats.structure === null
    ? ([
        ['Area', stats.areaText],
        ['Distance', stats.distanceText],
      ] as const)
    : ([
        ['Layers', stats.structure.layers],
        ['Layer height', stats.structure.layerHeight],
        ['Top layer altitude', stats.structure.top],
        ['Bottom layer altitude', stats.structure.bottom],
      ] as const)),
  ['Photos', stats.photosText],
  ['Photo interval', stats.intervalText],
];

const stated = (view: Json, key: string): string => {
  const text = optText(view, key);
  return text === ABSENT ? '' : text;
};

export const surveyStats = (view: Json | null): SurveyStats | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const structure = optJSONObject(view, 'structure');
  return SurveyStats({
    areaText: stated(view, 'areaText'),
    warning: optText(view, 'warning'),
    intervalText: stated(view, 'intervalText'),
    footprintText: stated(view, 'footprintText'),
    surfaceDistanceText: stated(view, 'surfaceDistanceText'),
    distanceText: stated(view, 'distanceText'),
    photosText: optText(view, 'shotsText'),
    structure: structure === null ? null : { layers: optText(structure, 'layers'), layerHeight: optText(structure, 'layerHeight'), top: optText(structure, 'top'), bottom: optText(structure, 'bottom') },
  });
};

export const insetRing = (corners: readonly TrackPoint[], fraction: number): readonly TrackPoint[] => {
  if (corners.length < 3) return [];
  const midLatitude = corners.reduce((sum, it) => sum + it.latitude, 0) / corners.length;
  const midLongitude = corners.reduce((sum, it) => sum + it.longitude, 0) / corners.length;
  return corners.map((it) => ({ latitude: midLatitude + (it.latitude - midLatitude) * fraction, longitude: midLongitude + (it.longitude - midLongitude) * fraction }));
};

export const fitSurveyArea = async (survey: Survey, corners: readonly TrackPoint[]): Promise<boolean> => {
  if (corners.length === 0 || survey.area.length !== corners.length) return false;
  return corners.reduce<Promise<boolean>>(async (fitted, at, vertex) => (await fitted) && SurveyBridge.adjustVertex(survey, vertex, at.latitude, at.longitude), Promise.resolve(true));
};

export const surveyStatsFor = async (items: readonly MissionItem[]): Promise<ReadonlyMap<number, SurveyStats>> => {
  const read = await Promise.all(items.filter((it) => it.complexPattern).map(async (item) => [item.index, surveyStats(await MapBridge.read(`view.surveyStats(${item.index})`))] as const));
  return new Map(read.flatMap(([index, stats]) => (stats === null ? [] : [[index, stats] as const])));
};
