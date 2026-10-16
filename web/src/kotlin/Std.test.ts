import { describe, expect, it } from 'vitest';
import { coerceIn, format, same } from './Std';
import { optBoolean, optDouble, optInt, optString, optText } from '@/bridge/Json';
import { sentenceCase } from '@/map/SentenceCase';

describe('format follows String.format', () => {
  it('formats the patterns the Android head uses', () => {
    expect(format('%.1f', 26.04)).toBe('26.0');
    expect(format('%d%%', 42)).toBe('42%');
    expect(format('%02d\'%02d"', 12, 7)).toBe('12\'07"');
    expect(format('%5.1f|%-5s|', 3.14159, 'ab')).toBe('  3.1|ab   |');
    expect(format('%,d', 1234567)).toBe('1,234,567');
    expect(format('%+.1f', 2)).toBe('+2.0');
    expect(format('%04X', 255)).toBe('00FF');
    expect(format('%2$s %1$s', 'world', 'hello')).toBe('hello world');
  });

  it('clamps and compares like Kotlin', () => {
    expect(coerceIn(12, 0, 10)).toBe(10);
    expect(same({ a: [1, { b: 2 }] }, { a: [1, { b: 2 }] })).toBe(true);
    expect(same({ a: 1 }, { a: 2 })).toBe(false);
  });
});

describe('Json opt helpers follow org.json', () => {
  const json = { t: true, s: 'TRUE', n: 3.9, ns: '7', nil: null, obj: { a: 1 } };

  it('reads booleans, numbers and strings with the same fallbacks', () => {
    expect(optBoolean(json, 't')).toBe(true);
    expect(optBoolean(json, 's')).toBe(true);
    expect(optBoolean(json, 'missing', true)).toBe(true);
    expect(optInt(json, 'n')).toBe(3);
    expect(optInt(json, 'ns')).toBe(7);
    expect(optInt(json, 'missing', -1)).toBe(-1);
    expect(Number.isNaN(optDouble(json, 'missing'))).toBe(true);
    expect(optString(json, 'nil')).toBe('null');
    expect(optText(json, 'nil')).toBe('');
    expect(optText(json, 'n')).toBe('3.9');
  });
});

describe('sentenceCase', () => {
  it('lowercases ordinary words but keeps proper nouns and acronyms', () => {
    expect(sentenceCase('Flight Mode Settings')).toBe('Flight mode settings');
    expect(sentenceCase('Use ArduPilot RTL Altitude')).toBe('Use ArduPilot RTL altitude');
    expect(sentenceCase('PX4 Pro Flight Stack')).toBe('PX4 Pro flight stack');
    expect(sentenceCase('Auto-Connect To Pixhawk')).toBe('Auto-connect to Pixhawk');
  });
});
