import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { makeFact } from '@/bridge/Qgc';
import { FIRST_RUN_SYSTEMS, VEHICLE_PREFERENCE, firstRun, firstRunQuestions } from './FirstRunDialog';

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

  it('the welcome asks two questions: the unit system, by the names Settings uses, and the vehicle type', () => {
    expect(FIRST_RUN_SYSTEMS).toEqual(['Metric', 'Imperial']);
    const firmware = fact('preferredFirmwareClass');
    const vehicle = fact(VEHICLE_PREFERENCE);
    expect(firstRunQuestions([firmware, vehicle, ...shown]).map((it) => it.name)).toEqual([VEHICLE_PREFERENCE]);
  });
});
