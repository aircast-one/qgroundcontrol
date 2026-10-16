import { call as coreCall, onCoreEvent } from '@/core/client';
import { sentenceCase } from '@/map/SentenceCase';
import { type Json, parseJson, optText, optBoolean, optInt, optJSONArray, opt, jsonStrings } from './Json';

export type FactSlider = { from: number; to: number; decimals: number; hint: string };

export type FactFields = {
  path: string;
  name: string;
  description: string;
  units: string;
  valueString: string;
  value: unknown;
  enumStrings: string[];
  enumValues?: string[];
  enumIndex: number;
  unknownEnumLabel?: string;
  controlKind?: string;
  bitmaskStrings?: string[];
  bitmaskValues?: number[];
  isBool: boolean;
  isString: boolean;
  wholeNumbersOnly?: boolean;
  readOnly: boolean;
  enabled?: boolean;
  disabledReason?: string;
  minString?: string;
  maxString?: string;
  minIsDefaultForType?: boolean;
  maxIsDefaultForType?: boolean;
  defaultValueString?: string;
  vehicleRebootRequired?: boolean;
  qgcRebootRequired?: boolean;
  warning?: boolean;
  changedFromDefault?: boolean;
  optional?: boolean;
  longDescription?: string;
  category?: string;
  group?: string;
  slider?: FactSlider | null;
  firstEntryIsAll?: boolean;
  indent?: boolean;
  smallFont?: boolean;
  shortLabel?: string;
  keywords?: string;
  inverted?: boolean;
  rawChoice?: boolean;
  valueDetails?: string;
  problem?: string;
  enumGroups?: string[];
};

export type Fact = Required<FactFields> & {
  title: string;
  heading: string;
  detail: string;
  isEnum: boolean;
  isBitmask: boolean;
  valueIsOffTheEnumList: boolean;
  boolValue: boolean;
  acceptsWrite: boolean;
  optionalSet: boolean;
};

const trimEndChars = (text: string, chars: string) => text.replace(new RegExp(`[${chars.replace(/[\]\\^-]/g, '\\$&')}]+$`), '');

export const labelCase = (note: string): string => (!note.trimEnd().endsWith('.') && note.split(' ').length <= 4 ? sentenceCase(note) : note);

const restates = (note: string, label: string): boolean => {
  const it = trimEndChars(note, '.').toLowerCase();
  const lower = label.toLowerCase();
  return it.length > 0 && label.length > 0 && (lower.startsWith(it) || it.startsWith(lower) || it.endsWith(` ${lower}`) || lower.endsWith(` ${it}`));
};

export const makeFact = (fields: FactFields): Fact => {
  const filled: Required<FactFields> = {
    enumValues: [],
    unknownEnumLabel: '',
    controlKind: '',
    bitmaskStrings: [],
    bitmaskValues: [],
    wholeNumbersOnly: false,
    enabled: true,
    disabledReason: '',
    minString: '',
    maxString: '',
    minIsDefaultForType: true,
    maxIsDefaultForType: true,
    defaultValueString: '',
    vehicleRebootRequired: false,
    qgcRebootRequired: false,
    warning: false,
    changedFromDefault: false,
    optional: false,
    longDescription: '',
    category: '',
    group: '',
    slider: null,
    firstEntryIsAll: false,
    indent: false,
    smallFont: false,
    shortLabel: '',
    keywords: '',
    inverted: false,
    rawChoice: false,
    valueDetails: '',
    problem: '',
    enumGroups: [],
    ...fields,
  };
  const title = trimEndChars(filled.description || filled.name, ': ');
  const heading = trimEndChars(filled.shortLabel, ': ') || title;
  const detail = filled.shortLabel.trim() !== '' && !filled.enumStrings.includes(filled.description) && !restates(filled.description, heading) ? labelCase(filled.description) : '';
  return {
    ...filled,
    title,
    heading,
    detail,
    isEnum: filled.enumStrings.length > 0 && filled.bitmaskStrings.length === 0,
    isBitmask: filled.bitmaskStrings.length > 0 && filled.bitmaskStrings.length === filled.bitmaskValues.length,
    valueIsOffTheEnumList: filled.unknownEnumLabel.trim() !== '' && filled.enumStrings[filled.enumIndex] === filled.unknownEnumLabel,
    boolValue: (filled.value === true || filled.valueString.toLowerCase() === 'true' || filled.valueString === '1') !== filled.inverted,
    acceptsWrite: !filled.readOnly && filled.enabled,
    optionalSet: typeof filled.value === 'number' && !Number.isNaN(filled.value),
  };
};

const VEHICLE_SUBJECT = 'The vehicle';
const CLIENT = 'app';

export const refusal = (answer: Json | null, subject: string = VEHICLE_SUBJECT): string | null => {
  if (answer === null) return `${subject} did not answer.`;
  if (optBoolean(answer, 'ok')) return null;
  return optText(answer, 'reason').trim() || `${subject} refused.`;
};

type Listener = () => void;

const latest = new Map<string, Json | null>();
const listeners = new Map<string, Set<Listener>>();
const registry = { watched: new Map<string, number>(), started: false };

const publish = (path: string, json: Json | null) => {
  latest.set(path, json);
  listeners.get(path)?.forEach((listener) => listener());
};

const retained = (counts: Map<string, number>, paths: string[]) =>
  paths.reduce((acc, path) => new Map(acc).set(path, (acc.get(path) ?? 0) + 1), counts);

const released = (counts: Map<string, number>, paths: string[]) =>
  paths.reduce((acc, path) => {
    const held = acc.get(path);
    if (held === undefined) return acc;
    const next = new Map(acc);
    if (held === 1) next.delete(path);
    else next.set(path, held - 1);
    return next;
  }, counts);

const sameKeys = (a: Map<string, number>, b: Map<string, number>) => a.size === b.size && [...a.keys()].every((key) => b.has(key));

const resend = (): boolean => {
  try {
    Qgc.sendWatch([...registry.watched.keys()].join(','));
    return true;
  } catch {
    return false;
  }
};

const parsedReply = (text: unknown): Json | null => (typeof text === 'string' ? parseJson(text) : null);

const write = async (path: string, body: Json): Promise<Json | null> => parsedReply(await coreCall('set', path, JSON.stringify(body)));

export const Qgc = {
  start(): void {
    if (registry.started) return;
    registry.started = true;
    onCoreEvent((path, json) => publish(path, parseJson(json) ?? {}));
  },

  path(path: string): Json | null {
    return latest.get(path) ?? null;
  },

  subscribe(path: string, listener: Listener): () => void {
    listeners.set(path, new Set([...(listeners.get(path) ?? []), listener]));
    return () => listeners.get(path)?.delete(listener);
  },

  publishForTest(path: string, json: Json | null): void {
    publish(path, json);
  },

  watchedPathsForTest(): Set<string> {
    return new Set(registry.watched.keys());
  },

  forgetWatchesForTest(): void {
    registry.watched = new Map();
  },

  sendWatch: (csv: string): void => {
    void coreCall('watchClient', CLIENT, csv);
  },

  watch(paths: string[]): void {
    const next = retained(registry.watched, paths);
    if (sameKeys(next, registry.watched)) {
      registry.watched = next;
      return;
    }
    const previous = registry.watched;
    registry.watched = next;
    if (!resend()) {
      registry.watched = previous;
      console.warn(`watch failed for ${paths.join(',')}, will retry`);
    }
  },

  unwatch(paths: string[]): void {
    const next = released(registry.watched, paths);
    const dropped = [...registry.watched.keys()].filter((key) => !next.has(key));
    if (dropped.length === 0) {
      registry.watched = next;
      return;
    }
    const previous = registry.watched;
    registry.watched = next;
    if (resend()) {
      dropped.forEach((path) => publish(path, null));
    } else {
      registry.watched = previous;
      console.warn(`unwatch failed for ${paths.join(',')}, still watching them`);
    }
  },

  async get(path: string): Promise<Json> {
    return parsedReply(await coreCall('get', path)) ?? {};
  },

  async getFields(path: string, fields: string[]): Promise<Json> {
    return parsedReply(await coreCall('getFields', path, fields.join(','))) ?? {};
  },

  async set(path: string, value: unknown): Promise<boolean> {
    const reply = await write(path, { value });
    if (optBoolean(reply, 'ok')) return true;
    console.warn(`set ${path} failed: ${optText(reply, 'reason') || 'the bridge rejected the write'}`);
    return false;
  },

  async writeRefusal(path: string, value: unknown): Promise<string | null> {
    return refusal(await write(path, { value }));
  },

  async writeForVehicleRefusal(path: string, value: unknown, vehicle: number): Promise<string | null> {
    return refusal(await write(path, { value, vehicle }));
  },

  async writeForcedRefusal(path: string, value: unknown): Promise<string | null> {
    return refusal(await write(path, { value, force: true }));
  },

  async call(path: string, ...args: unknown[]): Promise<Json | null> {
    return parsedReply(await coreCall('invoke', path, JSON.stringify(args)));
  },

  async invoke(path: string, ...args: unknown[]): Promise<boolean> {
    const ok = optBoolean(await Qgc.call(path, ...args), 'ok');
    if (!ok) console.warn(`invoke ${path} failed`);
    return ok;
  },

  async invokeResult(path: string, ...args: unknown[]): Promise<unknown> {
    return opt(await Qgc.call(path, ...args), 'result');
  },

  async refusalOf(path: string, ...args: unknown[]): Promise<string | null> {
    return Qgc.refusalOfSubject(VEHICLE_SUBJECT, path, ...args);
  },

  async refusalOfSubject(subject: string, path: string, ...args: unknown[]): Promise<string | null> {
    const reason = refusal(await Qgc.call(path, ...args), subject);
    if (reason) console.warn(`${path} refused: ${reason}`);
    return reason;
  },

  facts(groupPath: string, json: Json | null): Fact[] {
    return (optJSONArray(json, 'facts') ?? []).flatMap((entry) => (typeof entry === 'object' && entry !== null && !Array.isArray(entry) ? [Qgc.fact(groupPath, entry as Json)] : []));
  },

  factAt(path: string, json: Json): Fact {
    return makeFact({ ...Qgc.fact('', json), path });
  },

  fact(groupPath: string, json: Json): Fact {
    const name = optText(json, 'name');
    return makeFact({
      path: groupPath === '' ? name : `${groupPath}.${name}`,
      name,
      description: optText(json, 'shortDescription'),
      longDescription: optText(json, 'longDescription'),
      category: optText(json, 'category'),
      group: optText(json, 'group'),
      units: optText(json, 'units'),
      valueString: optText(json, 'valueString'),
      value: opt(json, 'value'),
      enumStrings: jsonStrings(optJSONArray(json, 'enumStrings')),
      enumValues: jsonStrings(optJSONArray(json, 'enumValues')),
      enumIndex: optInt(json, 'enumIndex', -1),
      unknownEnumLabel: optText(json, 'unknownEnumLabel'),
      bitmaskStrings: jsonStrings(optJSONArray(json, 'bitmaskStrings')),
      bitmaskValues: (optJSONArray(json, 'bitmaskValues') ?? []).map((bit) => (typeof bit === 'number' ? Math.trunc(bit) : Number(bit) || 0)),
      isBool: optBoolean(json, 'typeIsBool'),
      wholeNumbersOnly: optBoolean(json, 'typeIsInteger'),
      isString: optBoolean(json, 'typeIsString'),
      readOnly: optBoolean(json, 'readOnly'),
      minString: optText(json, 'minString'),
      maxString: optText(json, 'maxString'),
      minIsDefaultForType: optBoolean(json, 'minIsDefaultForType', true),
      maxIsDefaultForType: optBoolean(json, 'maxIsDefaultForType', true),
      defaultValueString: optText(json, 'defaultValueString'),
      vehicleRebootRequired: optBoolean(json, 'vehicleRebootRequired'),
      qgcRebootRequired: optBoolean(json, 'qgcRebootRequired'),
      changedFromDefault: optBoolean(json, 'defaultValueAvailable') && !optBoolean(json, 'valueEqualsDefault', true),
    });
  },
};

