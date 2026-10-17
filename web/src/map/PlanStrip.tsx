import type { MouseEvent, PointerEvent } from 'react';
import type { Json } from '@/bridge/Json';
import { format } from '@/kotlin/Std';
import { Button, IconButton, Text, TextButton } from '@/m3';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { invokeOk } from './BridgeCalls';
import type { MapHit } from './MissionEditing';
import { HOME_ITEM } from './MissionKinds';
import { summaryRow } from './MissionSummary';
import type { MissionItem } from './PlanBridge';
import { type ItemRow, legText, patternName, sequenceLabel } from './PlanItems';
import { sentenceCase } from './SentenceCase';
import type { Survey } from './SurveyBridge';
import { metresBetween } from './TerrainProfile';
import type { TrackPoint } from './VehicleMap';
import { NEWEST_PATTERN } from './VehicleSync';
import { WaypointStrip } from './WaypointPanel';
import { TAP, TAP_LOWER } from '@/kotlin/Platform';

export type PlanStat = { label: string; value: string };

export const planStats = (itemCount: number, items: readonly MissionItem[], summary: Json | null): readonly PlanStat[] => {
  const distance = summaryRow(summary, 'Distance');
  const time = summaryRow(summary, 'Time');
  const highest = highestAltitude(items);
  return [
    { label: 'Items', value: itemCount.toString() },
    ...(distance === null ? [] : [{ label: 'Distance', value: distance }]),
    ...(time === null ? [] : [{ label: 'Time', value: time.startsWith(NO_HOURS) ? time.slice(NO_HOURS.length) : time }]),
    ...(highest === null ? [] : [{ label: 'Max alt', value: highest }]),
  ];
};

export const highestAltitude = (items: readonly MissionItem[]): string | null =>
  items
    .filter((it) => it.index !== HOME_ITEM && !Number.isNaN(it.altitude) && it.altitudeText.trim() !== '')
    .reduce<MissionItem | null>((best, it) => (best === null || it.altitude > best.altitude ? it : best), null)?.altitudeText ?? null;

const NO_HOURS = '00:';
export const TAP_TO_ADD = `${TAP} the map to add a waypoint`;
export const TERRAIN_CONFLICT_HERE = 'Too close to the terrain here';
const PANEL_DRAG_THRESHOLD = 24;

export const advancedDetail = (item: MissionItem): string | null => {
  const text = [item.foldedCommands > 0 ? `Mission items ${sequenceLabel(item)}` : null, legText(item)].filter((it): it is string => it !== null).join(' · ');
  return text.trim() === '' ? null : text;
};

export const placedPattern = (surveys: readonly Survey[], wanted: number): Survey | null =>
  surveys.find((it) => it.index === wanted) ??
  (wanted === NEWEST_PATTERN ? surveys.reduce<Survey | null>((best, it) => (best === null || it.index > best.index ? it : best), null) : null);

export const appendSequence = (items: readonly MissionItem[]): number =>
  items.reduce<MissionItem | null>((best, it) => (best === null || it.index > best.index ? it : best), null)?.sequence ?? HOME_ITEM;

const FAR_FROM_AIRCRAFT_METRES = 1000.0;
const METRES_PER_MILE = 1609.344;
const FEET_PER_METRE = 3.28084;
const SHORT_MILES = 0.2;
const WHOLE_UNITS = 10.0;
const METRES_PER_KILOMETRE = 1000.0;
export const REPOSITION_MISSION = 'plan.missionController.repositionMission';

export const routeStart = (items: readonly MissionItem[]): TrackPoint | null => {
  const first =
    items.filter((it) => it.placed && it.index !== HOME_ITEM).reduce<MissionItem | null>((best, it) => (best === null || it.index < best.index ? it : best), null) ??
    items.find((it) => it.placed && it.index === HOME_ITEM) ??
    null;
  return first === null ? null : { latitude: first.latitude, longitude: first.longitude };
};

export const farFromAircraft = (aircraft: TrackPoint | null, items: readonly MissionItem[]): number | null => {
  const start = routeStart(items);
  if (start === null || aircraft === null) return null;
  const metres = metresBetween(aircraft, start);
  return metres > FAR_FROM_AIRCRAFT_METRES ? metres : null;
};

export const distanceWords = (metres: number, imperial: boolean): string => {
  const miles = metres / METRES_PER_MILE;
  const kilometres = metres / METRES_PER_KILOMETRE;
  if (imperial && miles < SHORT_MILES) return format('%,d ft', Math.round(metres * FEET_PER_METRE));
  if (imperial && miles < WHOLE_UNITS) return format('%.1f mi', miles);
  if (imperial) return format('%,d mi', Math.round(miles));
  if (metres < METRES_PER_KILOMETRE) return format('%d m', Math.round(metres));
  if (kilometres < WHOLE_UNITS) return format('%.1f km', kilometres);
  return format('%,d km', Math.round(kilometres));
};

export const startsFromAircraft = (metres: number, imperial: boolean): string => `Starts ${distanceWords(metres, imperial)} from the aircraft`;

export const repositionMission = (at: TrackPoint): Promise<boolean> =>
  invokeOk(REPOSITION_MISSION, `[{"latitude":${at.latitude},"longitude":${at.longitude}},true,true]`);

export const tapCloses = (selected: MapHit | null, panelOpen: boolean): boolean => selected !== null && (panelOpen || selected.kind !== 'Waypoint');

export const terrainWarning = (legs: number, items: number): string | null =>
  legs > 0 ? `${legs} ${legs === 1 ? 'leg hits' : 'legs hit'} the terrain` : items > 0 ? `${items} ${items === 1 ? 'item hits' : 'items hit'} the terrain` : null;

type PanelDrag = { start: number; dragged: boolean };

const panelDrags = new WeakMap<Element, PanelDrag>();

export const panelDrag = (onOpen: (open: boolean) => void) => ({
  style: { touchAction: 'pan-x' } as const,
  onPointerDown: (event: PointerEvent<HTMLElement>) => {
    panelDrags.set(event.currentTarget, { start: event.clientY, dragged: false });
  },
  onPointerMove: (event: PointerEvent<HTMLElement>) => {
    const drag = panelDrags.get(event.currentTarget);
    if (drag === undefined || event.currentTarget.hasPointerCapture(event.pointerId) || Math.abs(event.clientY - drag.start) <= PANEL_DRAG_THRESHOLD) return;
    event.currentTarget.setPointerCapture(event.pointerId);
  },
  onPointerUp: (event: PointerEvent<HTMLElement>) => {
    const drag = panelDrags.get(event.currentTarget);
    if (drag === undefined) return;
    const travelled = event.clientY - drag.start;
    const open = travelled < -PANEL_DRAG_THRESHOLD ? true : travelled > PANEL_DRAG_THRESHOLD ? false : null;
    panelDrags.set(event.currentTarget, { start: drag.start, dragged: open !== null });    if (open !== null) onOpen(open);
  },
  onPointerCancel: (event: PointerEvent<HTMLElement>) => {
    panelDrags.delete(event.currentTarget);
  },
  onClickCapture: (event: MouseEvent<HTMLElement>) => {
    const dragged = panelDrags.get(event.currentTarget)?.dragged === true;
    panelDrags.delete(event.currentTarget);
    if (!dragged) return;
    event.stopPropagation();
    event.preventDefault();
  },
});

export const FIRST_TAP_SETS_HOME = `The first ${TAP_LOWER} also sets home.`;

export const selectionTitle = (selected: MapHit, items: readonly MissionItem[]): string => {
  switch (selected.kind) {
    case 'FenceVertex': return 'Fence corner';
    case 'Circle':
    case 'CircleCentre':
    case 'CircleRadius': return 'Circular fence';
    case 'ShapeCentre':
    case 'ShapeRadius': return selected.fence ? 'Fence' : sentenceCase(patternName(selected.owner, items));
    case 'SurveyVertex': return `${sentenceCase(patternName(selected.item, items))} corner`;
    case 'Rally': return `Rally point ${selected.index + 1}`;
    case 'BreachReturn': return 'Breach return point';
    case 'LandingPlace': return 'Landing';
    default: return 'Selected';
  }
};

export const WaypointStripBar = ({
  rows,
  altitudes,
  conflicts,
  selected,
  onPick,
  onList,
  profileShown,
  onProfile,
}: {
  rows: readonly ItemRow[];
  altitudes: ReadonlyMap<number, string>;
  conflicts: ReadonlySet<number>;
  selected: number | null;
  onPick: (index: number) => void;
  onList: (() => void) | null;
  profileShown: boolean | null;
  onProfile: () => void;
}) => (
  <div className="flex w-full items-center pb-2">
    <WaypointStrip rows={rows} altitudes={altitudes} conflicts={conflicts} selected={selected} onPick={onPick} className="min-w-0 flex-1" />
    {onList === null ? null : (
      <IconButton onClick={onList} label="Show the plan as a list">
        <MaterialIcon name="list" filled />
      </IconButton>
    )}
    {profileShown === null ? null : (
      <IconButton onClick={onProfile} label={profileShown ? 'Hide the terrain profile' : 'Show the terrain profile'}>
        <Drawable name="plan_terrain" style={{ color: profileShown ? md('primary') : md('onSurfaceVariant') }} />
      </IconButton>
    )}
  </div>
);

export const EmptyMissionStrip = ({ homeSet, onTemplates, onDownload }: { homeSet: boolean; onTemplates: (() => void) | null; onDownload: (() => void) | null }) => (
  <div className="flex w-full flex-col px-3 pb-1">
    <Text typography="titleMedium">{TAP_TO_ADD}</Text>
    {homeSet ? null : (
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {FIRST_TAP_SETS_HOME}
      </Text>
    )}
    {onTemplates !== null || onDownload !== null ? (
      <div className="flex gap-1">
        {onTemplates === null ? null : <TextButton onClick={onTemplates}>Templates</TextButton>}
        {onDownload === null ? null : <TextButton onClick={onDownload}>Download from vehicle</TextButton>}
      </div>
    ) : null}
  </div>
);

export const SelectionHeader = ({
  title,
  detail,
  warning,
  open,
  onTitle,
  onDone,
}: {
  title: string;
  detail: string | null;
  warning: boolean;
  open: boolean;
  onTitle: () => void;
  onDone: () => void;
}) => (
  <div className="flex w-full items-center pb-2">
    <button type="button" onClick={onTitle} title={open ? 'Fold the settings' : 'Show the settings'} className="flex min-w-0 flex-1 flex-col pl-3 text-left">
      <span className="flex min-w-0 items-center">
        <Text typography="titleLarge" maxLines={1} ellipsis className="min-w-0">
          {title}
        </Text>
        <MaterialIcon name={open ? 'keyboard_arrow_down' : 'keyboard_arrow_up'} filled className="shrink-0" style={{ color: md('onSurfaceVariant') }} />
      </span>
      {detail === null ? null : (
        <Text typography="bodyMedium" color={warning ? md('error') : md('onSurfaceVariant')} maxLines={1} ellipsis>
          {detail}
        </Text>
      )}
    </button>
    <Button onClick={onDone} className="mr-1" style={{ paddingLeft: 16, paddingRight: 24 }}>
      <MaterialIcon name="check" filled size={18} />
      Done
    </Button>
  </div>
);
