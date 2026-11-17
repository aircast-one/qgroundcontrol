import { describe, expect, it } from 'vitest';
import { megabytesText, quantityText, wholeIfWhole, withUnit } from './Quantity';

describe('withUnit', () => {
  it('puts a space between a value and its unit, except for degrees and percent', () => {
    expect(withUnit('0.0', 'm')).toBe('0.0 m');
    expect(withUnit('5.0', 'm/s')).toBe('5.0 m/s');
    expect(withUnit('90', '°')).toBe('90°');
    expect(withUnit('45', '%')).toBe('45%');
    expect(withUnit('12', '')).toBe('12');
    expect(withUnit('', 'm')).toBe('');
  });
});

describe('quantityText', () => {
  it('shows a configured whole number without a trailing .0, so 50 m reads the same in Settings and Plan', () => {
    expect(quantityText('50.0', 'm')).toBe('50 m');
    expect(quantityText('15.5', 'm')).toBe('15.5 m');
    expect(quantityText('-3.00', 'm')).toBe('-3 m');
    expect(quantityText('Disabled', '')).toBe('Disabled');
  });

  it('shows large storage sizes in gigabytes', () => {
    expect(quantityText('10240', 'MB')).toBe('10 GB');
    expect(quantityText('2048', 'MB')).toBe('2 GB');
    expect(quantityText('1536', 'MB')).toBe('1.5 GB');
    expect(quantityText('128', 'MB')).toBe('128 MB');
  });
});

describe('wholeIfWhole', () => {
  it('drops only an all-zero fraction', () => {
    expect(wholeIfWhole('50.0')).toBe('50');
    expect(wholeIfWhole('5.50')).toBe('5.50');
    expect(wholeIfWhole('abc')).toBe('abc');
  });
});

describe('megabytesText', () => {
  it('climbs to gigabytes from 1024 MB, keeping one decimal only when it says something', () => {
    expect(megabytesText(1023)).toBe('1023 MB');
    expect(megabytesText(1024)).toBe('1 GB');
    expect(megabytesText(262_144)).toBe('256 GB');
  });
});
