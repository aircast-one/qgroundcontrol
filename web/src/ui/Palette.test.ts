import { describe, expect, it } from 'vitest';
import { makeFact } from '@/bridge/Qgc';
import { PALETTE_SETTING, paletteIsDark, paletteNamed, shouldDefaultToDark } from './Palette';

describe('PaletteTest', () => {
  it('indoorIsDarkOutdoorIsLightSystemFollowsTheDevice', () => {
    expect(paletteIsDark(1, false)).toBe(true);
    expect(paletteIsDark(0, true)).toBe(false);
    expect(paletteIsDark(2, true)).toBe(true);
    expect(paletteIsDark(2, false)).toBe(false);
  });

  it('darkWhileTheSettingIsUnread', () => {
    expect(paletteIsDark(null, false)).toBe(true);
  });

  it('defaultsToDarkOnceAndOnlyWhenUntouched', () => {
    expect(shouldDefaultToDark(false, false)).toBe(true);
    expect(shouldDefaultToDark(true, false)).toBe(false);
    expect(shouldDefaultToDark(false, true)).toBe(false);
  });

  it('the palette choice is named dark and light by its stored value, as Penpot names it', () => {
    const fact = makeFact({
      path: PALETTE_SETTING, name: 'indoorPalette', description: 'Color Scheme', units: '', valueString: '1', value: 1,
      enumStrings: ['Innen', 'Außen', 'System'], enumValues: ['1', '0', '2'], enumIndex: 0, isBool: false, isString: false, readOnly: false,
    });
    expect(paletteNamed(fact).enumStrings).toEqual(['Dark', 'Light', 'System']);
    expect(paletteNamed(makeFact({ ...fact, path: 'settings.x', enumStrings: ['A', 'B'], enumValues: ['1', '0'] })).enumStrings).toEqual(['A', 'B']);
  });
});
