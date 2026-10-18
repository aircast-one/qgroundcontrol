import { type Json, type JsonArray, arrayJSONObject, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { coerceAtLeast, replaceFirstCharUpper } from '@/kotlin/Std';
import { altitudeModePath } from './AltitudeModes';
import { coordinateJson, invokeOk, setOk, settingJson } from './BridgeCalls';
import { type LandingPattern, landingPattern } from './LandingPattern';
import { MapBridge } from './MapBridge';
import { removeMissionItem } from './MissionKinds';
import { LANDING_PLACE_APPROACH, LANDING_PLACE_TOUCHDOWN, withDrops } from './MissionLayers';
import { MISSION_CONTROLLER } from './SurveyBridge';
import { type TrackPoint, isPlottable } from './VehicleMap';

export const PLAN_ROOT = 'plan';
export const PLAN_ITEMS = `${PLAN_ROOT}.missionController.visualItems`;
export const PLAN_VIEW = 'view.missionItems(geometry)';

export const KIND_LAND = 'land';
export const KIND_WAYPOINT = 'waypoint';
export const KIND_TAKEOFF = 'takeoff';
export const KIND_DROP = 'drop';
export const KIND_SURVEY = 'survey';
export const KIND_CORRIDOR = 'corridor';
export const KIND_STRUCTURE = 'structure';

const INT_MAX_VALUE = 2147483647;

export type MissionKind = {
  readonly id: string;
  readonly label: string;
  readonly title: string;
  readonly enabled: boolean;
  readonly disabledReason: string;
};

export const missionKinds = (view: Json | null): readonly MissionKind[] =>
  jsonObjects(optJSONArray(view, 'kinds')).map((kind) => ({
    id: optText(kind, 'id'),
    label: optText(kind, 'complexName'),
    title: optText(kind, 'title'),
    enabled: optBoolean(kind, 'enabled', true),
    disabledReason: optText(kind, 'disabledReason'),
  }));

export const scanPatterns = (kinds: readonly MissionKind[]): readonly MissionKind[] => kinds.filter((it) => it.label.trim() !== '');

export const kindOffered = (kinds: readonly MissionKind[], id: string): boolean => kinds.length === 0 || kinds.some((it) => it.id === id);

export const kindAllows = (kinds: readonly MissionKind[], id: string): boolean => kinds.find((it) => it.id === id)?.enabled ?? true;

export const kindLabel = (kinds: readonly MissionKind[], id: string): string => {
  const title = kinds.find((it) => it.id === id)?.title ?? '';
  return title.trim() === '' ? replaceFirstCharUpper(id) : title;
};

export const blockedReason = (kinds: readonly MissionKind[]): string | null =>
  kinds.find((it) => !it.enabled && it.disabledReason.trim() !== '')?.disabledReason ?? null;

export const MAV_CMD_NAV_RETURN_TO_LAUNCH = 20;

export const planItems = (json: Json | null): JsonArray | null => optJSONArray(json, 'items');

export const planItemCount = (json: Json | null): number => coerceAtLeast((planItems(json)?.length ?? 0) - 1, 0);

export const linksStartToHome = (json: Json | null): boolean => optBoolean(json, 'linksStartToHome');

const indices = (items: JsonArray | null, from = 0): readonly number[] => Array.from({ length: Math.max((items?.length ?? 0) - from, 0) }, (_, at) => at + from);

export const planShape = (json: Json | null): readonly string[] => {
  const items = planItems(json);
  if (items === null) return [];
  const endsAfter = routeEndsAfter(items);
  const named = indices(items, 1)
    .flatMap((index) => {
      const element = arrayJSONObject(items, index);
      if (element === null) return [];
      if (optText(element, 'kind') === KIND_TAKEOFF) return ['takeoff'];
      if (optInt(element, 'command') === MAV_CMD_NAV_RETURN_TO_LAUNCH) return ['RTL'];
      return [];
    })
    .filter((name, at, all) => all.indexOf(name) === at);
  const stranded = indices(items, 1).filter((it) => it > endsAfter).length;
  return [...named, ...(stranded > 0 ? [`${stranded} after RTL`] : [])];
};

export type MissionItem = {
  index: number;
  sequence: number;
  latitude: number;
  longitude: number;
  command: string;
  selected: boolean;
  altitude: number;
  exit: TrackPoint | null;
  routed: boolean;
  kind: string;
  commandId: number;
  placed: boolean;
  afterRouteEnds: boolean;
  altitudeText: string;
  specifiesCoordinate: boolean;
  distance: number;
  distanceText: string;
  azimuthText: string;
  headingText: string;
  altitudeChange: number;
  altitudeChangeText: string;
  gradientText: string;
  altitudeBandText: string;
  blockedReason: string;
  readyForSave: boolean;
  cameraShots: number;
  extraSeconds: number;
  speedChangeText: string;
  foldedCommands: number;
  altitudeFrameText: string;
  altitudeMode: number;
  altitudeEditUnits: string;
  complexPattern: boolean;
  loiterRadius: number;
  terrainCollision: boolean;
  heading: number;
  gimbalYaw: number;
  closesRoute: boolean;
  legBroken: boolean;
  abbreviation: string;
  drops: boolean;
};

type MissionItemRequired = Pick<MissionItem, 'index' | 'sequence' | 'latitude' | 'longitude' | 'command' | 'selected'>;

export const MissionItem = (fields: MissionItemRequired & Partial<MissionItem>): MissionItem => ({
  altitude: NaN,
  exit: null,
  routed: true,
  kind: '',
  commandId: 0,
  placed: true,
  afterRouteEnds: false,
  altitudeText: '',
  specifiesCoordinate: false,
  distance: NaN,
  distanceText: '',
  azimuthText: '',
  headingText: '',
  altitudeChange: NaN,
  altitudeChangeText: '',
  gradientText: '',
  altitudeBandText: '',
  blockedReason: '',
  readyForSave: true,
  cameraShots: 0,
  extraSeconds: 0.0,
  speedChangeText: '',
  foldedCommands: 0,
  altitudeFrameText: '',
  altitudeMode: -1,
  altitudeEditUnits: '',
  complexPattern: false,
  loiterRadius: NaN,
  terrainCollision: false,
  heading: NaN,
  gimbalYaw: NaN,
  closesRoute: false,
  legBroken: false,
  abbreviation: '',
  drops: false,
  ...fields,
});

const isReturn = (element: Json | null): boolean => optBoolean(element, 'endsRoute') && optInt(element, 'command') === MAV_CMD_NAV_RETURN_TO_LAUNCH;

const isLanding = (element: Json | null): boolean => optBoolean(element, 'endsRoute') && optInt(element, 'command') !== MAV_CMD_NAV_RETURN_TO_LAUNCH;

export const routeEndsAfter = (items: JsonArray | null): number => indices(items).find((index) => isReturn(arrayJSONObject(items, index))) ?? INT_MAX_VALUE;

export const legsAfterLanding = (items: JsonArray | null): ReadonlySet<number> => {
  const flown = indices(items).filter((it) => optBoolean(arrayJSONObject(items, it), 'flownLeg'));
  return new Set(flown.slice(1).filter((_, at) => isLanding(arrayJSONObject(items, flown[at] ?? -1))));
};

export const placed = (element: Json, key: string): TrackPoint | null => {
  const at = optJSONObject(element, key);
  if (at === null) return null;
  const latitude = optDouble(at, 'latitude', NaN);
  const longitude = optDouble(at, 'longitude', NaN);
  return isPlottable(latitude, longitude) ? { latitude, longitude } : null;
};

export const allMissionItems = (json: Json | null): readonly MissionItem[] => {
  const items = planItems(json);
  if (items === null) return [];
  const endsAfter = routeEndsAfter(items);
  const broken = legsAfterLanding(items);
  return indices(items).flatMap((index) => {
    const element = arrayJSONObject(items, index);
    if (element === null) return [];
    const at = placed(element, 'coordinate');
    const exit = placed(element, 'exitCoordinate');
    return [
      MissionItem({
        index,
        sequence: optInt(element, 'sequence', index),
        latitude: at?.latitude ?? NaN,
        longitude: at?.longitude ?? NaN,
        command: optText(element, 'name'),
        kind: optText(element, 'kind'),
        commandId: optInt(element, 'command'),
        selected: optBoolean(element, 'selected'),
        altitude: optDouble(element, 'altitude', NaN),
        altitudeText: optText(element, 'altitudeText'),
        specifiesCoordinate: optBoolean(element, 'specifiesCoordinate'),
        distance: optDouble(element, 'distance', NaN),
        distanceText: optText(element, 'distanceText'),
        azimuthText: optText(element, 'azimuthText'),
        headingText: optText(element, 'headingText'),
        altitudeChange: optDouble(element, 'altitudeChange', NaN),
        altitudeChangeText: optText(element, 'altitudeChangeText'),
        gradientText: optText(element, 'gradientText'),
        altitudeBandText: optText(element, 'altitudeBandText'),
        blockedReason: optText(element, 'blockedReason'),
        readyForSave: !optBoolean(element, 'blocked') && !optBoolean(element, 'awaitingTerrain'),
        cameraShots: optInt(element, 'cameraShots'),
        extraSeconds: optDouble(element, 'extraSeconds', 0.0),
        speedChangeText: optText(element, 'speedChangeText'),
        foldedCommands: optInt(element, 'foldedCommands'),
        altitudeFrameText: optText(element, 'altitudeFrameText'),
        altitudeMode: optInt(element, 'altitudeMode', -1),
        altitudeEditUnits: optText(element, 'altitudeEditUnits'),
        complexPattern: !optBoolean(element, 'simple', true),
        loiterRadius: optDouble(element, 'loiterRadius', NaN),
        heading: optDouble(element, 'heading', NaN),
        gimbalYaw: optDouble(element, 'gimbalYaw', NaN),
        closesRoute: optBoolean(element, 'closesRoute'),
        legBroken: broken.has(index),
        abbreviation: optText(element, 'abbreviation'),
        routed: optBoolean(element, 'flownLeg') && index <= endsAfter,
        afterRouteEnds: index > endsAfter,
        placed: at !== null,
        exit: exit !== null && (at === null || exit.latitude !== at.latitude || exit.longitude !== at.longitude) ? exit : null,
      }),
    ];
  });
};

export const missionItems = (json: Json | null): readonly MissionItem[] => withDrops(allMissionItems(json)).filter((it) => it.placed);

export const PlanBridge = {
  rawItems: (): Promise<Json | null> => MapBridge.read(PLAN_VIEW),

  loadFromVehicle: (): Promise<boolean> => invokeOk(`${PLAN_ROOT}.loadFromVehicle`),

  sendToVehicle: (): Promise<boolean> => invokeOk(`${PLAN_ROOT}.sendToVehicle`),

  pauseVehicle: (): Promise<boolean> => invokeOk('vehicle.pauseVehicle'),

  setAltitude: (index: number, shown: number): Promise<boolean> => setOk(`${PLAN_ITEMS}.${index}.altitude`, settingJson(`${shown}`)),

  setAltitudeMode: (index: number, raw: number): Promise<boolean> => setOk(altitudeModePath(index), settingJson(`${raw}`)),

  removeItem: async (index: number): Promise<boolean> => (await removeMissionItem(index)).ok,

  removeItemRefusal: async (index: number): Promise<string | null> => {
    const outcome = await removeMissionItem(index);
    return outcome.ok ? null : outcome.reason;
  },

  selectSequence: (sequence: number): Promise<boolean> => invokeOk(`${MISSION_CONTROLLER}.setCurrentPlanViewSeqNum`, `[${sequence}, true]`),

  setLoiterRadius: (index: number, radius: number): Promise<boolean> => setOk(`${PLAN_ITEMS}.${index}.loiterRadius`, settingJson(`${radius}`)),

  moveItem: (index: number, latitude: number, longitude: number): Promise<boolean> =>
    setOk(`${PLAN_ITEMS}.${index}.coordinate`, settingJson(coordinateJson(latitude, longitude))),
};

export const takeoffMissing = (items: readonly MissionItem[]): boolean =>
  items.some((it) => it.latitude !== 0.0 || it.longitude !== 0.0) && !items.some((it) => it.kind === 'takeoff');

export const landingPatterns = async (items: readonly MissionItem[]): Promise<readonly LandingPattern[]> => {
  const read = await Promise.all(items.filter((it) => it.kind === KIND_LAND).map(async (item) => landingPattern(item.index, await MapBridge.read(`view.landingPattern(${item.index})`))));
  return read.filter((it): it is LandingPattern => it !== null);
};

export const moveLandingPlace = (index: number, place: number, latitude: number, longitude: number): Promise<boolean> => {
  const property = place === LANDING_PLACE_APPROACH ? 'finalApproachCoordinate' : place === LANDING_PLACE_TOUCHDOWN ? 'landingCoordinate' : null;
  return property === null ? Promise.resolve(false) : setOk(`${PLAN_ITEMS}.${index}.${property}`, settingJson(coordinateJson(latitude, longitude)));
};

export const placeTakeoff = (index: number, latitude: number, longitude: number): Promise<boolean> =>
  setOk(`${PLAN_ITEMS}.${index}.launchCoordinate`, settingJson(coordinateJson(latitude, longitude)));

export const placeLandingIfUnplaced = async (index: number, latitude: number, longitude: number): Promise<boolean> => {
  const view = await MapBridge.read(`view.landingPattern(${index})`);
  if (landingPattern(index, view)?.landing != null) return false;
  if (!(await moveLandingPlace(index, LANDING_PLACE_TOUCHDOWN, latitude, longitude))) return false;
  await invokeOk(`${PLAN_ITEMS}.${index}.setLandingHeadingToTakeoffHeading`);
  return leaveWizardMode(index);
};

export const leaveWizardMode = (index: number): Promise<boolean> => setOk(`${PLAN_ITEMS}.${index}.wizardMode`, settingJson('false'));
