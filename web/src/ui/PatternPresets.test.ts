import { describe, expect, it } from 'vitest';
import { presetsShownFirst } from './ItemEditor';
import { presetKind, presetNames } from './PatternPresets';

describe('PatternPresetsTest', () => {
  it('presets show for surveys and corridors only', () => {
    expect(presetKind({ presetKind: 'survey' })).toBe('survey');
    expect(presetKind({ presetKind: null })).toBeNull();
    expect(presetKind({})).toBeNull();
    expect(presetNames({ names: ['Mapping 80m', 'Inspection'] })).toEqual(['Mapping 80m', 'Inspection']);
  });

  it('presets come first only when displayPresetsTabFirst is on', () => {
    expect(presetsShownFirst({ value: true })).toBe(true);
    expect(presetsShownFirst({ value: false })).toBe(false);
    expect(presetsShownFirst({ value: null })).toBe(false);
    expect(presetsShownFirst({})).toBe(false);
  });
});
