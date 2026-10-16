import { type Json, jsonObjects, optBoolean, optJSONArray, optText } from '@/bridge/Json';
import { AppCommands } from '@/bridge/BridgeCommands';
import { getSharedPreferences } from '@/kotlin/Platform';

export const INSTRUMENT_GROUPS = 'view.instrumentGroups';

export const DEFAULT_INSTRUMENTS: readonly string[] = ['distanceToHome', 'altitudeRelative', 'groundSpeed', 'climbRate'];

const FORWARD_FLIGHT_CLASSES: ReadonlySet<string> = new Set(['fixedWing', 'vtol', 'airship']);

export const defaultInstruments = (vehicleClass: string): readonly string[] => [...DEFAULT_INSTRUMENTS, ...['airSpeed'].filter(() => FORWARD_FLIGHT_CLASSES.has(vehicleClass))];

export const INSTRUMENTS_VIEW = 'view.instruments';

export type InstrumentFact = { name: string; label: string; path: string };

export const InstrumentFact = (name: string, label: string, path: string): InstrumentFact => ({ name, label, path });

export type InstrumentGroup = { group: string; title: string; facts: readonly InstrumentFact[] };

export const InstrumentGroup = (group: string, title: string, facts: readonly InstrumentFact[]): InstrumentGroup => ({ group, title, facts });

const availableView = (view: Json | null): Json | null => (view !== null && optBoolean(view, 'available') ? view : null);

const instrumentFact = (it: Json): InstrumentFact => InstrumentFact(optText(it, 'name'), optText(it, 'label'), optText(it, 'selection'));

const askable = (fact: InstrumentFact): boolean => fact.name.trim() !== '' && fact.path.trim() !== '';

const listedGroups = (view: Json | null, key: string): readonly InstrumentGroup[] =>
  jsonObjects(optJSONArray(availableView(view), key))
    .map((entry) => InstrumentGroup(optText(entry, 'group'), optText(entry, 'title'), jsonObjects(optJSONArray(entry, 'facts')).map(instrumentFact).filter(askable)))
    .filter((it) => it.facts.length > 0);

export const instrumentGroups = (view: Json | null): readonly InstrumentGroup[] => [...listedGroups(view, 'groups'), ...listedGroups(view, 'packGroups')];

export const vehicleOwnGroup = (view: Json | null): InstrumentGroup | null => {
  const facts = optJSONArray(availableView(view), 'vehicleFacts');
  if (facts === null) return null;
  const listed = jsonObjects(facts).map(instrumentFact).filter(askable);
  return listed.length === 0 ? null : InstrumentGroup('vehicle', 'Vehicle', listed);
};

const sameList = (a: readonly string[], b: readonly string[]): boolean => a.length === b.length && a.every((it, index) => it === b[index]);

const GENERIC_CLASS = 'generic';

export const instrumentsPath = (chosen: readonly string[], vehicleClass: string = GENERIC_CLASS): string =>
  chosen.length === 0 || sameList(chosen, defaultInstruments(vehicleClass)) ? INSTRUMENTS_VIEW : `${INSTRUMENTS_VIEW}(${chosen.join(',')})`;

export const showsInstruments = (chosen: readonly string[]): boolean => chosen.length > 0;

export const withInstrument = (chosen: readonly string[], name: string): readonly string[] => {
  if (!chosen.includes(name)) return [...chosen, name];
  const at = chosen.indexOf(name);
  return [...chosen.slice(0, at), ...chosen.slice(at + 1)];
};

export const selectionId = (selection: string): string => (selection.includes('/') ? selection : `vehicle/${selection}`);

export const movedInstrument = (chosen: readonly string[], index: number, by: number): readonly string[] => {
  const to = index + by;
  const inside = (at: number) => at >= 0 && at < chosen.length;
  return inside(to) && inside(index) ? chosen.map((name, i) => (i === index ? chosen[to]! : i === to ? chosen[index]! : name)) : chosen;
};

export const replacedInstrument = (chosen: readonly string[], index: number, path: string): readonly string[] =>
  chosen.map((name, i) => (i === index ? path : name)).filter((name, i) => i === index || name !== path);

export const removedInstrument = (chosen: readonly string[], index: number): readonly string[] => chosen.filter((_, i) => i !== index);

export const emptyCatalogueText = (connected: boolean): string =>
  connected ? 'This vehicle reported no readings this screen can ask for.' : 'Connect a vehicle to see what it can report.';

export const instrumentChoiceNote = (chosen: readonly string[]): string =>
  chosen.length === 0 ? 'Nothing chosen. The flight screen shows no readings.' : `${chosen.length} chosen.`;

const STORE = 'fly-instruments';

export const instrumentVehicleClass = (view: Json | null): string => {
  const named = optText(view, 'vehicleClass');
  return named.trim() === '' ? GENERIC_CLASS : named;
};

export const chosenKey = (vehicleClass: string): string => `chosen-${vehicleClass}`;

export const readChosen = (vehicleClass: string): readonly string[] => {
  const stored = getSharedPreferences(STORE).getString(chosenKey(vehicleClass), null);
  return stored === null ? defaultInstruments(vehicleClass) : stored.split(',').filter((it) => it.trim() !== '');
};

export const writeChosen = (vehicleClass: string, chosen: readonly string[]): void => {
  getSharedPreferences(STORE).putString(chosenKey(vehicleClass), chosen.join(','));
  void AppCommands.setSubtitleInstruments(vehicleClass, [...chosen]);
};

const VALUE_SIZE_KEY = 'fontSize';

export type ValueSize = { readonly name: 'Default' | 'Small' | 'Medium' | 'Large'; readonly ordinal: number; readonly label: string; readonly scale: number };

export const ValueSize = {
  Default: { name: 'Default', ordinal: 0, label: 'Default', scale: 1 },
  Small: { name: 'Small', ordinal: 1, label: 'Small', scale: 0.86 },
  Medium: { name: 'Medium', ordinal: 2, label: 'Medium', scale: 1.25 },
  Large: { name: 'Large', ordinal: 3, label: 'Large', scale: 1.5 },
} as const satisfies Record<string, ValueSize>;

export const ValueSizeEntries: readonly ValueSize[] = [ValueSize.Default, ValueSize.Small, ValueSize.Medium, ValueSize.Large];

export const valueSizeAt = (ordinal: number): ValueSize => ValueSizeEntries[ordinal] ?? ValueSize.Default;

export const nextValueSize = (size: ValueSize): ValueSize => valueSizeAt((size.ordinal + 1) % ValueSizeEntries.length);

export const valueSizePillText = (size: ValueSize): string => `Size: ${size.label}`;

export const readValueSize = (vehicleClass: string): ValueSize => valueSizeAt(getSharedPreferences(STORE).getInt(`${VALUE_SIZE_KEY}-${vehicleClass}`, 0));

export const writeValueSize = (vehicleClass: string, size: ValueSize): void => {
  getSharedPreferences(STORE).putInt(`${VALUE_SIZE_KEY}-${vehicleClass}`, size.ordinal);
};

const SHOWN_GROUP = 'shown';

export const shownFirst = (groups: readonly InstrumentGroup[], shown: readonly string[]): readonly InstrumentGroup[] => {
  const all = groups.flatMap((it) => it.facts);
  const picked = InstrumentGroup(
    SHOWN_GROUP,
    'Shown',
    shown.flatMap((path) => all.filter((it) => it.path === path).slice(0, 1)),
  );
  return [picked, ...groups.map((group) => ({ ...group, facts: group.facts.filter((it) => !shown.includes(it.path)) }))].filter((it) => it.facts.length > 0);
};
