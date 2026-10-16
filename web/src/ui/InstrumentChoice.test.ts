import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import {
  DEFAULT_INSTRUMENTS,
  InstrumentFact,
  InstrumentGroup,
  ValueSize,
  ValueSizeEntries,
  chosenKey,
  defaultInstruments,
  emptyCatalogueText,
  instrumentChoiceNote,
  instrumentGroups,
  instrumentVehicleClass,
  instrumentsPath,
  movedInstrument,
  nextValueSize,
  removedInstrument,
  replacedInstrument,
  selectionId,
  shownFirst,
  showsInstruments,
  valueSizeAt,
  valueSizePillText,
  vehicleOwnGroup,
  withInstrument,
} from './InstrumentChoice';
import { scaledNumber } from './VehicleUi';

const json = (text: string): Json => parseJson(text) ?? {};

describe('InstrumentChoiceTest', () => {
  it('the tiles already shown lead the sheet in their own order and leave their groups', () => {
    const alt = InstrumentFact('alt', 'Altitude', 'a.alt');
    const speed = InstrumentFact('speed', 'Speed', 'a.speed');
    const roll = InstrumentFact('roll', 'Roll', 'v.roll');
    const groups = [InstrumentGroup('vehicle', 'Vehicle', [roll]), InstrumentGroup('air', 'Air', [alt, speed])];

    expect(shownFirst(groups, ['a.speed', 'a.alt']).map((it) => [it.title, it.facts.map((fact) => fact.label)])).toEqual([
      ['Shown', ['Speed', 'Altitude']],
      ['Vehicle', ['Roll']],
    ]);
    expect(shownFirst(groups, []).map((it) => it.title)).toEqual(['Vehicle', 'Air']);
  });

  const served = json(`
    {"available":true,"groups":[
      {"group":"gps","title":"GPS","facts":[
        {"name":"lat","label":"Latitude","selection":"gps/lat"},
        {"name":"hdop","label":"HDOP","selection":"gps/hdop"}]},
      {"group":"wind","title":"Wind","facts":[
        {"name":"speed","label":"Wind Speed","selection":"wind/speed"}]},
      {"group":"empty","title":"Empty","facts":[]}
    ]}`);

  it('a group the vehicle reports nothing for is not offered', () => {
    expect(instrumentGroups(served).map((it) => it.group)).toEqual(['gps', 'wind']);
  });

  it("each battery pack is offered after the vehicle's groups, like the battery0 fact group QGC's value editor lists", () => {
    const withPacks = { ...served, packGroups: [{ group: 'batteries.0', title: 'Battery 1', facts: [{ name: 'voltage', label: 'Voltage', selection: 'batteries.0/voltage' }] }] };
    expect(instrumentGroups(withPacks).map((it) => it.group)).toEqual(['gps', 'wind', 'batteries.0']);
    expect(instrumentGroups(withPacks).at(-1)!.facts.map((it) => it.path)).toEqual(['batteries.0/voltage']);
  });

  it('a fact is asked for by the selection the core spells', () => {
    const gps = instrumentGroups(served)[0]!;
    expect(gps.facts.map((it) => it.path), 'a bare lon resolves against the vehicle group and answers noSuchFact').toEqual(['gps/lat', 'gps/hdop']);
    expect(gps.facts[0]!.label).toBe('Latitude');
  });

  it('a fact the core did not spell a selection for is not offered', () => {
    const unspelled = json('{"available":true,"groups":[{"group":"gps","title":"GPS","facts":[{"name":"lat","label":"Latitude"}]}]}');
    expect(instrumentGroups(unspelled), 'a row that cannot be asked for is a row that silently does nothing when tapped').toEqual([]);
  });

  it('no vehicle offers nothing rather than an empty catalogue', () => {
    expect(instrumentGroups(json('{"available":false,"groups":[]}'))).toEqual([]);
    expect(instrumentGroups(null)).toEqual([]);
  });

  it("choosing nothing draws nothing, and the core's own fallback is never reached", () => {
    expect(instrumentsPath(['lat', 'hdop'])).toBe('view.instruments(lat,hdop)');
    expect(instrumentsPath(DEFAULT_INSTRUMENTS)).toBe('view.instruments');
    expect(instrumentsPath(defaultInstruments('fixedWing'), 'fixedWing')).toBe('view.instruments');
    expect(instrumentsPath(DEFAULT_INSTRUMENTS, 'fixedWing')).toBe(`view.instruments(${DEFAULT_INSTRUMENTS.join(',')})`);
    expect(showsInstruments(['lat'])).toBe(true);
    expect(showsInstruments([])).toBe(false);
  });

  it('choosing toggles with no limit, the row wraps like TelemetryChipsLayer', () => {
    expect(withInstrument(['a'], 'b')).toEqual(['a', 'b']);
    expect(withInstrument(['a', 'b'], 'b')).toEqual(['a']);
    const many = Array.from({ length: 9 }, (_, index) => `f${index + 1}`);
    expect(withInstrument(many, 'extra')).toEqual([...many, 'extra']);
  });

  it('the note says where the operator stands', () => {
    expect(instrumentChoiceNote([])).toContain('no readings');
    expect(instrumentChoiceNote(['a', 'b'])).toBe('2 chosen.');
  });

  it('the readings the screen starts with are in the catalogue and match what is chosen', () => {
    const own = vehicleOwnGroup(
      json(`{"kind":"object","available":true,"groups":[],"vehicleFacts":[
        {"name":"altitudeRelative","label":"Alt (Rel)","selection":"altitudeRelative"},
        {"name":"groundSpeed","label":"Ground Speed","selection":"groundSpeed"},
        {"name":"distanceToHome","label":"Distance to Home","selection":"distanceToHome"},
        {"name":"heading","label":"Heading","selection":"heading"},
        {"name":"climbRate","label":"Climb Rate","selection":"climbRate"},
        {"name":"rangeFinderDist","label":"Range Finder Dist","selection":"rangeFinderDist"}]}`),
    )!;
    expect(own.title).toBe('Vehicle');
    expect(DEFAULT_INSTRUMENTS.every((name) => own.facts.some((it) => it.path === name))).toBe(true);
    expect(own.facts[0]!.path).toBe('altitudeRelative');
    expect(own.facts.at(-1)!.label).toBe('Range Finder Dist');
  });

  it('no vehicle contributes no group rather than an empty one', () => {
    expect(vehicleOwnGroup(json('{"kind":"object","available":false,"vehicleFacts":[]}'))).toBeNull();
    expect(vehicleOwnGroup(json('{"kind":"object","available":true,"vehicleFacts":[]}'))).toBeNull();
    expect(vehicleOwnGroup(null)).toBeNull();
  });

  it('an empty catalogue with a vehicle connected does not tell the operator to connect one', () => {
    expect(emptyCatalogueText(true)).toBe('This vehicle reported no readings this screen can ask for.');
    expect(emptyCatalogueText(false)).toBe('Connect a vehicle to see what it can report.');
  });

  it("each vehicle class keeps its own instruments, like FactValueGrid's settings key", () => {
    expect(instrumentVehicleClass(json('{"vehicleClass":"multiRotor"}'))).toBe('multiRotor');
    expect(instrumentVehicleClass(null)).toBe('generic');
    expect(chosenKey('fixedWing')).toBe('chosen-fixedWing');
  });

  it("the size pill cycles QGC's four value sizes and says which", () => {
    expect(ValueSizeEntries.map((it) => it.label)).toEqual(['Default', 'Small', 'Medium', 'Large']);
    expect(nextValueSize(ValueSize.Default)).toBe(ValueSize.Small);
    expect(nextValueSize(ValueSize.Large)).toBe(ValueSize.Default);
    expect(valueSizeAt(9)).toBe(ValueSize.Default);
    expect(valueSizePillText(ValueSize.Medium)).toBe('Size: Medium');
    expect(ValueSizeEntries.map((it) => it.scale)).toEqual([1, 0.86, 1.25, 1.5]);
  });

  it('a scaled value keeps the telemetry figures and grows its line with it', () => {
    const large = scaledNumber(1.5);
    expect(large.fontSize).toBe(22 * 1.5);
    expect(large.fontVariantNumeric).toBe('tabular-nums');
  });

  it('values move, swap their reading and go away in place', () => {
    const row = ['a', 'b', 'c'];
    expect(movedInstrument(row, 1, -1)).toEqual(['b', 'a', 'c']);
    expect(movedInstrument(row, 2, 1)).toEqual(row);
    expect(replacedInstrument(row, 1, 'x')).toEqual(['a', 'x', 'c']);
    expect(replacedInstrument(row, 0, 'c'), 'a reading already in the row moves rather than doubling').toEqual(['c', 'b']);
    expect(removedInstrument(row, 1)).toEqual(['a', 'c']);
  });

  it('a bare vehicle selection and its served id agree', () => {
    expect(selectionId('altitudeRelative')).toBe('vehicle/altitudeRelative');
    expect(selectionId('gps/lock')).toBe('gps/lock');
  });
});
