import { describe, expect, it } from 'vitest';
import { appFontScale } from './AppFontScale';

describe('AppFontScaleTest', () => {
  it('font size scales from the platform default and unset keeps it', () => {
    expect(appFontScale(14)).toBeCloseTo(1, 3);
    expect(appFontScale(21)).toBeCloseTo(1.5, 3);
    expect(appFontScale(0)).toBeCloseTo(1, 3);
    expect(appFontScale(60)).toBeCloseTo(1, 3);
    expect(appFontScale(null)).toBeCloseTo(1, 3);
  });
});
