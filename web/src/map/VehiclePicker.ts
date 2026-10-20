import { Qgc } from '@/bridge/Qgc';
import { type Json, isJson, isNull, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { call } from '@/core/client';
import { invokeOk, setOk, settingJson } from './BridgeCalls';
import { type RadarReading, radarReading } from './ProximityRadarMap';
import { type TrackPoint, isPlottable } from './VehicleMap';
import type { UploadGate } from './VehicleSync';

const vehicleBridge = { lastRefusal: null as string | null, lastAsked: null as number | null };

export const VehicleBridge = {
  get lastRefusal(): string | null {
    return vehicleBridge.lastRefusal;
  },

  get lastAsked(): number | null {
    return vehicleBridge.lastAsked;
  },

  forget(): void {
    vehicleBridge.lastAsked = null;
  },

  async askFor(id: number): Promise<boolean> {
    try {
      const parsed: unknown = JSON.parse(await call<string>('invoke', 'vehicles.setActive', `[${id}]`));
      if (!isJson(parsed)) throw new Error('the answer is not a JSON object');
      const reason = optText(parsed, 'reason');
      vehicleBridge.lastRefusal = reason.trim() === '' ? null : reason;
      const accepted = optBoolean(parsed, 'ok');
      if (accepted) vehicleBridge.lastAsked = id;
      return accepted;
    } catch (error) {
      vehicleBridge.lastRefusal = `bridge threw: ${error instanceof Error ? error.message : String(error)}`;
      return false;
    }
  },
};

export const VEHICLES_VIEW = 'view.vehicles';

export const VEHICLE_MANAGER = 'vehicles';

type SelectedTarget = readonly [string, number];

const inOrder = <T,>(items: readonly T[], run: (item: T) => Promise<boolean>): Promise<readonly boolean[]> =>
  items.reduce<Promise<readonly boolean[]>>(async (done, item) => {
    const held = await done;
    return [...held, await run(item)];
  }, Promise.resolve([]));

const idAt = async (path: string): Promise<number | null> => {
  try {
    const id = optInt(await Qgc.getFields(path, ['id']), 'id', -1);
    return id >= 0 ? id : null;
  } catch {
    return null;
  }
};

const armedAt = async (path: string): Promise<boolean> => {
  try {
    return optBoolean(await Qgc.getFields(path, ['armed']), 'armed');
  } catch {
    return false;
  }
};

const still = async ([path, id]: SelectedTarget, write: (path: string) => Promise<boolean>): Promise<boolean> => (await idAt(path)) === id && write(path);

const targetsAt = (paths: readonly string[]): Promise<readonly SelectedTarget[]> =>
  paths.reduce<Promise<readonly SelectedTarget[]>>(async (done, path) => {
    const held = await done;
    const id = await idAt(path);
    return id === null ? held : [...held, [path, id] as const];
  }, Promise.resolve([]));

const selectedPaths = async (): Promise<readonly SelectedTarget[]> => {
  const count = await Qgc.get(`${VEHICLE_MANAGER}.selectedVehicles.count`).then((json) => optInt(json, 'value', 0), () => 0);
  return targetsAt(Array.from({ length: Math.max(0, count) }, (_, index) => `${VEHICLE_MANAGER}.selectedVehicles.${index}`));
};

const armedTargets = (targets: readonly SelectedTarget[]): Promise<readonly SelectedTarget[]> =>
  targets.reduce<Promise<readonly SelectedTarget[]>>(async (done, target) => {
    const held = await done;
    return (await armedAt(target[0])) ? [...held, target] : held;
  }, Promise.resolve([]));

export const FleetBridge = {
  setSelected: (id: number, selected: boolean): Promise<boolean> =>
    invokeOk(selected ? `${VEHICLE_MANAGER}.selectVehicle` : `${VEHICLE_MANAGER}.deselectVehicle`, `[${id}]`),

  selectAll: async (choices: VehicleChoices): Promise<boolean> =>
    (await inOrder(choices.choices.filter((it) => !it.selected), (it) => FleetBridge.setSelected(it.id, true))).every((it) => it),

  deselectAll: (): Promise<boolean> => invokeOk(`${VEHICLE_MANAGER}.deselectAllVehicles`),

  async command(action: string, confirmed: ReadonlySet<number>): Promise<boolean> {
    const targets = (await selectedPaths()).filter(([, id]) => confirmed.has(id));
    if (targets.length === 0) return false;
    switch (action) {
      case 'mvArm': return (await inOrder(targets, (it) => still(it, (path) => setOk(`${path}.armed`, settingJson('true'))))).every((it) => it);
      case 'mvDisarm': return (await inOrder(targets, (it) => still(it, (path) => setOk(`${path}.armed`, settingJson('false'))))).every((it) => it);
      case 'mvPause': return (await inOrder(targets, (it) => still(it, (path) => invokeOk(`${path}.pauseVehicle`)))).every((it) => it);
      case 'mvStartMission': {
        const sent = await inOrder(await armedTargets(targets), (it) => still(it, (path) => invokeOk(`${path}.startMission`)));
        return sent.length > 0 && sent.every((it) => it);
      }
      case 'mvRtl': {
        const sent = await inOrder(await armedTargets(targets), (it) => still(it, (path) => invokeOk(`${path}.guidedModeRTL`, '[false]')));
        return sent.length > 0 && sent.every((it) => it);
      }
      default: return false;
    }
  },
};

export const CHOOSER_TITLE = 'Fly which aircraft?';

export type VehicleChoice = {
  readonly id: number;
  readonly name: string;
  readonly state: string;
  readonly link: string;
  readonly contactLost: boolean;
  readonly active: boolean;
  readonly latitude: number;
  readonly longitude: number;
  readonly selected: boolean;
  readonly heading: number;
  readonly home: TrackPoint | null;
  readonly radar: RadarReading | null;
  readonly armed: boolean;
  readonly telemetry: readonly (readonly [string, string])[];
  readonly index: number;
  readonly flightModes: readonly string[];
};

export const VehicleChoice = (fields: {
  id: number;
  name: string;
  state: string;
  link: string;
  contactLost: boolean;
  active: boolean;
  latitude?: number;
  longitude?: number;
  selected?: boolean;
  heading?: number;
  home?: TrackPoint | null;
  radar?: RadarReading | null;
  armed?: boolean;
  telemetry?: readonly (readonly [string, string])[];
  index?: number;
  flightModes?: readonly string[];
}): VehicleChoice => ({
  latitude: NaN,
  longitude: NaN,
  selected: false,
  heading: NaN,
  home: null,
  radar: null,
  armed: false,
  telemetry: [],
  index: -1,
  flightModes: [],
  ...fields,
});

export const vehicleFlightModePath = (choice: VehicleChoice): string => `vehicles.vehicles.${choice.index}.flightMode`;

export const vehicleTelemetryLine = (choice: VehicleChoice): string | null =>
  choice.telemetry.length === 0 ? null : choice.telemetry.map(([label, value]) => `${label} ${value}`).join(' · ');

export type VehicleChoices = {
  readonly ambiguous: boolean;
  readonly choices: readonly VehicleChoice[];
  readonly canSelectAll: boolean;
  readonly canDeselectAll: boolean;
  readonly active: VehicleChoice | null;
  readonly selectedCount: number;
};

export const VehicleChoices = (fields: { ambiguous: boolean; choices: readonly VehicleChoice[]; canSelectAll?: boolean; canDeselectAll?: boolean }): VehicleChoices => ({
  canSelectAll: false,
  canDeselectAll: false,
  ...fields,
  active: fields.choices.find((it) => it.active) ?? null,
  selectedCount: fields.choices.filter((it) => it.selected).length,
});

const vehicleChoiceState = (entry: Json): string =>
  [
    optText(entry, 'flightMode').trim() === '' ? null : optText(entry, 'flightMode'),
    optBoolean(entry, 'flying') ? 'Flying' : optBoolean(entry, 'armed') ? 'Armed' : 'Disarmed',
  ]
    .filter((it) => it !== null)
    .join(' · ');

const vehicleChoiceOf = (entry: Json): VehicleChoice | null => {
  const id = optInt(entry, 'id', -1);
  if (id < 0) return null;
  const name = optText(entry, 'name');
  const coordinate = optJSONObject(entry, 'coordinate');
  const home = optJSONObject(entry, 'home');
  const homePoint = home === null ? null : { latitude: optDouble(home, 'latitude', NaN), longitude: optDouble(home, 'longitude', NaN) };
  return VehicleChoice({
    id,
    name: name.trim() === '' ? `Vehicle ${id}` : name,
    state: vehicleChoiceState(entry),
    link: optText(entry, 'link'),
    contactLost: !isNull(entry, 'contactLost') && optBoolean(entry, 'contactLost'),
    active: optBoolean(entry, 'active'),
    latitude: coordinate === null ? NaN : optDouble(coordinate, 'latitude'),
    longitude: coordinate === null ? NaN : optDouble(coordinate, 'longitude'),
    selected: optBoolean(entry, 'selected'),
    heading: isNull(entry, 'heading') ? NaN : optDouble(entry, 'heading', NaN),
    radar: radarReading(optJSONObject(entry, 'proximity')),
    armed: optBoolean(entry, 'armed'),
    index: optInt(entry, 'index', -1),
    flightModes: jsonStrings(optJSONArray(entry, 'flightModes')).filter((it) => it.trim() !== ''),
    telemetry: jsonObjects(optJSONArray(entry, 'telemetry')).map((row) => [optText(row, 'label'), optText(row, 'value')] as const),
    home: homePoint !== null && isPlottable(homePoint.latitude, homePoint.longitude) ? homePoint : null,
  });
};

export const vehicleChoices = (view: Json | null): VehicleChoices =>
  VehicleChoices({
    canSelectAll: optBoolean(view, 'canSelectAll'),
    canDeselectAll: optBoolean(view, 'canDeselectAll'),
    ambiguous: optBoolean(view, 'ambiguous'),
    choices: (optJSONArray(view, 'vehicles') ?? []).flatMap((entry) => {
      const choice = isJson(entry) ? vehicleChoiceOf(entry) : null;
      return choice === null ? [] : [choice];
    }),
  });

export const linkDistinguishes = (choices: readonly VehicleChoice[]): boolean =>
  new Set(choices.map((it) => it.link).filter((it) => it.trim() !== '')).size > 1;

export const vehicleChoiceLine = (choice: VehicleChoice, distinguishes = true): string => {
  const link = distinguishes && choice.link.trim() !== '' ? choice.link : null;
  return choice.contactLost
    ? ['No contact', link].filter((it) => it !== null).join(' · ')
    : [choice.state.trim() === '' ? null : choice.state, link].filter((it) => it !== null).join(' · ');
};

export const lostVehicles = (choices: VehicleChoices): readonly VehicleChoice[] => choices.choices.filter((it) => it.contactLost && !it.active);

export const lostVehiclesText = (lost: readonly VehicleChoice[]): string | null => {
  if (lost.length === 0) return null;
  if (lost.length === 1) return `${lost[0]?.name ?? ''} is not answering`;
  return `${lost.length} other vehicles are not answering`;
};

export const activeVehicleTitle = (choices: VehicleChoices, subtitle: string): string =>
  !choices.ambiguous ? subtitle : [choices.active?.name ?? null, subtitle.trim() === '' ? null : subtitle].filter((it) => it !== null).join(' · ');

export const uploadHeading = (gate: UploadGate, choices: VehicleChoices): string => {
  const asked = gate.heading.trim() === '' ? 'Upload this plan?' : gate.heading;
  const target = choices.ambiguous ? (choices.active?.name ?? null) : null;
  if (target === null) return asked;
  return asked.endsWith('?') ? `${asked.slice(0, -1)} to ${target}?` : `${asked} to ${target}`;
};

export const handoverNotice = (before: VehicleChoices | null, now: VehicleChoices, asked: number | null): string | null => {
  const left = before?.active ?? null;
  const arrived = now.active;
  if (left === null || arrived === null) return null;
  if (left.id === arrived.id || arrived.id === asked) return null;
  return now.choices.some((it) => it.id === left.id)
    ? `${left.name} stopped answering. Now flying ${arrived.name}.`
    : `${left.name} is gone. Now flying ${arrived.name}.`;
};

export const activeChanged = (before: VehicleChoices | null, now: VehicleChoices): boolean => {
  const was = before?.active?.id ?? null;
  const isNow = now.active?.id ?? null;
  return was !== null && isNow !== null && was !== isNow;
};

export const rememberedChoices = (previous: VehicleChoices | null, now: VehicleChoices): VehicleChoices | null => {
  if (now.choices.length === 0) return null;
  if (now.active !== null) return now;
  return previous;
};
