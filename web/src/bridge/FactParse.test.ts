import { describe, expect, it } from 'vitest';
import { Qgc, refusal } from './Qgc';
import { truthy } from './QgcState';

describe('RefusalTest', () => {
  it('a failed answer says why, and names what refused when it gives no reason', () => {
    expect(refusal({ ok: true })).toBeNull();
    expect(refusal({ ok: false, reason: 'Not armed' })).toBe('Not armed');
    expect(refusal({ ok: false })).toBe('The vehicle refused.');
    expect(refusal(null)).toBe('The vehicle did not answer.');
    expect(refusal({ ok: false }, 'The link')).toBe('The link refused.');
  });
});

describe('FactParseTest', () => {
  it('a json null string field reads as empty, not as the word null', () => {
    const fact = Qgc.fact('settings.x', { name: 'FLTMODE_CH', shortDescription: null, units: null, valueString: '5.000', minString: null, maxString: null, defaultValueString: null });
    expect(fact.defaultValueString).toBe('');
    expect(fact.minString).toBe('');
    expect(fact.maxString).toBe('');
    expect(fact.description).toBe('');
    expect(fact.units).toBe('');
    expect(fact.valueString).toBe('5.000');
  });

  it('a fact with real strings keeps them', () => {
    const fact = Qgc.fact('', { name: 'RTL_ALT', shortDescription: 'Return altitude', units: 'cm', valueString: '1500', minString: '0', maxString: '8000', defaultValueString: '1500' });
    expect(fact.path).toBe('RTL_ALT');
    expect(fact.description).toBe('Return altitude');
    expect(fact.units).toBe('cm');
    expect(fact.defaultValueString).toBe('1500');
  });
});

describe('TruthyTest', () => {
  it('a real boolean is read as itself', () => {
    expect(truthy(true)).toBe(true);
    expect(truthy(false)).toBe(false);
  });

  it('a property declared int but meaning bool still reads true', () => {
    expect(truthy(1)).toBe(true);
    expect(truthy(0)).toBe(false);
  });

  it('a string encoding is accepted either way round', () => {
    expect(truthy('true')).toBe(true);
    expect(truthy('1')).toBe(true);
    expect(truthy('0')).toBe(false);
    expect(truthy('false')).toBe(false);
  });

  it('an absent or unreadable value is not true', () => {
    expect(truthy(null)).toBe(false);
    expect(truthy('banana')).toBe(false);
  });
});
