import { FenceBridge, type FenceCircle, type FencePolygon, type RallyPoint, rallyAltitudeFor } from './FenceBridge';
import type { MapHit } from './MissionEditing';
import { AT_END, HOME_ITEM } from './MissionKinds';
import { HOME_SEAL, LANDING_PLACE_APPROACH, draggedCircleRadius, draggedLoiterRadius, waypointColour, waypointLabel } from './MissionLayers';
import { KIND_STRUCTURE, type MissionItem, PlanBridge, moveLandingPlace } from './PlanBridge';
import { circleAround, polygonCentre, replaceShape, shapeMovedTo, shapeTarget, shapeVertices } from './PolygonTools';
import { sentenceCase } from './SentenceCase';
import { type Survey, SurveyBridge, type SurveyStats } from './SurveyBridge';
import { metresBetween } from './TerrainProfile';
import type { TrackPoint } from './VehicleMap';

export const NO_POSITION = 'no position';
export const AFTER_THE_ROUTE_ENDS = 'after the route ends';

export const itemCountText = (count: number): string => (count === 1 ? '1 item' : `${count} items`);

export type ItemRow = {
  index: number;
  number: string;
  name: string;
  detail: string;
  colour: string;
  placed: boolean;
  readyForSave: boolean;
  seal: string;
};

export const ItemRow = (fields: { index: number; number: string; name: string; detail: string; colour: string; placed: boolean; readyForSave?: boolean; seal?: string }): ItemRow => ({
  readyForSave: true,
  seal: fields.number,
  ...fields,
});

export const NOT_READY_SEAL = '?';

const blankToNull = (text: string): string | null => (text.trim() === '' ? null : text);

export const altitudeWithFrame = (item: MissionItem): string | null => {
  const height = blankToNull(item.altitudeText) ?? blankToNull(item.altitudeBandText);
  if (height === null) return null;
  const frame = item.altitudeFrameText;
  return frame.trim() === '' ? height : `${height} ${frame}`;
};

export const rallyAltitudeLabel = (point: RallyPoint): string => `Alt ${blankToNull(point.altitudeUnits) ?? 'm'}`;

export const rallyAltitudeIsEditable = (point: RallyPoint): boolean => !Number.isNaN(point.altitude) && point.altitudePath.trim() !== '';

export const itemSeal = (item: MissionItem): string =>
  item.index === HOME_ITEM ? HOME_SEAL : waypointLabel(item.sequence, false, item.complexPattern ? '' : item.abbreviation);

export const itemTitle = (item: MissionItem): string => {
  const name = sentenceCase(blankToNull(item.command) ?? 'Item');
  const seal = itemSeal(item);
  return /^\p{Nd}*$/u.test(seal) ? `${name} ${seal}` : name;
};

export const sequenceLabel = (item: MissionItem): string =>
  item.foldedCommands > 0 ? `${item.sequence}–${item.sequence + item.foldedCommands}` : item.sequence.toString();

export const surveyTiles = (item: MissionItem, stats: SurveyStats | null): readonly (readonly [string, string])[] =>
  stats === null
    ? []
    : ([
        ['AREA', stats.areaText],
        ['DISTANCE', item.kind !== KIND_STRUCTURE ? stats.distanceText : ''],
        ['PHOTOS', item.cameraShots > 0 ? item.cameraShots.toString() : ''],
        ['INTERVAL', stats.intervalText],
      ] as const).filter((it) => it[1].trim() !== '');

export const sheetDetail = (item: MissionItem, stats: SurveyStats | null): string =>
  surveyTiles(item, stats).length === 0 ? itemDetail(item, stats) : itemDetail({ ...item, cameraShots: 0 }, stats === null ? null : { ...stats, areaText: '' });

export const itemRows = (items: readonly MissionItem[], stats: ReadonlyMap<number, SurveyStats> = new Map()): readonly ItemRow[] =>
  items.map((item) =>
    ItemRow({
      index: item.index,
      number: item.readyForSave ? sequenceLabel(item) : NOT_READY_SEAL,
      name: blankToNull(item.command) ?? `Item ${item.sequence}`,
      detail: itemDetail(item, stats.get(item.index) ?? null),
      colour: waypointColour(item.kind, item.commandId),
      placed: item.placed,
      readyForSave: item.readyForSave,
      seal: item.readyForSave ? itemSeal(item) : NOT_READY_SEAL,
    }),
  );

export const itemDetail = (item: MissionItem, stats: SurveyStats | null = null): string =>
  [
    altitudeWithFrame(item) ?? (!item.placed && item.specifiesCoordinate ? NO_POSITION : null),
    item.afterRouteEnds ? AFTER_THE_ROUTE_ENDS : null,
    blankToNull(item.speedChangeText),
    stats === null ? null : blankToNull(stats.areaText),
    photosText(item.cameraShots),
    holdText(item.extraSeconds),
    blankToNull(item.blockedReason),
    stats === null ? null : blankToNull(stats.warning),
  ]
    .filter((it): it is string => it !== null)
    .join(' · ');

export const worthListing = (items: readonly MissionItem[]): boolean => items.some((it) => it.index !== HOME_ITEM);

export const rowAt = (rows: readonly ItemRow[], index: number): ItemRow | null => rows.find((it) => it.index === index) ?? null;

export const selectionAfterRemove = (removed: number, countBefore: number): MapHit | null => {
  const last = countBefore - 2;
  return last > HOME_ITEM ? { kind: 'Waypoint', index: Math.min(removed, last) } : null;
};

export const missionItemIndex = (selected: MapHit | null): number | null => {
  switch (selected?.kind) {
    case 'Waypoint':
    case 'LandingPlace':
    case 'LoiterRadius':
    case 'LoiterRotation':
      return selected.index;
    case 'SurveyVertex':
      return selected.item;
    case 'ShapeCentre':
    case 'ShapeRadius':
      return selected.fence ? null : selected.owner;
    default:
      return null;
  }
};

export const insertAfter = (selected: MapHit | null, items: readonly MissionItem[]): number => {
  const index = missionItemIndex(selected);
  if (index === null || !items.some((it) => it.index === index)) return AT_END;
  return index + 1 >= items.length ? AT_END : index + 1;
};

export const selectionSequence = (selected: MapHit | null, items: readonly MissionItem[]): number | null => {
  const index = missionItemIndex(selected);
  return index === null ? null : (items.find((it) => it.index === index)?.sequence ?? null);
};

export const addingAfterText = (selected: MapHit | null, items: readonly MissionItem[]): string | null => {
  const index = missionItemIndex(selected);
  const item = index === null ? undefined : items.find((it) => it.index === index);
  return item === undefined ? null : `Tap the map to add after #${item.sequence}`;
};

export const legText = (item: MissionItem): string | null => {
  const shown = ([
    ['Alt diff', item.altitudeChangeText],
    ['Azimuth', item.azimuthText],
    ['Heading', item.headingText],
    ['Gradient', item.gradientText],
    ['Prev WP', item.distanceText],
  ] as const).filter(([, value]) => value.trim() !== '');
  return shown.length === 0 ? null : shown.map(([label, value]) => `${label} ${value}`).join(' · ');
};

export const movedText = (hit: MapHit, items: readonly MissionItem[]): string => {
  switch (hit.kind) {
    case 'Waypoint': {
      const item = items.find((it) => it.index === hit.index);
      return item === undefined ? 'Moved an item' : `Moved #${item.sequence}`;
    }
    case 'FenceVertex': return 'Moved a fence corner';
    case 'SurveyVertex': return 'Moved a survey corner';
    case 'Rally': return 'Moved a rally point';
    case 'BreachReturn': return 'Moved the breach return point';
    case 'CircleCentre': return 'Moved a fence circle';
    case 'CircleRadius': return 'Changed a fence radius';
    case 'ShapeCentre': return hit.fence ? 'Moved a fence' : 'Moved a survey area';
    case 'ShapeRadius': return 'Changed the circle radius';
    case 'LoiterRadius': return 'Changed the loiter radius';
    case 'LoiterRotation': return 'Changed the loiter direction';
    case 'Circle': return 'Changed a fence radius';
    case 'Midpoint': return 'Added a corner';
    case 'LandingPlace': return hit.place === LANDING_PLACE_APPROACH ? 'Moved the final approach' : 'Moved the touchdown';
  }
};

const MoveWrites = { held: false, tail: Promise.resolve() as Promise<unknown>, committed: 0 };

const withMoveLock = <T,>(work: () => Promise<T>): Promise<T> => {
  const run = MoveWrites.tail.then(async () => {
    MoveWrites.held = true;
    try {
      return await work();
    } finally {
      MoveWrites.held = false;
    }
  });
  MoveWrites.tail = run.catch(() => undefined);
  return run;
};

export const moveGeneration = (): number => MoveWrites.committed;

export const writeDragStep = async (
  generation: number,
  hit: MapHit,
  latitude: number,
  longitude: number,
  surveys: readonly Survey[],
  rally: readonly RallyPoint[],
  fences: readonly FencePolygon[],
  items: readonly MissionItem[],
  circles: readonly FenceCircle[] = [],
): Promise<boolean> =>
  MoveWrites.held || withMoveLock(async () => generation !== MoveWrites.committed || applyMove(hit, latitude, longitude, surveys, rally, fences, items, circles));

export const writeMove = (
  hit: MapHit,
  latitude: number,
  longitude: number,
  surveys: readonly Survey[],
  rally: readonly RallyPoint[],
  fences: readonly FencePolygon[] = [],
  items: readonly MissionItem[] = [],
  circles: readonly FenceCircle[] = [],
): Promise<boolean> =>
  withMoveLock(() => {
    MoveWrites.committed += 1;
    return applyMove(hit, latitude, longitude, surveys, rally, fences, items, circles);
  });

const shapeOwnerTarget = (fence: boolean, owner: number, surveys: readonly Survey[]) =>
  fence ? shapeTarget(owner, null) : shapeTarget(null, surveys.find((it) => it.index === owner) ?? null);

const applyMove = async (
  hit: MapHit,
  latitude: number,
  longitude: number,
  surveys: readonly Survey[],
  rally: readonly RallyPoint[],
  fences: readonly FencePolygon[],
  items: readonly MissionItem[],
  circles: readonly FenceCircle[],
): Promise<boolean> => {
  const to: TrackPoint = { latitude, longitude };
  switch (hit.kind) {
    case 'Waypoint':
      return PlanBridge.moveItem(hit.index, latitude, longitude);
    case 'FenceVertex':
      return FenceBridge.adjustVertex(hit.polygon, hit.vertex, latitude, longitude);
    case 'SurveyVertex': {
      const survey = surveys.find((it) => it.index === hit.item);
      return survey !== undefined && (await SurveyBridge.adjustVertex(survey, hit.vertex, latitude, longitude));
    }
    case 'Rally':
      return FenceBridge.moveRallyPoint(hit.index, latitude, longitude, rallyAltitudeFor(rally, hit.index));
    case 'CircleCentre':
      return FenceBridge.moveCircle(hit.index, latitude, longitude);
    case 'CircleRadius': {
      const circle = circles.find((it) => it.index === hit.index);
      return circle !== undefined && (await FenceBridge.setCircleRadius(circle.index, draggedCircleRadius(circle, to)));
    }
    case 'ShapeCentre': {
      const target = shapeOwnerTarget(hit.fence, hit.owner, surveys);
      const vertices = hit.fence ? fences.find((it) => it.index === hit.owner)?.vertices : surveys.find((it) => it.index === hit.owner)?.area;
      const moved = vertices === undefined ? null : shapeMovedTo(vertices, to);
      return target !== null && moved !== null && (await replaceShape(target, moved));
    }
    case 'ShapeRadius': {
      const target = shapeOwnerTarget(hit.fence, hit.owner, surveys);
      const current = target === null ? null : shapeVertices(target, fences, surveys);
      const centre = current === null ? null : polygonCentre(current);
      const circle = current === null || centre === null ? null : circleAround(current, metresBetween(centre, to));
      return target !== null && circle !== null && (await replaceShape(target, circle));
    }
    case 'LoiterRadius': {
      const item = items.find((it) => it.index === hit.index);
      return item !== undefined && (await PlanBridge.setLoiterRadius(item.index, draggedLoiterRadius(item, to)));
    }
    case 'LoiterRotation':
      return false;
    case 'BreachReturn':
      return FenceBridge.setBreachReturn(to);
    case 'Circle':
      return true;
    case 'Midpoint':
      return false;
    case 'LandingPlace':
      return moveLandingPlace(hit.index, hit.place, latitude, longitude);
  }
};

export const patternName = (index: number, items: readonly MissionItem[]): string => blankToNull(items.find((it) => it.index === index)?.command ?? '') ?? 'pattern';

export const layersText = (survey: Survey | null): string | null => {
  const count = survey !== null && survey.layers > 1 ? survey.layers : null;
  if (survey === null || count === null) return null;
  const span = survey.layerSpanText;
  return span.trim() === '' ? `${count} layers, one drawn` : `${count} layers, ${span}, one drawn`;
};

export const cameraText = (stats: SurveyStats | null): string | null => {
  const parts = [
    stats === null || blankToNull(stats.surfaceDistanceText) === null ? null : `${stats.surfaceDistanceText} above the surface`,
    stats === null || blankToNull(stats.footprintText) === null ? null : `each shot covers ${stats.footprintText}`,
    stats === null || blankToNull(stats.intervalText) === null ? null : `a shot every ${stats.intervalText}`,
  ].filter((it): it is string => it !== null);
  return parts.length === 0 ? null : parts.join(' · ');
};

export const photosText = (shots: number): string | null => (shots <= 0 ? null : shots === 1 ? '1 photo' : `${shots} photos`);

export const holdText = (seconds: number): string | null => (Number.isNaN(seconds) || seconds <= 0.0 ? null : `holds ${Math.trunc(seconds)} s`);
