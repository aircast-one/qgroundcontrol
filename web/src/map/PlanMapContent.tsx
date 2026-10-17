import { Fragment, type ReactNode, useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { type Json, has, isNull, jsonStrings, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { useBackHandler, useLaunchedEffect } from '@/kotlin/Compose';
import { openDocuments } from '@/kotlin/Platform';
import { delay, format, same } from '@/kotlin/Std';
import { MutableState, useValue } from '@/kotlin/State';
import { AlertDialog, Button, DropdownMenu, DropdownMenuItem, FilledTonalButton, IconButton, OutlinedTextField, Slider, Surface, Switch, Text, TextButton, stepsFor } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { aircast, md, withAlpha } from '@/theme/Colors';
import { AircastSheet } from './AircastSheet';
import { LATITUDE_LIMIT, LONGITUDE_LIMIT, RALLY_ALTITUDE_DECIMALS, SURFACE_DISTANCE_DECIMALS, altitudeFieldText, metresPerUnit, parsedAltitude, parsedCoordinate as parsedCoordinateWithin, parsedSurfaceDistance } from './AltitudeEntry';
import { BreachReturnDialog } from './BreachReturnDialog';
import { PLAN_UNDO, invokeOk, setOk, settingJson } from './BridgeCalls';
import {
  type BreachReturn,
  FENCE_POLYGONS,
  FenceBridge,
  type FenceCircle,
  type FencePolygon,
  type FenceRow,
  type FirmwareFence,
  GEOFENCE_NOT_SUPPORTED,
  NO_GEOFENCE,
  NO_RALLY_POINTS,
  RALLY_NOT_SUPPORTED,
  type RallyPoint,
  breachReturn,
  cornerRemovable,
  fenceCircles,
  fenceHeading,
  fencePolygons,
  fenceRowHit,
  fenceRows,
  fenceSelectionAfterRemove,
  firmwareFence,
  grownRadius,
  rallyAfterRemove,
  rallyPoints,
  rallyRows,
  shrunkRadius,
  trimmedRadius,
  typedRadius,
} from './FenceBridge';
import { LoadStep, loadStep, replaceWarning } from './LoadConfirm';
import { type LandingPattern, landingText } from './LandingPattern';
import { MapBridge, useMapPath } from './MapBridge';
import type { MapHit } from './MissionEditing';
import { AT_END, BEFORE_THE_REST, HOME_ITEM, insertMissionItem, removeMissionItem } from './MissionKinds';
import { MISSION_SPLIT_PATH, legSplit } from './MissionLayers';
import { missionSummaryText } from './MissionSummary';
import { OperatorBridge, operatorPoint } from './OperatorPosition';
import { readCamera } from './MapCamera';
import { centersOnVehicleAtEntry, fitsPlanOnEntry, planEntryPoint, planIsDrawn, stillFirstRead } from './PlanBounds';
import {
  KIND_CORRIDOR,
  KIND_DROP,
  KIND_LAND,
  KIND_STRUCTURE,
  KIND_SURVEY,
  KIND_TAKEOFF,
  KIND_WAYPOINT,
  type MissionItem,
  type MissionKind,
  PLAN_ITEMS,
  PlanBridge,
  allMissionItems,
  kindAllows,
  kindLabel,
  kindOffered,
  landingPatterns,
  linksStartToHome,
  missionKinds,
  placeLandingIfUnplaced,
  placeTakeoff,
  planItemCount,
  scanPatterns,
  takeoffMissing,
} from './PlanBridge';
import {
  type ItemRow,
  addingAfterText,
  cameraText,
  insertAfter,
  itemCountText,
  itemRows,
  itemTitle,
  layersText,
  moveGeneration,
  movedText,
  patternName,
  rallyAltitudeIsEditable,
  rallyAltitudeLabel,
  selectionAfterRemove,
  selectionSequence,
  sheetDetail,
  surveyTiles,
  worthListing,
  writeDragStep,
  writeMove,
} from './PlanItems';
import { PlanRail, RAIL_WIDTH, RailButton, RailDivider } from './PlanRail';
import {
  EmptyMissionStrip,
  type PlanStat,
  SelectionHeader,
  TERRAIN_CONFLICT_HERE,
  WaypointStripBar,
  advancedDetail,
  appendSequence,
  farFromAircraft,
  panelDrag,
  placedPattern,
  planStats,
  repositionMission,
  selectionTitle,
  startsFromAircraft,
  tapCloses,
  terrainWarning,
} from './PlanStrip';
import { selectionText } from './PlanSummary';
import {
  type ShapeTarget,
  circleAround,
  circleRadius,
  circleRadiusMetres,
  defaultCircle,
  defaultLine,
  defaultRectangle,
  importShapeFiles,
  liveCircles,
  positionTitle,
  replaceShape,
  shapeCaption,
  shapeCentreHit,
  shapeEditable,
  shapeTarget,
  shapeVertices,
  traceCaption,
} from './PolygonTools';
import { currentMapType, qgcRasterStyle } from './QgcTileSource';
import { ScaleBarView } from './ScaleBarView';
import { fenceDetail, selectedFence, selectedLanding, selectedSurvey, selectionSurvives } from './SelectionValidity';
import { sentenceCase } from './SentenceCase';
import { GRID_ANGLE_MAX, type Survey, SurveyBridge, type SurveyStats, fitSurveyArea, gridAngleShown, insetRing, surveyStatsFor } from './SurveyBridge';
import { TERRAIN_VIEW, type TerrainProfile, collidingItems, collisionLegs, terrainProfile } from './TerrainProfile';
import { ELEVATION_PROVIDER, SHOW_MISSION_ITEM_STATUS, TerrainProfileView, missionItemStatusShown } from './TerrainProfileView';
import { downloadMessage, uploadMessage, uploadOutcome } from './UploadOutcome';
import { VEHICLES_VIEW, uploadHeading, vehicleChoices } from './VehiclePicker';
import { TrackPoint, VehicleMap, isPlottable } from './VehicleMap';
import { PlanFocus, UPLOADED, type UploadGate, freshPlanView, notReadyToSend, planSupport, syncRefusal, uploadGate, uploadLabel, uploadStep, vehicleSyncState } from './VehicleSync';
import { WaypointSettings } from './WaypointPanel';

const PLAN_POLL_MS = 700;
const FAILURE_MESSAGE_MS = 2500;
const PANEL_MAX_FRACTION = 0.5;
const SIDE_PANEL_WIDTH = 380;
const SIDE_PANEL_MIN_WIDTH_DP = 840;
export const SUMMARY_MAX_FRACTION = 0.74;

const WAITING_FOR_QGC = 'Waiting for QGroundControl';
const CHECKING_TERRAIN = 'Checking the terrain…';
const FAR_FROM_AIRCRAFT_TEXT = 'The aircraft is not where this route begins. Move the route so it starts at the aircraft, or upload it as it is.';
const VIEW_CORNERS = 4;
const PATTERN_KINDS: ReadonlySet<string> = new Set([KIND_SURVEY, KIND_CORRIDOR, KIND_STRUCTURE]);

const NO_POINTS: readonly TrackPoint[] = [];

class Remembered<T> extends MutableState<T> {
  override get value(): T {
    return super.value;
  }

  override set value(next: T) {
    if (!same(super.value, next)) super.value = next;
  }
}

const useRemembered = <T,>(initial: T): MutableState<T> => {
  const [state] = useState(() => new Remembered(initial));
  useValue(state);
  return state;
};

const firstWhere = <T,>(state: MutableState<T>, predicate: (value: T) => boolean): Promise<T> =>
  new Promise((resolve) => {
    if (predicate(state.value)) {
      resolve(state.value);
      return;
    }
    const unsubscribe = state.subscribe(() => {
      if (!predicate(state.value)) return;
      unsubscribe();
      resolve(state.value);
    });
  });

const subscribeResize = (listener: () => void) => {
  window.addEventListener('resize', listener);
  return () => window.removeEventListener('resize', listener);
};

const useWindowWidth = (): number => useSyncExternalStore(subscribeResize, () => window.innerWidth, () => 0);

const useWindowHeight = (): number => useSyncExternalStore(subscribeResize, () => window.innerHeight, () => 0);

const useMeasuredHeight = (): readonly [(element: HTMLElement | null) => (() => void) | undefined, number] => {
  const [height, setHeight] = useState(0);
  const ref = useCallback((element: HTMLElement | null) => {
    if (element === null) return undefined;
    const read = () => setHeight(element.offsetHeight);
    read();
    const observer = new ResizeObserver(read);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return [ref, height] as const;
};

const subscribeVisibility = (listener: () => void) => {
  document.addEventListener('visibilitychange', listener);
  return () => document.removeEventListener('visibilitychange', listener);
};

const useDocumentStarted = (): boolean => useSyncExternalStore(subscribeVisibility, () => document.visibilityState !== 'hidden', () => true);

const sendPlan = (say: (message: string | null) => void, done: () => void, pauseFirst = false): void => {
  say('Uploading to vehicle');
  void (async () => {
    if (pauseFirst) await PlanBridge.pauseVehicle();
    const outcome = uploadOutcome(await PlanBridge.sendToVehicle());
    say(uploadMessage(outcome));
    await delay(FAILURE_MESSAGE_MS);
    done();
  })();
};

const FenceHeading = ({ text }: { text: string }) => (
  <Text typography="titleSmall" className="block w-full px-1 pb-1 pt-2">
    {text}
  </Text>
);

const PaletteNote = ({ text }: { text: string }) => (
  <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block w-full p-1">
    {text}
  </Text>
);

const FALLBACK_FENCE_DEGREES = 0.002;

export const fenceWindow = (visible: readonly TrackPoint[], at: TrackPoint): readonly [TrackPoint, TrackPoint] => {
  const [topLeft, , bottomRight] = visible;
  return visible.length === 4 && topLeft !== undefined && bottomRight !== undefined
    ? [topLeft, bottomRight]
    : [TrackPoint(at.latitude + FALLBACK_FENCE_DEGREES, at.longitude - FALLBACK_FENCE_DEGREES), TrackPoint(at.latitude - FALLBACK_FENCE_DEGREES, at.longitude + FALLBACK_FENCE_DEGREES)];
};

export type PlanUpload = { readonly enabled: boolean; readonly emphasised: boolean; readonly label: string; readonly shown: boolean; readonly onClick: () => void; readonly done: boolean };

export const PlanUpload = (fields: { enabled: boolean; emphasised: boolean; label: string; shown: boolean; onClick: () => void }): PlanUpload => ({ ...fields, done: fields.label === UPLOADED });

export type PlanBar = { readonly upload: PlanUpload; readonly stats: readonly PlanStat[]; readonly warning: string | null; readonly note: string | null };

type BridgeOptions = { label?: string | null; done?: string | null; then?: (() => void) | null };

type OnBridge = (work: () => Promise<boolean>, options?: BridgeOptions) => void;

type Say = (message: string) => void;

const hitKey = (hit: MapHit | null): string => JSON.stringify(hit);

export const PlanMapContent = ({
  mapStyle,
  onCentre = null,
  itemPanel = null,
  header = null,
  routeSettings = null,
  fitKey = 0,
  onTemplates = null,
}: {
  mapStyle: string;
  onCentre?: ((latitude: number, longitude: number) => void) | null;
  itemPanel?: ((index: number, at: TrackPoint | null, leg: string | null, remove: (() => void) | null) => ReactNode) | null;
  header?: ((bar: PlanBar) => ReactNode) | null;
  routeSettings?: (() => ReactNode) | null;
  fitKey?: number;
  onTemplates?: (() => void) | null;
}) => {
  const follow = useRemembered(false);
  const shownStyle = useRemembered(mapStyle);
  useEffect(() => {
    shownStyle.value = mapStyle;
  }, [mapStyle]);
  const fitRequest = useRemembered(0);
  const edits = useRemembered(0);
  const fitOnly = useRemembered<readonly TrackPoint[] | null>(null);
  const positioning = useRemembered<readonly [MapHit, TrackPoint] | null>(null);
  const loadArmed = useRemembered(false);
  const items = useRemembered<readonly MissionItem[]>([]);
  const allItems = useRemembered<readonly MissionItem[]>([]);
  const itemCount = useRemembered(0);
  const linkStartToHome = useRemembered(false);
  const fences = useRemembered<readonly FencePolygon[]>([]);
  const chosenCircles = useRemembered<ReadonlySet<string>>(new Set());
  const radiusFor = useRemembered<ShapeTarget | null>(null);
  const rally = useRemembered<readonly RallyPoint[]>([]);
  const operator = useRemembered<TrackPoint | null>(null);
  const circles = useRemembered<readonly FenceCircle[]>([]);
  const firmware = useRemembered<FirmwareFence | null>(null);
  const breach = useRemembered<BreachReturn | null>(null);
  const editingBreach = useRemembered(false);
  const surveyList = useRemembered<readonly Survey[]>([]);
  const circled = liveCircles(chosenCircles.value, fences.value, surveyList.value);
  const landingList = useRemembered<readonly LandingPattern[]>([]);
  const surveyStatsMap = useRemembered<ReadonlyMap<number, SurveyStats>>(new Map());
  const selected = useRemembered<MapHit | null>(null);
  const panelOpen = useRemembered(true);
  const readsStarted = useRemembered(0);
  const planReads = useRemembered(0);
  const layer = useRemembered<PlanLayer>(PlanLayer.Mission);
  const selectedKey = hitKey(selected.value);
  useEffect(() => {
    const owning = layerOf(selected.value);
    if (owning !== null) layer.value = owning;
  }, [selectedKey]);

  useBackHandler(selected.value !== null, () => {
    selected.value = null;
  });
  const busy = useRemembered<string | null>(null);
  const removals = useRemembered(0);
  const removeItem = (item: MissionItem): void => {
    void (async () => {
      busy.value = `Removing #${item.sequence}`;
      const outcome = await removeMissionItem(item.index);
      if (outcome.ok) removals.value += 1;
      selected.value = outcome.ok ? selectionAfterRemove(item.index, allItems.value.length) : null;
      busy.value = outcome.ok ? null : outcome.reason;
      if (!outcome.ok) {
        await delay(FAILURE_MESSAGE_MS);
        busy.value = null;
      }
    })();
  };
  const centre = useRemembered<TrackPoint | null>(null);
  const zoom = useRemembered(0.0);
  const [controlsRef, controlsHeightPx] = useMeasuredHeight();
  const [headerRef, headerPx] = useMeasuredHeight();
  const screenWidth = useWindowWidth();
  const screenHeight = useWindowHeight();
  const sidePanel = screenWidth >= SIDE_PANEL_MIN_WIDTH_DP;
  const sidePanelPx = sidePanel ? SIDE_PANEL_WIDTH : 0;
  const railPx = RAIL_WIDTH + 8;

  const kindsView = useMapPath('view.missionKinds');
  const insertable = useMemo(() => missionKinds(kindsView), [kindsView]);

  const say: Say = (message) => {
    busy.value = message;
    void (async () => {
      await delay(FAILURE_MESSAGE_MS);
      busy.value = null;
    })();
  };

  const addMissionItem = (kindId: string, label: string, at: TrackPoint | null, index: number = AT_END): void => {
    void (async () => {
      if (at === null) {
        busy.value = 'Move the map to where this should go';
        await delay(FAILURE_MESSAGE_MS);
        busy.value = null;
        return;
      }
      busy.value = label;
      const outcome = await insertMissionItem(kindId, at.latitude, at.longitude, index);
      if (!outcome.ok) {
        busy.value = outcome.reason;
        await delay(FAILURE_MESSAGE_MS);
        busy.value = null;
        return;
      }
      busy.value = null;
      const added = outcome.index ?? null;
      if (added === null) return;
      if (PATTERN_KINDS.has(kindId)) PlanFocus.newPattern.value = added;
      if (kindId === KIND_LAND) await placeLandingIfUnplaced(added, at.latitude, at.longitude);
      else if (kindId === KIND_TAKEOFF) await placeTakeoff(added, at.latitude, at.longitude);
      edits.value += 1;
      selected.value = { kind: 'Waypoint', index: added };
    })();
  };

  const onRefusal = (work: () => Promise<string | null>, label: string | null = null): void => {
    busy.value = label;
    void (async () => {
      const refusal = await work();
      busy.value = refusal;
      if (refusal !== null) await delay(FAILURE_MESSAGE_MS);
      busy.value = null;
    })();
  };

  const onBridge: OnBridge = (work, { label = null, done = null, then = null } = {}) => {
    busy.value = label;
    void (async () => {
      const ok = await work();
      if (!ok) {
        busy.value = `${label ?? 'That'} did not work`;
        await delay(FAILURE_MESSAGE_MS);
        busy.value = null;
        return;
      }
      if (then !== null) {
        edits.value += 1;
        then();
      }
      busy.value = done;
      if (done !== null) {
        await delay(FAILURE_MESSAGE_MS);
        busy.value = null;
      }
    })();
  };

  const planStatus = useMapPath('view.plan');
  const planHasItems = optBoolean(planStatus, 'containsItems');
  const globalFrame = has(planStatus, 'globalAltitudeFrame') && !isNull(planStatus, 'globalAltitudeFrame') ? optInt(planStatus, 'globalAltitudeFrame') : null;
  const planOffline = optBoolean(planStatus, 'offline');
  const planDirty = optBoolean(planStatus, 'dirty');
  const planSyncing = optText(optJSONObject(planStatus, 'sync'), 'state') === 'busy';
  const support = useMemo(() => planSupport(planStatus), [planStatus]);
  const canUndo = optBoolean(planStatus, 'canUndo');
  const templates = optJSONObject(planStatus, 'templates');
  const homeSet = templates === null || optBoolean(templates, 'homeSet');
  const addable = (kind: string): boolean => kindAllows(insertable, kind) || !homeSet;
  const addablePattern = (kind: MissionKind): boolean => kind.enabled || !homeSet;
  const uploadAsk = useRemembered<UploadGate | null>(null);
  const vehiclesJson = useMapPath(VEHICLES_VIEW);
  const flown = useMemo(() => vehicleChoices(vehiclesJson).active, [vehiclesJson]);
  const latitude = flown?.latitude ?? NaN;
  const longitude = flown?.longitude ?? NaN;
  const patternWanted = useRemembered<readonly MissionKind[]>([]);
  const missionSummaryView = useMapPath('view.missionSummary');
  const terrainView = useMapPath(TERRAIN_VIEW);
  const profile = useMemo(() => terrainProfile(terrainView), [terrainView]);
  const terrainHits = useMemo(() => collisionLegs(terrainView), [terrainView]);
  const collidingPatterns = useMemo(() => collidingItems(terrainView), [terrainView]);
  const collidingSimple = useMemo(() => collidingItems(terrainView, 'collidingSimpleItems'), [terrainView]);
  const conflicts = useMemo<ReadonlySet<number>>(() => new Set([...collidingSimple, ...collidingPatterns]), [collidingSimple, collidingPatterns]);
  const elevationProviderJson = useMapPath(ELEVATION_PROVIDER);
  const elevationNotice = optText(elevationProviderJson, 'value');
  const missionStatusJson = useMapPath(`${SHOW_MISSION_ITEM_STATUS}.rawValue`);
  const missionStatusShown = missionItemStatusShown(missionStatusJson);

  const placeAt = (): TrackPoint | null => {
    const at = centre.value;
    if (at !== null && isPlottable(at.latitude, at.longitude)) return at;
    return isPlottable(latitude, longitude) ? TrackPoint(latitude, longitude) : null;
  };

  const visible = useRemembered<readonly TrackPoint[]>([]);
  const importInto = async (target: ShapeTarget): Promise<void> => {
    const files = await openDocuments([]);
    if (files.length === 0) return;
    const message = await importShapeFiles(files, target);
    if (message !== null) say(message);
  };
  const tracing = useRemembered<readonly [ShapeTarget, readonly TrackPoint[]] | null>(null);
  const firstRead = useRemembered(true);
  const listOpen = useRemembered(false);
  const centreRequest = useRemembered(0);
  const centreOn = useRemembered<TrackPoint | null>(null);
  const focusItem = (index: number): void => {
    panelOpen.value = true;
    selected.value = { kind: 'Waypoint', index };
    const placedItem = items.value.find((it) => it.index === index);
    if (placedItem === undefined) return;
    centreOn.value = TrackPoint(placedItem.latitude, placedItem.longitude);
    centreRequest.value += 1;
    follow.value = false;
  };
  const pickRow = (row: ItemRow): void => focusItem(row.index);
  const addRallyAt = (lat: number, lon: number): void => {
    const next = rally.value.length;
    onBridge(() => FenceBridge.addRallyPoint(lat, lon), {
      label: 'Adding rally',
      then: () => {
        selected.value = { kind: 'Rally', index: next };
      },
    });
  };
  const centredOnEntry = useRemembered(false);
  const lastViewed = useMemo(() => readCamera()?.centre ?? null, []);
  const entryPoint = planEntryPoint(isPlottable(latitude, longitude) ? TrackPoint(latitude, longitude) : null, operator.value, lastViewed);
  useEffect(() => {
    if (!centersOnVehicleAtEntry(centredOnEntry.value, entryPoint !== null, fitRequest.value)) return;
    centredOnEntry.value = true;
    centreOn.value = entryPoint;
    centreRequest.value += 1;
  }, [entryPoint !== null]);

  useEffect(() => {
    if (fitKey !== 0) firstRead.value = true;
  }, [fitKey]);

  const selectedSequence = selectionSequence(selected.value, allItems.value);

  useEffect(() => {
    const take = () => {
      const index = PlanFocus.requests.value;
      if (index === null) return;
      selected.value = { kind: 'Waypoint', index };
      PlanFocus.requests.value = null;
    };
    take();
    return PlanFocus.requests.subscribe(take);
  }, []);

  const newPattern = useValue(PlanFocus.newPattern);
  useLaunchedEffect([newPattern], async (scope) => {
    const wanted = newPattern;
    if (wanted === null) return;
    const since = readsStarted.value;
    await firstWhere(planReads, (it) => it > since);
    if (!scope.isActive()) return;
    PlanFocus.newPattern.value = null;
    const pattern = placedPattern(surveyList.value, wanted);
    const target = pattern === null ? null : shapeTarget(null, pattern);
    if (target === null) return;
    const view = visible.value;
    if (view.length === VIEW_CORNERS) onBridge(() => replaceShape(target, target.line ? defaultLine(view) : defaultRectangle(view)));
  });

  const appendAfter = appendSequence(allItems.value);
  useEffect(() => {
    const sequence = selectedSequence ?? (selected.value === null ? appendAfter : null);
    if (sequence === null) return;
    void PlanBridge.selectSequence(sequence);
  }, [selectedKey, selectedSequence, appendAfter]);

  const refresh = async (): Promise<void> => {
    const readAt = edits.value;
    const readNumber = readsStarted.value + 1;
    readsStarted.value = readNumber;
    const plan = await PlanBridge.rawItems();
    if (plan !== null) MapBridge.markReachable();
    const nextAll = allMissionItems(plan);
    const nextItems = nextAll.filter((it) => it.placed);
    const nextItemCount = planItemCount(plan);
    const nextLink = linksStartToHome(plan);
    const fenceView = await FenceBridge.read();
    const nextFences = await fencePolygons(fenceView);
    const nextRally = rallyPoints(fenceView);
    const nextOperator = operatorPoint(await OperatorBridge.read());
    const nextCircles = fenceCircles(fenceView);
    const nextFirmware = firmwareFence(fenceView);
    const nextBreach = breachReturn(fenceView);
    const nextSurveys = await SurveyBridge.surveysFrom(plan);
    const nextLandings = await landingPatterns(nextAll);
    const nextStats = await surveyStatsFor(nextAll);
    const drawn = planIsDrawn(nextItems, nextSurveys, nextFences, nextCircles, nextRally);
    if (fitsPlanOnEntry(firstRead.value, plan !== null, drawn)) {
      follow.value = false;
      fitRequest.value += 1;
    }
    firstRead.value = stillFirstRead(firstRead.value, plan !== null);
    if (!firstRead.value && drawn) centredOnEntry.value = true;
    allItems.value = nextAll;
    items.value = nextItems;
    itemCount.value = nextItemCount;
    linkStartToHome.value = nextLink;
    fences.value = nextFences;
    rally.value = nextRally;
    operator.value = nextOperator;
    if (readAt === edits.value && !selectionSurvives(selected.value, nextAll, nextFences, nextCircles, nextRally, nextSurveys, nextLandings, nextBreach !== null)) {
      selected.value = null;
    }
    circles.value = nextCircles;
    firmware.value = nextFirmware;
    breach.value = nextBreach;
    surveyList.value = nextSurveys;
    landingList.value = nextLandings;
    surveyStatsMap.value = nextStats;
    planReads.value = readNumber;
  };

  const started = useDocumentStarted();
  useLaunchedEffect([started], async (scope) => {
    if (!started) return;
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      await refresh();
      await delay(PLAN_POLL_MS);
      return poll();
    };
    await poll();
  });

  const syncState = vehicleSyncState(planOffline, planSyncing);
  const uploadBlocked = syncRefusal(syncState, 'upload to') !== null;
  const aircraftAt = isPlottable(latitude, longitude) && !planOffline ? TrackPoint(latitude, longitude) : null;
  const farAway = farFromAircraft(aircraftAt, allItems.value);
  const farText = farAway === null ? null : startsFromAircraft(farAway, optBoolean(missionSummaryView, 'imperial'));
  const farAsk = useRemembered(false);

  const sendUpload = (): void => {
    const refusal = syncRefusal(vehicleSyncState(planOffline, planSyncing), 'upload to');
    if (refusal !== null) {
      say(refusal);
      return;
    }
    void (async () => {
      const view = await freshPlanView();
      const step = uploadStep(uploadGate(view), notReadyToSend(view));
      switch (step.kind) {
        case 'Refuse':
          PlanFocus.notReady(view);
          say(step.reason);
          return;
        case 'Confirm':
          uploadAsk.value = step.gate;
          return;
        case 'Send':
          sendPlan(
            (message) => {
              busy.value = message;
            },
            () => {
              busy.value = null;
            },
          );
      }
    })();
  };

  const upload = (): void => {
    if (farAway !== null) farAsk.value = true;
    else sendUpload();
  };

  const download = (): void => {
    loadArmed.value = false;
    busy.value = 'Downloading from vehicle';
    selected.value = null;
    void (async () => {
      const outcome = uploadOutcome(await PlanBridge.loadFromVehicle());
      busy.value = downloadMessage(outcome);
      await delay(FAILURE_MESSAGE_MS);
      busy.value = null;
    })();
  };

  const requestDownload = (): void => {
    const refusal = syncRefusal(vehicleSyncState(planOffline, planSyncing), 'download from');
    if (refusal !== null) say(refusal);
    else if (loadStep(planDirty, loadArmed.value) === LoadStep.Confirm) loadArmed.value = true;
    else download();
  };

  const uploadText = uploadLabel(planOffline, planSyncing, planDirty, planHasItems);
  const uploadEnabled = planHasItems && !planOffline && !planSyncing;
  const uploadEmphasised = uploadEnabled && !uploadBlocked;
  const planUpload = PlanUpload({ enabled: uploadEnabled, emphasised: uploadEmphasised, label: uploadText, shown: !planOffline, onClick: upload });

  const chosen = selected.value;
  const ownerKey = ownerOf(chosen);
  const shownItems = useMemo(() => items.value.map((it) => ({ ...it, terrainCollision: collidingSimple.has(it.index) })), [items.value, collidingSimple]);
  const shownFences = useMemo(() => fences.value.map((it) => (ownerKey === `p${it.index}` ? it : { ...it, editable: null })), [fences.value, ownerKey]);
  const shownSurveys = useMemo(
    () => surveyList.value.map((it) => (ownerKey === `m${it.index}` ? it : { ...it, editable: null })).map((it) => ({ ...it, collides: collidingPatterns.has(it.index) })),
    [surveyList.value, ownerKey, collidingPatterns],
  );
  const shownLandings = useMemo(() => landingList.value.map((it) => ({ ...it, collides: collidingPatterns.has(it.index) })), [landingList.value, collidingPatterns]);

  const ready = useValue(MapBridge.bridgeReady);
  const mapStart = sidePanel ? SIDE_PANEL_WIDTH : 0;
  const railStart = mapStart + 8;
  const headerHeight = headerPx;
  const panelHeight = controlsHeightPx;
  const listedInPanel = sidePanel && layer.value === PlanLayer.Mission && worthListing(allItems.value);
  const railTop = headerHeight + 8;
  const railBottom = sidePanel ? 8 : panelHeight + 8;
  const railMaxHeight = `calc(100% - ${railTop + railBottom}px)`;
  const summaryStart = railStart + RAIL_WIDTH + 8;
  const message = busy.value ?? (ready ? null : WAITING_FOR_QGC);
  const chosenItem = chosen?.kind === 'Waypoint' ? (allItems.value.find((it) => it.index === chosen.index) ?? null) : null;
  const rows = useMemo(() => itemRows(allItems.value, surveyStatsMap.value), [allItems.value, surveyStatsMap.value]);
  const altitudes = useMemo<ReadonlyMap<number, string>>(() => new Map(allItems.value.map((it) => [it.index, it.altitudeText])), [allItems.value]);
  const traced = tracing.value;
  const panelScroll = useRef<HTMLDivElement>(null);
  useEffect(() => {
    panelScroll.current?.scrollTo({ top: 0 });
  }, [selectedKey]);

  const survey = selectedSurvey(chosen, surveyList.value);
  const shapeFence = (chosen?.kind === 'ShapeCentre' || chosen?.kind === 'ShapeRadius') && chosen.fence ? chosen.owner : null;
  const fenceHit = chosen?.kind === 'FenceVertex' ? chosen : null;
  const surveyHit = chosen?.kind === 'SurveyVertex' ? chosen : null;
  const rallyHit = chosen?.kind === 'Rally' ? chosen : null;
  const circleIndex = chosen?.kind === 'Circle' || chosen?.kind === 'CircleCentre' ? chosen.index : null;
  const circle = circleIndex === null ? null : (circles.value.find((it) => it.index === circleIndex) ?? null);
  const landing = selectedLanding(chosen, landingList.value);
  const fence = selectedFence(chosen, fences.value, circles.value);
  const fenceText = fenceDetail(chosen, fences.value, circles.value);
  const landingNote = landingText(landing);
  const cameraNote = survey === null ? null : cameraText(surveyStatsMap.value.get(survey.index) ?? null);
  const layersNote = layersText(survey);
  const cornerHit = fenceHit ?? surveyHit;
  const corner = cornerHit === null ? null : cornerPosition(cornerHit, fences.value, surveyList.value);
  const target = shapeTarget(fenceHit?.polygon ?? shapeFence, survey !== null && (surveyHit !== null || chosen?.kind === 'ShapeCentre' || chosen?.kind === 'ShapeRadius') ? survey : null);
  const targetShape = target === null ? null : shapeEditable(target, fences.value, surveyList.value);
  const targetCentre = target === null ? null : shapeCentreHit(target, fences.value, surveyList.value);
  const deletedPattern = surveyHit?.item ?? (chosen?.kind === 'ShapeCentre' && !chosen.fence ? chosen.owner : null);
  const rallyPoint = rallyHit === null ? null : (rally.value.find((it) => it.index === rallyHit.index) ?? null);
  const showsTools = survey !== null || fenceHit !== null || shapeFence !== null || rallyHit !== null || circle !== null || landing !== null;
  const chosenStats = chosenItem === null ? null : (surveyStatsMap.value.get(chosenItem.index) ?? null);
  const tiles = chosenItem === null ? [] : surveyTiles(chosenItem, chosenStats);
  const chosenDetail = (() => {
    if (chosenItem === null) return null;
    if (conflicts.has(chosenItem.index)) return TERRAIN_CONFLICT_HERE;
    const pattern = chosenItem.complexPattern ? sheetDetail(chosenItem, chosenStats) : '';
    if (pattern.trim() !== '') return pattern;
    return panelOpen.value ? null : addingAfterText(chosen, allItems.value);
  })();
  const listedFences = support.fenceRefused ? [] : fenceRows(fences.value, circles.value);

  const bar: PlanBar = {
    upload: planUpload,
    stats: planStats(itemCount.value, allItems.value, missionSummaryView),
    warning: farText ?? terrainWarning(terrainHits.length, conflicts.size),
    note: itemCount.value > 0 && optBoolean(terrainView, 'checking') ? CHECKING_TERRAIN : null,
  };

  const radiusVertices = radiusFor.value === null ? null : shapeVertices(radiusFor.value, fences.value, surveyList.value);
  const radiusShape = radiusFor.value === null ? null : shapeEditable(radiusFor.value, fences.value, surveyList.value);
  const radiusShown = radiusVertices === null ? 0.0 : (circleRadius(radiusVertices) ?? 0.0);
  const radiusUnits = radiusShape?.metresPerUnit ?? 1.0;

  return (
    <div className="relative size-full overflow-hidden">
      <VehicleMap
        className="absolute inset-0 size-full"
        mapStyle={shownStyle.value}
        follow={follow.value}
        missionItems={shownItems}
        linkStartToHome={linkStartToHome.value}
        fencePolygons={shownFences}
        fenceCircles={circles.value}
        firmwareFence={firmware.value}
        breachReturn={breach.value?.point ?? null}
        rallyPoints={rally.value}
        operator={operator.value}
        surveys={shownSurveys}
        landings={shownLandings}
        editable
        circledShapes={circled}
        collisionLegs={terrainHits}
        onAdd={(lat, lon) => {
          const trace = tracing.value;
          if (trace !== null) {
            tracing.value = [trace[0], [...trace[1], TrackPoint(lat, lon)]];
            return;
          }
          if (layer.value === PlanLayer.Mission) {
            panelOpen.value = false;
            addMissionItem(KIND_WAYPOINT, 'Adding a waypoint', TrackPoint(lat, lon), insertAfter(selected.value, allItems.value));
          } else if (layer.value === PlanLayer.Rally && support.rally) addRallyAt(lat, lon);
        }}
        onBlankTap={(lat, lon) => {
          const trace = tracing.value;
          if (trace !== null) {
            tracing.value = [trace[0], [...trace[1], TrackPoint(lat, lon)]];
          } else if (tapCloses(selected.value, panelOpen.value)) {
            selected.value = null;
            panelOpen.value = false;
          } else if (layer.value === PlanLayer.Mission) {
            panelOpen.value = false;
            addMissionItem(KIND_WAYPOINT, 'Adding a waypoint', TrackPoint(lat, lon), insertAfter(selected.value, allItems.value));
          } else if (layer.value === PlanLayer.Rally && support.rally) addRallyAt(lat, lon);
          else selected.value = null;
        }}
        canDrag={(hit) => dragAllowed(hit, selected.value, layer.value)}
        onMove={(hit, lat, lon) => {
          const generation = moveGeneration();
          onBridge(() => writeDragStep(generation, hit, lat, lon, surveyList.value, rally.value, fences.value, allItems.value, circles.value));
        }}
        onWaypointSelected={(hit) => {
          if (hit !== null && actsOnTap(hit) && !dragAllowed(hit, selected.value, layer.value)) return;
          switch (hit?.kind) {
            case 'Midpoint':
              if (hit.path === MISSION_SPLIT_PATH) {
                addMissionItem(
                  KIND_WAYPOINT,
                  'Adding a waypoint',
                  legSplit(
                    allItems.value.filter((it) => it.placed),
                    hit.segment,
                  ),
                  hit.segment,
                );
              } else onBridge(() => invokeOk(`${hit.path}.${hit.invokable}`, `[${hit.segment}]`), { label: 'Adding a corner' });
              return;
            case 'BreachReturn':
              editingBreach.value = true;
              return;
            case 'LoiterRotation': {
              const item = allItems.value.find((it) => it.index === hit.index);
              if (item !== undefined) onBridge(() => PlanBridge.setLoiterRadius(item.index, -item.loiterRadius), { done: movedText(hit, allItems.value) });
              return;
            }
            case 'LoiterRadius':
              selected.value = { kind: 'Waypoint', index: hit.index };
              return;
            case 'CircleRadius':
              selected.value = { kind: 'Circle', index: hit.index };
              return;
            default:
              panelOpen.value = true;
              selected.value = hit;
          }
        }}
        onMoved={(hit, lat, lon) => {
          onBridge(() => writeMove(hit, lat, lon, surveyList.value, rally.value, fences.value, allItems.value, circles.value), { done: movedText(hit, allItems.value) });
        }}
        selectedWaypoint={chosen?.kind === 'Waypoint' ? chosen.index : null}
        onViewChanged={(corners) => {
          visible.value = corners;
        }}
        tracePoints={traced?.[1] ?? NO_POINTS}
        traceLine={traced?.[0].line === true}
        onCentreChanged={(at, level) => {
          centre.value = at;
          zoom.value = level;
          onCentre?.(at.latitude, at.longitude);
        }}
        bottomInsetPx={controlsHeightPx}
        topInsetPx={headerPx}
        leftInsetPx={sidePanelPx + railPx}
        rightInsetPx={railPx}
        fitRequest={fitRequest.value}
        fitOnly={fitOnly.value}
        onFitFailed={() => onBridge(async () => false, { label: 'Fitting the plan' })}
        centreRequest={centreRequest.value}
        centreOn={centreOn.value}
      />

      {radiusFor.value === null ? null : (
        <RadiusDialog
          key={`${radiusShown}:${radiusUnits}`}
          radius={radiusShown}
          unit={radiusShape?.distanceUnit ?? 'm'}
          metresPerUnit={radiusUnits}
          onDismiss={() => {
            radiusFor.value = null;
          }}
          onSet={(radius) => {
            const target = radiusFor.value;
            radiusFor.value = null;
            const ring = radiusVertices === null ? null : circleAround(radiusVertices, radius);
            if (target !== null && ring !== null) onBridge(() => replaceShape(target, ring), { label: 'Changed the circle radius' });
          }}
        />
      )}

      {positioning.value === null ? null : (
        <PositionDialog
          key={`${positioning.value[1].latitude},${positioning.value[1].longitude}`}
          at={positioning.value[1]}
          title={positionTitle(positioning.value[0])}
          onDismiss={() => {
            positioning.value = null;
          }}
          onMove={(moved) => {
            const hit = positioning.value?.[0];
            positioning.value = null;
            if (hit === undefined) return;
            onBridge(() => writeMove(hit, moved.latitude, moved.longitude, surveyList.value, rally.value, fences.value, allItems.value, circles.value), { done: movedText(hit, allItems.value) });
          }}
        />
      )}

      {header === null ? null : (
        <div ref={headerRef} className="absolute right-0 top-0" style={{ left: mapStart }}>
          {header(bar)}
        </div>
      )}

      <div className="pointer-events-none absolute flex flex-col items-start" style={{ left: summaryStart, top: headerHeight + 8, width: `calc((100% - ${summaryStart + 8}px) * ${SUMMARY_MAX_FRACTION})` }}>
        {message === null ? null : (
          <Surface color={withAlpha(md('surfaceContainer'), 0.94)} contentColor={md('onSurface')} shape="extraLarge" className="pointer-events-auto">
            <Text typography="bodyMedium" className="block px-4 py-2.5">
              {message}
            </Text>
          </Surface>
        )}
        {centre.value === null ? null : <ScaleBarView latitude={centre.value.latitude} zoom={zoom.value} className="pl-1 pt-2" />}
      </div>

      <PlanRail className="absolute" style={{ left: railStart, top: railTop, maxHeight: railMaxHeight }}>
        {PlanLayerEntries.map((option) => (
          <RailButton
            key={option.name}
            icon={option.icon}
            label={option.label}
            chosen={layer.value === option}
            labelled
            onClick={() => {
              panelOpen.value = true;
              layer.value = option;
              if (layerOf(selected.value) !== option) selected.value = null;
            }}
          />
        ))}
        <RailDivider />
        {layer.value === PlanLayer.Mission ? (
          <>
            <RailButton icon="plan_add" label="Waypoint" enabled={placeAt() !== null} onClick={() => addMissionItem(KIND_WAYPOINT, 'Adding a waypoint', placeAt(), insertAfter(selected.value, allItems.value))} />
            <RailButton
              icon="plan_grid"
              label="Pattern"
              enabled={scanPatterns(insertable).length > 0}
              onClick={() => {
                patternWanted.value = scanPatterns(insertable);
              }}
            />
            {kindOffered(insertable, KIND_TAKEOFF) && takeoffMissing(items.value) ? (
              <RailButton icon="plan_flight_takeoff" label="Takeoff" enabled={addable(KIND_TAKEOFF)} onClick={() => addMissionItem(KIND_TAKEOFF, 'Adding a takeoff', placeAt(), BEFORE_THE_REST)} />
            ) : null}
            <RailButton
              icon="plan_flight_land"
              label={kindLabel(insertable, KIND_LAND)}
              enabled={itemCount.value > 0}
              onClick={() => addMissionItem(KIND_LAND, 'Adding a landing', placeAt(), insertAfter(selected.value, allItems.value))}
            />
            {kindOffered(insertable, KIND_DROP) ? (
              <RailButton icon="plan_drop" label="Drop" enabled={itemCount.value > 0 && addable(KIND_DROP)} onClick={() => addMissionItem(KIND_DROP, 'Adding a drop', placeAt(), insertAfter(selected.value, allItems.value))} />
            ) : null}
          </>
        ) : layer.value === PlanLayer.Fence ? (
          support.fenceRefused ? null : (
            <>
              <RailButton
                icon="plan_polygon"
                label="Polygon"
                enabled={support.fence}
                onClick={() => {
                  const at = placeAt();
                  const next = fences.value.length;
                  onBridge(
                    async () => {
                      const window = at === null ? null : fenceWindow(visible.value, at);
                      return window === null ? false : FenceBridge.addInclusionPolygon(window[0], window[1]);
                    },
                    {
                      label: 'Adding fence',
                      then: () => {
                        selected.value = { kind: 'FenceVertex', polygon: next, vertex: 0 };
                      },
                    },
                  );
                }}
              />
              <RailButton
                icon="plan_circle"
                label="Circle"
                enabled={support.fence}
                onClick={() => {
                  const at = placeAt();
                  const next = circles.value.length;
                  onBridge(
                    async () => {
                      const window = at === null ? null : fenceWindow(visible.value, at);
                      return window === null ? false : FenceBridge.addInclusionCircle(window[0], window[1]);
                    },
                    {
                      label: 'Adding circle',
                      then: () => {
                        selected.value = { kind: 'Circle', index: next };
                      },
                    },
                  );
                }}
              />
              <RailButton
                icon="plan_home"
                label="Breach"
                enabled={support.fence}
                chosen={breach.value !== null}
                onClick={() => {
                  if (breach.value !== null) {
                    editingBreach.value = true;
                    return;
                  }
                  const at = placeAt();
                  onBridge(async () => at !== null && (await FenceBridge.setBreachReturn(at)), { label: 'Adding breach return point' });
                }}
              />
            </>
          )
        ) : (
          <RailButton
            icon="plan_add"
            label="Add point"
            enabled={support.rally && placeAt() !== null}
            onClick={() => {
              const at = placeAt();
              if (at !== null) addRallyAt(at.latitude, at.longitude);
            }}
          />
        )}
      </PlanRail>

      <PlanRail className="absolute" style={{ right: 8, top: railTop, maxHeight: railMaxHeight }}>
        <RailButton icon="plan_undo" label="Undo" enabled={canUndo} onClick={() => onBridge(() => invokeOk(PLAN_UNDO))} />
        <CenterMenu
          launch={(() => {
            const start = allItems.value.find((it) => it.sequence === 0);
            return start === undefined ? null : TrackPoint(start.latitude, start.longitude);
          })()}
          myLocation={operator.value}
          onFit={(points) => {
            follow.value = false;
            fitOnly.value = points;
            fitRequest.value += 1;
          }}
          onCentre={(point) => {
            follow.value = false;
            centreOn.value = point;
            centreRequest.value += 1;
          }}
          missionPoints={missionFitPoints(allItems.value)}
          following={follow.value}
          onFollow={() => {
            follow.value = !follow.value;
          }}
          anchor={(open) => <RailButton icon="plan_my_location" label="Centre" chosen={follow.value} onClick={open} />}
        />
        <MapTypeMenu
          onStyle={(style) => {
            shownStyle.value = style;
          }}
        />
      </PlanRail>

      {loadArmed.value ? (
        <AlertDialog
          onDismissRequest={() => {
            loadArmed.value = false;
          }}
          title="Load plan from vehicle?"
          text={replaceWarning(allItems.value.filter((it) => it.index !== HOME_ITEM).length)}
          confirmButton={<TextButton onClick={download}>Replace</TextButton>}
          dismissButton={
            <TextButton
              onClick={() => {
                loadArmed.value = false;
              }}
            >
              Keep mine
            </TextButton>
          }
        />
      ) : null}

      {farAsk.value && aircraftAt !== null ? (
        <AlertDialog
          onDismissRequest={() => {
            farAsk.value = false;
          }}
          title={farText ?? ''}
          text={FAR_FROM_AIRCRAFT_TEXT}
          confirmButton={
            <Button
              onClick={() => {
                farAsk.value = false;
                onBridge(() => repositionMission(aircraftAt), {
                  label: 'Moving the route to the aircraft',
                  done: 'Route moved to the aircraft',
                  then: () => {
                    follow.value = false;
                    centreOn.value = aircraftAt;
                    centreRequest.value += 1;
                  },
                });
              }}
            >
              Move it to the aircraft
            </Button>
          }
          dismissButton={
            <TextButton
              onClick={() => {
                farAsk.value = false;
                sendUpload();
              }}
            >
              Upload anyway
            </TextButton>
          }
        />
      ) : null}

      {uploadAsk.value === null ? null : (
        <AlertDialog
          onDismissRequest={() => {
            uploadAsk.value = null;
          }}
          title={uploadHeading(uploadAsk.value, vehicleChoices(vehiclesJson))}
          text={uploadAsk.value.refusal}
          confirmButton={
            <TextButton
              onClick={() => {
                const pauses = uploadAsk.value?.pausesFirst ?? false;
                uploadAsk.value = null;
                sendPlan(
                  (next) => {
                    busy.value = next;
                  },
                  () => {
                    busy.value = null;
                  },
                  pauses,
                );
              }}
            >
              {uploadAsk.value.proceedTitle.trim() === '' ? 'Upload' : uploadAsk.value.proceedTitle}
            </TextButton>
          }
          dismissButton={
            <TextButton
              onClick={() => {
                uploadAsk.value = null;
              }}
            >
              Cancel
            </TextButton>
          }
        />
      )}

      {patternWanted.value.length === 0 ? null : (
        <AlertDialog
          onDismissRequest={() => {
            patternWanted.value = [];
          }}
          title="Which pattern?"
          text={patternWanted.value.find((it) => !addablePattern(it) && it.disabledReason.trim() !== '')?.disabledReason ?? 'A pattern covers an area or a line with a camera run.'}
          confirmButton={
            <div className="flex flex-col">
              {patternWanted.value.map((kind) => (
                <TextButton
                  key={kind.id}
                  enabled={addablePattern(kind)}
                  onClick={() => {
                    patternWanted.value = [];
                    addMissionItem(kind.id, `Adding ${kind.label.toLowerCase()}`, placeAt(), insertAfter(selected.value, allItems.value));
                  }}
                >
                  {kind.label}
                </TextButton>
              ))}
            </div>
          }
          dismissButton={
            <TextButton
              onClick={() => {
                patternWanted.value = [];
              }}
            >
              Cancel
            </TextButton>
          }
        />
      )}

      {breach.value !== null && editingBreach.value ? (
        <BreachReturnDialog
          breach={breach.value}
          onDismiss={() => {
            editingBreach.value = false;
          }}
          onAltitude={(shown) => {
            const path = breach.value?.altitudePath ?? '';
            editingBreach.value = false;
            onBridge(() => FenceBridge.setBreachAltitude(path, shown), { label: 'Setting breach return altitude' });
          }}
          onRemove={() => {
            editingBreach.value = false;
            onBridge(() => FenceBridge.clearBreachReturn(), { label: 'Removing breach return point' });
          }}
        />
      ) : null}

      <div
        ref={sidePanel ? undefined : controlsRef}
        className={sidePanel ? 'absolute left-0 top-0 h-full' : 'absolute inset-x-0 bottom-0 rounded-t-[28px]'}
        style={{ width: sidePanel ? SIDE_PANEL_WIDTH : undefined, background: md('surfaceContainerLow'), color: md('onSurface') }}
      >
        <div className={`flex flex-col px-3 ${sidePanel ? 'h-full pt-3' : ''}`} style={sidePanel ? undefined : { maxHeight: screenHeight * PANEL_MAX_FRACTION }}>
          <div
            className="flex w-full shrink-0 flex-col"
            {...panelDrag((open) => {
              panelOpen.value = open;
            })}
          >
            {sidePanel ? null : (
              <button
                type="button"
                aria-label={panelOpen.value ? 'Fold the panel' : 'Open the panel'}
                className="flex h-6 w-full items-center justify-center"
                onClick={() => {
                  panelOpen.value = !panelOpen.value;
                }}
              >
                <span className="h-1 w-8 rounded-full" style={{ background: withAlpha(md('onSurfaceVariant'), 0.4) }} />
              </button>
            )}
            {layer.value === PlanLayer.Mission && itemCount.value > 0 && (layerOf(chosen) === null || layerOf(chosen) === PlanLayer.Mission) ? (
              <WaypointStripBar
                rows={rows}
                altitudes={altitudes}
                conflicts={conflicts}
                selected={chosenItem?.index ?? null}
                onPick={focusItem}
                onList={
                  sidePanel
                    ? null
                    : () => {
                        listOpen.value = true;
                      }
                }
                profileShown={profileShown(layer.value, profile) ? missionStatusShown : null}
                onProfile={() => onBridge(() => setOk(SHOW_MISSION_ITEM_STATUS, settingJson((!missionStatusShown).toString())))}
              />
            ) : null}
            {chosenItem !== null ? (
              <SelectionHeader
                title={itemTitle(chosenItem)}
                detail={chosenDetail}
                warning={conflicts.has(chosenItem.index)}
                open={panelOpen.value}
                onTitle={() => {
                  panelOpen.value = !panelOpen.value;
                }}
                onDone={() => {
                  selected.value = null;
                }}
              />
            ) : chosen !== null ? (
              <SelectionHeader
                title={selectionTitle(chosen, allItems.value)}
                detail={selectionText(chosen, allItems.value, circles.value, fences.value)}
                warning={false}
                open={panelOpen.value}
                onTitle={() => {
                  panelOpen.value = !panelOpen.value;
                }}
                onDone={() => {
                  selected.value = null;
                }}
              />
            ) : layer.value === PlanLayer.Mission && itemCount.value === 0 ? (
              <EmptyMissionStrip homeSet={homeSet} onTemplates={onTemplates} onDownload={planOffline ? null : requestDownload} />
            ) : layer.value !== PlanLayer.Mission ? (
              <Text typography="titleMedium" className="block pb-1 pl-3">
                {layer.value.label}
              </Text>
            ) : null}
          </div>
          {panelOpen.value || sidePanel ? (
            <div ref={panelScroll} className="flex min-h-0 flex-col overflow-y-auto [&>*]:shrink-0">
              {chosen === null && layer.value === PlanLayer.Mission && missionStatusShown && profileShown(layer.value, profile) && itemCount.value > 0 ? (
                <TerrainProfileView
                  profile={profile}
                  notice={elevationNotice}
                  selectedSequence={selectedSequence}
                  onSelect={(sequence) => {
                    const item = allItems.value.find((it) => it.sequence === sequence);
                    if (item !== undefined) selected.value = { kind: 'Waypoint', index: item.index };
                  }}
                />
              ) : null}
              {chosenItem !== null ? (
                <>
                  {tiles.length === 0 ? null : (
                    <div className="flex gap-2 px-3 pb-3">
                      {tiles.map(([label, value]) => (
                        <StatTile key={label} label={label} value={value} />
                      ))}
                    </div>
                  )}
                  <WaypointSettings item={chosenItem} globalFrame={globalFrame} onWrite={(label, work) => onBridge(work, { label })} />
                </>
              ) : chosen !== null ? null : layer.value === PlanLayer.Mission ? (
                routeSettings?.()
              ) : layer.value === PlanLayer.Rally ? (
                <>
                  {support.rallyRefused ? <PaletteNote text={RALLY_NOT_SUPPORTED} /> : rally.value.length === 0 ? <PaletteNote text={NO_RALLY_POINTS} /> : null}
                  {rally.value.length === 0 ? null : <FenceHeading text="Rally points" />}
                  {rallyRows(rally.value).map((row) => (
                    <FenceListRow
                      key={row.index}
                      row={row}
                      chosen={false}
                      onSelect={() => {
                        selected.value = { kind: 'Rally', index: row.index };
                      }}
                      onRemove={() => {
                        const count = rally.value.length;
                        onBridge(() => FenceBridge.removeRallyPoint(row.index), {
                          label: `Removing ${row.title.toLowerCase()}`,
                          then: () => {
                            selected.value = rallyAfterRemove(row.index, count);
                          },
                        });
                      }}
                    />
                  ))}
                </>
              ) : (
                <>
                  {support.fenceRefused ? <PaletteNote text={GEOFENCE_NOT_SUPPORTED} /> : fences.value.length === 0 && circles.value.length === 0 ? <PaletteNote text={NO_GEOFENCE} /> : null}
                  {listedFences.map((row, at) => {
                    const heading = fenceHeading(row, listedFences[at - 1] ?? null);
                    const inclusion = row.inclusion;
                    return (
                      <Fragment key={`${row.circle ? 'c' : 'p'}${row.index}`}>
                        {heading === null ? null : <FenceHeading text={heading} />}
                        <FenceListRow
                          row={row}
                          chosen={false}
                          onSelect={() => {
                            selected.value = fenceRowHit(row);
                          }}
                          onInclusion={inclusion === null ? null : (keep) => onBridge(() => FenceBridge.setPolygonInclusion(row.index, keep), { label: `Changing ${row.title.toLowerCase()}` })}
                          onRemove={() => {
                            onBridge(() => (row.circle ? FenceBridge.deleteCircle(row.index) : FenceBridge.deletePolygon(row.index)), {
                              label: `Removing ${row.title.toLowerCase()}`,
                              then: () => {
                                selected.value = fenceSelectionAfterRemove(row, selected.value);
                              },
                            });
                          }}
                        />
                      </Fragment>
                    );
                  })}
                </>
              )}
              {listedInPanel ? (
                <>
                  <ItemListHeading rows={rows} summary={missionSummaryText(missionSummaryView)} className="px-3 py-1" />
                  {rows.map((row) => (
                    <ItemRowView key={row.index} row={row} selected={chosen?.kind === 'Waypoint' && chosen.index === row.index} onClick={() => pickRow(row)} />
                  ))}
                </>
              ) : null}

              {traced === null ? null : (
                <div className="flex items-center">
                  <Text typography="labelSmall">{traceCaption(traced[1].length, traced[0].minimum)}</Text>
                  <TextButton
                    enabled={traced[1].length >= traced[0].minimum}
                    onClick={() => {
                      tracing.value = null;
                      onBridge(() => replaceShape(traced[0], traced[1]), { label: 'Tracing shape' });
                    }}
                  >
                    Done
                  </TextButton>
                  <TextButton
                    onClick={() => {
                      tracing.value = null;
                    }}
                  >
                    Cancel
                  </TextButton>
                </div>
              )}

              {showsTools ? (
                <div className="flex w-full flex-wrap items-start gap-1">
                  {fence === null || fence.flip === null ? null : (
                    <TextButton
                      onClick={() => {
                        const flip = fence.flip;
                        if (flip !== null) onBridge(flip, { label: fence.keepsIn ? 'Making it keep-out' : 'Making it keep-in' });
                      }}
                    >
                      {fence.keepsIn ? 'Make keep-out' : 'Make keep-in'}
                    </TextButton>
                  )}

                  {fenceText === null ? null : <PaletteNote text={fenceText} />}

                  {landingNote === null ? null : <PaletteNote text={landingNote} />}

                  {cameraNote === null ? null : <PaletteNote text={cameraNote} />}

                  {layersNote === null ? null : <PaletteNote text={layersNote} />}

                  {survey !== null && survey.kind === KIND_SURVEY ? <GridAngle key={survey.index} index={survey.index} onBridge={onBridge} /> : null}

                  {survey !== null && visible.value.length === survey.area.length ? (
                    <TextButton onClick={() => onBridge(() => fitSurveyArea(survey, insetRing(visible.value, SURVEY_FIT_INSET)), { label: 'Sizing the area', done: 'Area sized to the view' })}>Size to view</TextButton>
                  ) : null}

                  {cornerHit !== null && corner !== null ? (
                    <TextButton
                      onClick={() => {
                        positioning.value = [cornerHit, corner];
                      }}
                    >
                      Edit position
                    </TextButton>
                  ) : null}

                  {target === null ? null : (
                    <>
                      {targetShape === null ? null : <Text typography="labelSmall">{shapeCaption(targetShape, circled.has(target.path))}</Text>}
                      {target.line ? (
                        <TextButton enabled={visible.value.length === 4} onClick={() => onBridge(() => replaceShape(target, defaultLine(visible.value)), { label: 'Drawing line' })}>
                          Line
                        </TextButton>
                      ) : (
                        <>
                          <TextButton
                            enabled={visible.value.length === 4}
                            onClick={() => {
                              chosenCircles.value = new Set([...chosenCircles.value].filter((it) => it !== target.path));
                              onBridge(() => replaceShape(target, defaultRectangle(visible.value)), { label: 'Drawing rectangle' });
                            }}
                          >
                            Rectangle
                          </TextButton>
                          <TextButton
                            enabled={visible.value.length === 4}
                            onClick={() => {
                              chosenCircles.value = new Set([...chosenCircles.value, target.path]);
                              onBridge(() => replaceShape(target, defaultCircle(visible.value)), { label: 'Drawing circle' });
                            }}
                          >
                            Circle
                          </TextButton>
                          {circled.has(target.path) ? (
                            <>
                              <TextButton
                                onClick={() => {
                                  radiusFor.value = target;
                                }}
                              >
                                Set radius{'…'}
                              </TextButton>
                              {targetCentre === null ? null : (
                                <TextButton
                                  onClick={() => {
                                    positioning.value = targetCentre;
                                  }}
                                >
                                  Edit position{'…'}
                                </TextButton>
                              )}
                            </>
                          ) : null}
                        </>
                      )}
                      <TextButton
                        onClick={() => {
                          tracing.value = [target, []];
                        }}
                      >
                        Trace
                      </TextButton>
                      <TextButton
                        onClick={() => {
                          chosenCircles.value = new Set([...chosenCircles.value].filter((it) => it !== target.path));
                          void importInto(target);
                        }}
                      >
                        Import{'…'}
                      </TextButton>
                    </>
                  )}

                  {fenceHit === null ? null : (
                    <>
                      {cornerRemovable(fences.value.find((it) => it.index === fenceHit.polygon) ?? null) ? (
                        <TextButton
                          onClick={() => {
                            onBridge(() => FenceBridge.removeVertex(fenceHit.polygon, fenceHit.vertex), { label: 'Removing corner' });
                            selected.value = null;
                          }}
                        >
                          Remove vertex
                        </TextButton>
                      ) : null}
                      <TextButton
                        onClick={() => {
                          onBridge(() => FenceBridge.deletePolygon(fenceHit.polygon));
                          selected.value = null;
                        }}
                      >
                        Delete fence
                      </TextButton>
                    </>
                  )}

                  {surveyHit === null ? null : (
                    <>
                      {survey !== null && survey.editable?.canRemoveVertex === true ? (
                        <TextButton
                          onClick={() => {
                            onBridge(() => SurveyBridge.removeVertex(survey, surveyHit.vertex), { label: 'Removing corner' });
                            selected.value = null;
                          }}
                        >
                          Remove vertex
                        </TextButton>
                      ) : null}
                      <SurveyAltitudeField key={surveyHit.item} item={surveyHit.item} say={say} onBridge={onBridge} />
                    </>
                  )}

                  {deletedPattern === null ? null : (
                    <TextButton
                      onClick={() => {
                        onRefusal(() => PlanBridge.removeItemRefusal(deletedPattern));
                        selected.value = null;
                      }}
                    >
                      Delete {patternName(deletedPattern, allItems.value)}
                    </TextButton>
                  )}

                  {circle === null ? null : <CircleTools key={`${circle.index}:${circle.radius}`} circle={circle} say={say} onBridge={onBridge} onDeleted={() => {
                        selected.value = null;
                      }} />}

                  {rallyHit === null ? null : (
                    <>
                      {rallyPoint === null ? null : (
                        <>
                          <RallyCoordinateField key={`${rallyPoint.index}:Latitude:${rallyPoint.latitude}`} point={rallyPoint} label="Latitude" value={rallyPoint.latitude} limit={LATITUDE_LIMIT} say={say} onBridge={onBridge} />
                          <RallyCoordinateField key={`${rallyPoint.index}:Longitude:${rallyPoint.longitude}`} point={rallyPoint} label="Longitude" value={rallyPoint.longitude} limit={LONGITUDE_LIMIT} say={say} onBridge={onBridge} />
                        </>
                      )}
                      {rallyPoint !== null && rallyAltitudeIsEditable(rallyPoint) ? <RallyAltitudeField key={`${rallyPoint.index}:${rallyPoint.altitude}`} point={rallyPoint} say={say} onBridge={onBridge} /> : null}
                      <TextButton
                        onClick={() => {
                          const count = rally.value.length;
                          onBridge(() => FenceBridge.removeRallyPoint(rallyHit.index), {
                            then: () => {
                              selected.value = rallyAfterRemove(rallyHit.index, count);
                            },
                          });
                        }}
                      >
                        Delete rally point
                      </TextButton>
                    </>
                  )}
                </div>
              ) : null}

              {chosenItem === null ? null : (
                <Fragment key={removals.value}>
                  {itemPanel?.(chosenItem.index, chosenItem.placed ? TrackPoint(chosenItem.latitude, chosenItem.longitude) : null, advancedDetail(chosenItem), chosenItem.index > HOME_ITEM ? () => removeItem(chosenItem) : null)}
                </Fragment>
              )}
            </div>
          ) : null}
        </div>
      </div>

      {listOpen.value ? (
        <AircastSheet
          onDismissRequest={() => {
            listOpen.value = false;
          }}
        >
          <ItemListHeading rows={rows} summary={missionSummaryText(missionSummaryView)} className="px-6 py-2" />
          <div className="flex w-full flex-col pb-6">
            {rows.map((row) => (
              <ItemRowView
                key={row.index}
                row={row}
                selected={chosen?.kind === 'Waypoint' && chosen.index === row.index}
                onClick={() => {
                  pickRow(row);
                  listOpen.value = false;
                }}
              />
            ))}
          </div>
        </AircastSheet>
      ) : null}
    </div>
  );
};

const SURVEY_FIT_INSET = 0.8;

const ITEM_MARKER_SIZE = 40;

const GridAngle = ({ index, onBridge }: { index: number; onBridge: OnBridge }) => {
  const [angle, setAngle] = useState<number | null>(null);
  const latest = useRef<number | null>(null);
  useLaunchedEffect([index], async (scope) => {
    const shown = gridAngleShown(await SurveyBridge.gridAngle(index));
    if (!scope.isActive()) return;
    latest.current = shown;
    setAngle(shown);
  });
  return angle === null ? null : (
    <div className="flex w-[220px] flex-col">
      <Text typography="labelSmall">Angle {Math.trunc(angle)}°</Text>
      <Slider
        value={angle}
        valueRange={[0, GRID_ANGLE_MAX]}
        steps={stepsFor(0, GRID_ANGLE_MAX, 1)}
        label="Grid angle"
        onValueChange={(next) => {
          latest.current = Math.round(next);
          setAngle(latest.current);
        }}
        onValueChangeFinished={() => {
          const chosen = latest.current;
          if (chosen !== null) onBridge(() => SurveyBridge.setGridAngle(index, chosen), { label: 'Setting the grid angle' });
        }}
      />
    </div>
  );
};

const SurveyAltitudeField = ({ item, say, onBridge }: { item: number; say: Say; onBridge: OnBridge }) => {
  const [surveyAlt, setSurveyAlt] = useState('');
  const [surveyUnit, setSurveyUnit] = useState('m');
  useLaunchedEffect([item], async (scope) => {
    const shown = await SurveyBridge.altitude(item);
    const unit = await SurveyBridge.altitudeUnits(item);
    if (!scope.isActive()) return;
    setSurveyAlt(altitudeFieldText(shown, SURFACE_DISTANCE_DECIMALS));
    setSurveyUnit(unit.trim() === '' ? 'm' : unit);
  });
  return (
    <OutlinedTextField
      value={surveyAlt}
      onValueChange={setSurveyAlt}
      label={`Above surface ${surveyUnit}`}
      singleLine
      keyboardType="decimal"
      onDone={() => {
        const shown = parsedSurfaceDistance(surveyAlt, metresPerUnit(surveyUnit));
        if (shown === null) say('Not an altitude');
        else onBridge(() => SurveyBridge.setAltitude(item, shown), { label: 'Setting survey altitude' });
      }}
      className="w-[150px] [&_input]:text-body-small"
    />
  );
};

const CircleTools = ({ circle, say, onBridge, onDeleted }: { circle: FenceCircle; say: Say; onBridge: OnBridge; onDeleted: () => void }) => {
  const bigger = grownRadius(circle);
  const smaller = shrunkRadius(circle);
  const [typedCircleRadius, setTypedCircleRadius] = useState(() => trimmedRadius(circle.radius));
  return (
    <>
      <OutlinedTextField
        value={typedCircleRadius}
        onValueChange={setTypedCircleRadius}
        label="Radius"
        suffix={circle.radiusUnits}
        singleLine
        keyboardType="decimal"
        onDone={() => {
          const wanted = typedRadius(typedCircleRadius, circle);
          if (wanted === null) say('Not a radius this fence accepts');
          else onBridge(() => FenceBridge.setCircleRadius(circle.index, wanted));
        }}
        className="w-[110px] [&_input]:text-body-small"
      />
      <TextButton
        enabled={bigger !== null}
        onClick={() => {
          if (bigger !== null) onBridge(() => FenceBridge.setCircleRadius(circle.index, bigger));
        }}
      >
        Bigger
      </TextButton>
      <TextButton
        enabled={smaller !== null}
        onClick={() => {
          if (smaller !== null) onBridge(() => FenceBridge.setCircleRadius(circle.index, smaller));
        }}
      >
        Smaller
      </TextButton>
      <TextButton
        onClick={() => {
          onBridge(() => FenceBridge.deleteCircle(circle.index));
          onDeleted();
        }}
      >
        Delete circle
      </TextButton>
    </>
  );
};

const doubleText = (value: number): string => (Number.isInteger(value) ? value.toFixed(1) : String(value));

const RallyCoordinateField = ({ point, label, value, limit, say, onBridge }: { point: RallyPoint; label: string; value: number; limit: number; say: Say; onBridge: OnBridge }) => {
  const [typed, setTyped] = useState(() => doubleText(value));
  return (
    <OutlinedTextField
      value={typed}
      onValueChange={setTyped}
      label={label}
      singleLine
      keyboardType="text"
      onDone={() => {
        const entered = parsedCoordinateWithin(typed, limit);
        if (entered === null) {
          say(`Not a ${label}`);
          return;
        }
        const [lat, lon] = label === 'Latitude' ? [entered, point.longitude] : [point.latitude, entered];
        onBridge(() => FenceBridge.moveRallyPoint(point.index, lat, lon, point.altitudeMetres), { label: 'Moving rally point' });
      }}
      className="w-[150px] [&_input]:text-body-small"
    />
  );
};

const RallyAltitudeField = ({ point, say, onBridge }: { point: RallyPoint; say: Say; onBridge: OnBridge }) => {
  const [typed, setTyped] = useState(() => altitudeFieldText(point.altitude, RALLY_ALTITUDE_DECIMALS));
  return (
    <OutlinedTextField
      value={typed}
      onValueChange={setTyped}
      label={rallyAltitudeLabel(point)}
      singleLine
      keyboardType="text"
      onDone={() => {
        const shown = parsedAltitude(typed);
        if (shown === null) say('Not an altitude');
        else onBridge(() => FenceBridge.setRallyAltitude(point.altitudePath, shown), { label: 'Setting altitude' });
      }}
      className="w-[120px] [&_input]:text-body-small"
    />
  );
};

const ItemListHeading = ({ rows, summary, className }: { rows: readonly ItemRow[]; summary: string; className?: string }) => (
  <div className={`flex flex-col ${className ?? ''}`}>
    <Text typography="titleMedium">{itemCountText(rows.filter((it) => it.index !== HOME_ITEM).length)}</Text>
    {summary.trim() === '' ? null : (
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {summary}
      </Text>
    )}
  </div>
);

const ItemRowView = ({ row, selected, onClick }: { row: ItemRow; selected: boolean; onClick: () => void }) => (
  <Surface
    onClick={onClick}
    color={selected ? md('secondaryContainer') : md('surfaceContainerLow')}
    contentColor={selected ? md('onSecondaryContainer') : md('onSurface')}
    className="flex min-h-[72px] w-full items-center gap-4 pb-2 pl-4 pr-3 pt-2"
  >
    <span
      className="flex shrink-0 items-center justify-center rounded-full"
      style={{ width: ITEM_MARKER_SIZE, height: ITEM_MARKER_SIZE, ...(row.readyForSave ? { background: row.colour } : { border: `1px solid ${aircast('warning')}` }) }}
    >
      <Text typography="labelLarge" color={row.readyForSave ? md('surface') : aircast('warning')} maxLines={1}>
        {row.number}
      </Text>
    </span>
    <span className="flex min-w-0 flex-1 flex-col">
      <Text typography="titleMedium" maxLines={1}>
        {sentenceCase(row.name)}
      </Text>
      {row.detail.trim() === '' ? null : (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')} maxLines={2}>
          {row.detail}
        </Text>
      )}
    </span>
    {selected ? <Text typography="labelMedium">Selected</Text> : null}
    <MaterialIcon name="keyboard_arrow_right" filled style={{ color: md('onSurfaceVariant') }} />
  </Surface>
);

export const MAP_TYPES_VIEW = 'view.mapTypes';

export type MapTypes = { readonly current: string; readonly types: readonly string[]; readonly path: string };

export const mapTypes = (view: Json | null): MapTypes | null =>
  view !== null && has(view, 'types') ? { current: optText(view, 'current'), types: jsonStrings(optJSONArray(view, 'types')), path: optText(view, 'path') } : null;

const MapTypeMenu = ({ onStyle }: { onStyle: (style: string) => void }) => {
  const [open, setOpen] = useState(false);
  const [listed, setListed] = useState<MapTypes | null>(null);
  return (
    <div className="relative">
      <RailButton
        icon="plan_layers"
        label="Map"
        onClick={() => {
          void (async () => {
            setListed(mapTypes(await MapBridge.read(MAP_TYPES_VIEW)));
            setOpen(true);
          })();
        }}
      />
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {(listed?.types ?? []).map((type) => (
          <DropdownMenuItem
            key={type}
            text={type}
            trailingIcon={type === listed?.current ? <Text>✓</Text> : undefined}
            onClick={() => {
              setOpen(false);
              const path = listed?.path;
              if (path === undefined) return;
              void (async () => {
                await setOk(path, settingJson(JSON.stringify(type)));
                onStyle(qgcRasterStyle(await currentMapType()));
              })();
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};

export const missionFitPoints = (items: readonly MissionItem[]): readonly TrackPoint[] =>
  items.filter((it) => isPlottable(it.latitude, it.longitude)).map((it) => TrackPoint(it.latitude, it.longitude));

export const parsedCoordinate = (latitude: string, longitude: string): TrackPoint | null => {
  const lat = parsedCoordinateWithin(latitude, LATITUDE_LIMIT);
  if (lat === null) return null;
  const lon = parsedCoordinateWithin(longitude, LONGITUDE_LIMIT);
  return lon === null ? null : TrackPoint(lat, lon);
};

export const cornerPosition = (hit: MapHit, fences: readonly FencePolygon[], surveys: readonly Survey[]): TrackPoint | null => {
  switch (hit.kind) {
    case 'FenceVertex':
      return fences.find((it) => it.index === hit.polygon)?.vertices[hit.vertex] ?? null;
    case 'SurveyVertex':
      return surveys.find((it) => it.index === hit.item)?.area[hit.vertex] ?? null;
    default:
      return null;
  }
};

const RadiusDialog = ({ radius, unit, metresPerUnit: perUnit, onDismiss, onSet }: { radius: number; unit: string; metresPerUnit: number; onDismiss: () => void; onSet: (radius: number) => void }) => {
  const [text, setText] = useState(() => format('%.1f', radius / perUnit));
  const parsed = circleRadiusMetres(text, perUnit);
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Set radius"
      text={<OutlinedTextField value={text} onValueChange={setText} label={`Radius (${unit})`} singleLine />}
      confirmButton={
        <TextButton enabled={parsed !== null} onClick={() => parsed !== null && onSet(parsed)}>
          Set
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

const PositionDialog = ({ at, title, onDismiss, onMove }: { at: TrackPoint; title: string; onDismiss: () => void; onMove: (moved: TrackPoint) => void }) => {
  const [latitude, setLatitude] = useState(() => format('%.7f', at.latitude));
  const [longitude, setLongitude] = useState(() => format('%.7f', at.longitude));
  const parsed = parsedCoordinate(latitude, longitude);
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title={title}
      text={
        <div className="flex flex-col">
          <OutlinedTextField value={latitude} onValueChange={setLatitude} label="Latitude" singleLine />
          <OutlinedTextField value={longitude} onValueChange={setLongitude} label="Longitude" singleLine />
        </div>
      }
      confirmButton={
        <TextButton enabled={parsed !== null} onClick={() => parsed !== null && onMove(parsed)}>
          Move
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

const CoordinateDialog = ({ onDismiss, onCentre }: { onDismiss: () => void; onCentre: (point: TrackPoint) => void }) => {
  const [latitude, setLatitude] = useState('');
  const [longitude, setLongitude] = useState('');
  const parsed = parsedCoordinate(latitude, longitude);
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Center map on coordinate"
      text={
        <div className="flex flex-col">
          <OutlinedTextField value={latitude} onValueChange={setLatitude} label="Latitude" singleLine />
          <OutlinedTextField value={longitude} onValueChange={setLongitude} label="Longitude" singleLine />
        </div>
      }
      confirmButton={
        <TextButton
          enabled={parsed !== null}
          onClick={() => {
            onDismiss();
            if (parsed !== null) onCentre(parsed);
          }}
        >
          Center
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

export const CenterMenu = ({
  launch,
  myLocation,
  onCentre,
  launchLabel = 'Launch',
  missionPoints = null,
  onFit = () => undefined,
  following = null,
  onFollow = () => undefined,
  anchor = (open) => <FilledTonalButton onClick={open}>Center map</FilledTonalButton>,
}: {
  launch: TrackPoint | null;
  myLocation: TrackPoint | null;
  onCentre: (point: TrackPoint) => void;
  launchLabel?: string;
  missionPoints?: readonly TrackPoint[] | null;
  onFit?: (points: readonly TrackPoint[] | null) => void;
  following?: boolean | null;
  onFollow?: () => void;
  anchor?: (open: () => void) => ReactNode;
}) => {
  const [open, setOpen] = useState(false);
  const [asking, setAsking] = useState(false);
  const fleetJson = useMapPath(VEHICLES_VIEW);
  const active = useMemo(() => vehicleChoices(fleetJson).choices.find((it) => it.active) ?? null, [fleetJson]);
  const vehicle = active !== null && isPlottable(active.latitude, active.longitude) ? TrackPoint(active.latitude, active.longitude) : null;
  const close = (then: () => void) => () => {
    setOpen(false);
    then();
  };
  return (
    <>
      <div className="relative">
        {anchor(() => setOpen(true))}
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {following === null ? null : <DropdownMenuItem text="Follow vehicle" enabled={vehicle !== null} trailingIcon={following ? <MaterialIcon name="check" filled /> : undefined} onClick={close(onFollow)} />}
          {missionPoints === null ? null : (
            <>
              <DropdownMenuItem text="Mission" onClick={close(() => onFit(missionPoints))} />
              <DropdownMenuItem text="All items" onClick={close(() => onFit(null))} />
            </>
          )}
          <DropdownMenuItem text={launchLabel} enabled={launch !== null} onClick={close(() => launch !== null && onCentre(launch))} />
          <DropdownMenuItem text="Vehicle" enabled={vehicle !== null} onClick={close(() => vehicle !== null && onCentre(vehicle))} />
          <DropdownMenuItem text="My location" enabled={myLocation !== null} onClick={close(() => myLocation !== null && onCentre(myLocation))} />
          <DropdownMenuItem text="Coordinates…" onClick={close(() => setAsking(true))} />
        </DropdownMenu>
      </div>
      {asking ? <CoordinateDialog onDismiss={() => setAsking(false)} onCentre={onCentre} /> : null}
    </>
  );
};

const StatTile = ({ label, value }: { label: string; value: string }) => {
  const unit = value.includes(' ') ? value.slice(value.lastIndexOf(' ') + 1) : '';
  const number = (value.endsWith(unit) ? value.slice(0, value.length - unit.length) : value).trim();
  return (
    <div className="flex flex-col pb-1 pr-5 pt-1">
      <Text typography="labelSmall" color={md('onSurfaceVariant')}>
        {label}
      </Text>
      <div className="flex items-end gap-1">
        <Text typography="titleLarge">{number}</Text>
        {unit.trim() === '' ? null : (
          <Text typography="labelMedium" color={md('onSurfaceVariant')}>
            {unit}
          </Text>
        )}
      </div>
    </div>
  );
};

export type PlanLayer = { readonly name: 'Mission' | 'Fence' | 'Rally'; readonly ordinal: number; readonly label: string; readonly icon: DrawableName };

export const PlanLayer = {
  Mission: { name: 'Mission', ordinal: 0, label: 'Mission', icon: 'plan_route' },
  Fence: { name: 'Fence', ordinal: 1, label: 'Fence', icon: 'plan_shield' },
  Rally: { name: 'Rally', ordinal: 2, label: 'Rally', icon: 'plan_flag' },
} as const satisfies Record<string, PlanLayer>;

export const PlanLayerEntries: readonly PlanLayer[] = [PlanLayer.Mission, PlanLayer.Fence, PlanLayer.Rally];

export const ownerOf = (hit: MapHit | null): string | null => {
  switch (hit?.kind) {
    case 'Waypoint':
    case 'LandingPlace':
    case 'LoiterRadius':
    case 'LoiterRotation':
      return `m${hit.index}`;
    case 'SurveyVertex':
      return `m${hit.item}`;
    case 'ShapeCentre':
    case 'ShapeRadius':
      return hit.fence ? `p${hit.owner}` : `m${hit.owner}`;
    case 'FenceVertex':
      return `p${hit.polygon}`;
    case 'Circle':
    case 'CircleCentre':
    case 'CircleRadius':
      return `c${hit.index}`;
    case 'Rally':
      return `r${hit.index}`;
    case 'BreachReturn':
      return 'breach';
    default:
      return null;
  }
};

export const dragAllowed = (hit: MapHit, selected: MapHit | null, layer: PlanLayer): boolean => {
  if (hit.kind === 'Midpoint') {
    const owner = midpointOwner(hit.path);
    return owner === null ? selected?.kind === 'Waypoint' : owner === ownerOf(selected);
  }
  if (hit.kind === 'BreachReturn') return layer === PlanLayer.Fence;
  const owner = ownerOf(hit);
  return layerOf(hit) === layer && owner !== null && owner === ownerOf(selected);
};

export const profileShown = (layer: PlanLayer, profile: TerrainProfile): boolean => layer === PlanLayer.Mission && profile.points.length > 0;

export const actsOnTap = (hit: MapHit): boolean => hit.kind === 'Midpoint' || hit.kind === 'LoiterRotation' || hit.kind === 'BreachReturn';

export const midpointOwner = (path: string): string | null => {
  const owned = (prefix: string, tag: string) => `${tag}${path.slice(prefix.length).split('.')[0] ?? ''}`;
  if (path.startsWith(`${FENCE_POLYGONS}.`)) return owned(`${FENCE_POLYGONS}.`, 'p');
  if (path.startsWith(`${PLAN_ITEMS}.`)) return owned(`${PLAN_ITEMS}.`, 'm');
  return null;
};

export const layerOf = (hit: MapHit | null): PlanLayer | null => {
  switch (hit?.kind) {
    case undefined:
      return null;
    case 'Rally':
      return PlanLayer.Rally;
    case 'FenceVertex':
    case 'Circle':
    case 'CircleCentre':
    case 'CircleRadius':
    case 'BreachReturn':
      return PlanLayer.Fence;
    case 'ShapeCentre':
    case 'ShapeRadius':
      return hit.fence ? PlanLayer.Fence : PlanLayer.Mission;
    case 'Midpoint':
      return null;
    default:
      return PlanLayer.Mission;
  }
};

const FenceListRow = ({
  row,
  chosen,
  onSelect,
  onInclusion = null,
  onRemove,
}: {
  row: FenceRow;
  chosen: boolean;
  onSelect: () => void;
  onInclusion?: ((keep: boolean) => void) | null;
  onRemove: () => void;
}) => (
  <div className="flex w-full items-center rounded-shape-sm" style={{ background: chosen ? md('secondaryContainer') : 'transparent' }}>
    <button type="button" onClick={onSelect} className="flex min-w-0 flex-1 items-center py-1 text-left">
      <span className="flex min-w-0 flex-1 flex-col">
        <Text typography="bodyLarge">{row.title}</Text>
        {row.detail.trim() === '' ? null : (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {row.detail}
          </Text>
        )}
      </span>
      {row.radius.trim() === '' ? null : (
        <Text typography="bodyMedium" className="px-2">
          {row.radius}
        </Text>
      )}
    </button>
    {onInclusion !== null && row.inclusion !== null ? (
      <span className="px-2">
        <Switch checked={row.inclusion} onCheckedChange={onInclusion} label={row.title} />
      </span>
    ) : null}
    <IconButton onClick={onRemove} label={`Remove ${row.title}`}>
      <MaterialIcon name="close" filled />
    </IconButton>
  </div>
);
