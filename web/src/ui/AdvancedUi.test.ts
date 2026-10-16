import { describe, expect, it } from 'vitest';
import { advancedUiShown } from './AdvancedUi';

describe('AdvancedUiTest', () => {
  it('advanced mode is on until the core says otherwise like QGCCorePlugin', () => {
    expect(advancedUiShown(null)).toBe(true);
    expect(advancedUiShown({ shown: false })).toBe(false);
  });
});
