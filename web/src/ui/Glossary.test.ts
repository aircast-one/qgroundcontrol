import { describe, expect, it } from 'vitest';

const GLOSSARY: readonly (readonly [RegExp, string])[] = [
  [/\bAdd link\b/, 'Add a link'],
  [/multi-vehicle panel/i, 'multi-aircraft panel'],
  [/\bMultiple vehicles\b/, 'Multiple aircraft'],
  [/\bconnected to multiple vehicles\b/, 'more than one aircraft is connected'],
  [/\b(PX4 MultiRotor|ArduCopter|ArduPlane|ArduSub|ArduRover|Generic) MockLink\b/, 'Simulated vehicle (…)'],
  [/\bCentre\b|\b(off|map|its|the|circle's) centre\b/, 'center'],
  [/\bthis phone\b/i, 'this device'],
];

const WEB: Readonly<Record<string, string>> = import.meta.glob(['../**/*.{ts,tsx}', '!../**/*.test.{ts,tsx}', '!../core/pkg/**'], { query: '?raw', import: 'default', eager: true });
const CORE: Readonly<Record<string, string>> = import.meta.glob('../../../groundstation/src/**/*.rs', { query: '?raw', import: 'default', eager: true });

const shipped = (path: string, text: string): string => (path.endsWith('.rs') ? (text.split('#[cfg(test)]')[0] ?? text) : text);

const breaches = (files: Readonly<Record<string, string>>): readonly string[] =>
  Object.entries(files).flatMap(([path, text]) =>
    shipped(path, text)
      .split('\n')
      .flatMap((line, at) => GLOSSARY.filter(([banned]) => banned.test(line)).map(([, preferred]) => `${path}:${at + 1} says "${line.trim()}", the glossary says ${preferred}`)),
  );

describe('Glossary', () => {
  it('the web head words things one way', () => {
    expect(Object.keys(WEB).length).toBeGreaterThan(100);
    expect(breaches(WEB)).toEqual([]);
  });

  it('the core, which every head shows, words things the same way', () => {
    expect(Object.keys(CORE).length).toBeGreaterThan(100);
    expect(breaches(CORE)).toEqual([]);
  });
});
