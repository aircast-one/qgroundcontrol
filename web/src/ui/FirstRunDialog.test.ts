import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { makeFact } from '@/bridge/Qgc';
import { FIRST_RUN_SYSTEMS, firstRun, firstRunSystemIndex, firstRunSystemWrites, firstRunUnitRows } from './FirstRunDialog';

const fact = (name: string) =>
  makeFact({
    path: `settings.unitsSettings.${name}`,
    name,
    description: name,
    units: '',
    valueString: '0',
    value: 0,
    enumStrings: ['A', 'B'],
    enumIndex: 0,
    isBool: false,
    isString: false,
    readOnly: false,
  });

const shown = ['horizontalDistanceUnits', 'verticalDistanceUnits', 'areaUnits', 'speedUnits', 'temperatureUnits', 'weightUnits', 'customUnits'].map(fact);

describe('FirstRunDialogTest', () => {
  it('the prompt shows until the core says it was shown', () => {
    const prompt = firstRun(
      parseJson(
        '{"show":true,"title":"Preferences","vehicleHeading":"Vehicle Preferences","vehicleDescription":"d",' +
          '"vehiclePreferences":[{"name":"preferredFirmwareClass","label":"Preferred Firmware","control":"choice","path":"settings.appSettings.preferredFirmwareClass"}],' +
          '"unitsHeading":"Measurement Units","unitsDescription":"u"}',
      ),
    )!;
    expect(prompt.title).toBe('Preferences');
    expect(prompt.preferences).toHaveLength(1);
    expect(prompt.preferences[0]?.title).toBe('Preferred Firmware');
    expect(firstRun(parseJson('{"show":false}'))).toBeNull();
  });

  it("the five unit rows always show, like InitialSetupPrompt's repeater", () => {
    expect(firstRunUnitRows(shown).map((it) => it.name)).toEqual(['horizontalDistanceUnits', 'verticalDistanceUnits', 'areaUnits', 'speedUnits', 'temperatureUnits']);
  });

  it('imperial writes feet, square feet, feet per second and fahrenheit like changeSystemOfUnits', () => {
    const rows = firstRunUnitRows(shown);
    expect(firstRunSystemWrites(false, rows).map((it) => it[1])).toEqual([0, 0, 0, 0, 1]);
    expect(firstRunSystemWrites(true, rows).map((it) => it[1])).toEqual([1, 1, 1, 1, 0]);
    expect(firstRunSystemWrites(true, rows)[0]?.[0]).toBe('settings.unitsSettings.horizontalDistanceUnits');
  });

  it('hidden units are not written', () => {
    expect(firstRunSystemWrites(true, firstRunUnitRows([fact('speedUnits')])).map((it) => it[0])).toEqual(['settings.unitsSettings.speedUnits']);
  });

  it('metric is chosen only while horizontal distance is in meters', () => {
    expect(firstRunSystemIndex(1.0)).toBe(0);
    expect(firstRunSystemIndex(0.0)).toBe(1);
    expect(firstRunSystemIndex(Number.NaN)).toBe(1);
    expect(FIRST_RUN_SYSTEMS).toEqual(['Metric System', 'Imperial System']);
  });
});
