import { describe, expect, it } from 'vitest';
import { makeFact } from '@/bridge/Qgc';
import { PRESET_UNIT_FACTS, UNIT_SYSTEM_CUSTOM, unitFacts, unitRowsFor, unitSystemLabel, unitSystemNote } from './UnitsSection';

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

const all = [...PRESET_UNIT_FACTS, 'weightUnits', 'customUnits'].map(fact);

describe('UnitsSectionTest', () => {
  it('a preset hides the measurements it already decides', () => {
    expect(unitRowsFor(0, all).map((it) => it.name)).toEqual([]);
  });

  it('custom shows every measurement', () => {
    expect(unitRowsFor(UNIT_SYSTEM_CUSTOM, all).map((it) => it.name)).toEqual(PRESET_UNIT_FACTS);
  });

  it('the custom-units flag is never a row of its own', () => {
    expect([0, 1, UNIT_SYSTEM_CUSTOM].every((system) => unitRowsFor(system, all).every((it) => it.name !== 'customUnits'))).toBe(true);
  });

  it('weight is never a row, as GeneralSettings lists only five units and weight follows the locale', () => {
    expect([0, 1, UNIT_SYSTEM_CUSTOM].every((system) => unitRowsFor(system, all).every((it) => it.name !== 'weightUnits'))).toBe(true);
  });

  it('the note names the system in force', () => {
    expect(unitSystemNote(0)).toContain('Metric');
    expect(unitSystemNote(1)).toContain('Imperial');
    expect(unitSystemNote(UNIT_SYSTEM_CUSTOM)).toBe('Each measurement is set on its own below.');
  });

  it('a system index the bridge should never send is treated as custom', () => {
    [-1, 3, 99].map((rogue) => {
      expect(unitSystemLabel(rogue)).toBe('Custom');
      expect(unitSystemNote(rogue)).toBe('Each measurement is set on its own below.');
      expect(unitRowsFor(rogue, all).map((it) => it.name)).toEqual(PRESET_UNIT_FACTS);
    });
  });

  it('unit rows come from the units group of the general settings page', () => {
    const page = {
      sections: [
        { group: 'appSettings', subsections: [{ title: '', controls: [{ name: 'audioMuted', label: 'Mute', control: 'toggle' }] }] },
        {
          group: 'unitsSettings',
          subsections: [
            { title: '', controls: [{ name: 'horizontalDistanceUnits', label: 'Distance', control: 'choice', options: [{ label: 'Feet', raw: '0' }, { label: 'Meters', raw: '1' }], display: 'Meters' }] },
            { title: 'Other', controls: [{ name: 'temperatureUnits', label: 'Temperature', control: 'choice', options: [], display: '' }] },
          ],
        },
      ],
    };
    const facts = unitFacts(page);
    expect(facts.map((it) => it.name)).toEqual(['horizontalDistanceUnits', 'temperatureUnits']);
    expect(facts[0]?.enumIndex).toBe(1);
    expect(unitFacts(null).map((it) => it.name)).toEqual([]);
  });
});
