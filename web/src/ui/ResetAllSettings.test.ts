import { describe, expect, it } from 'vitest';
import { resetPending } from './ResetAllSettings';

describe('ResetAllSettingsTest', () => {
  it('a pending reset reads from the setting value', () => {
    expect(resetPending(true)).toBe(true);
    expect(resetPending(false)).toBe(false);
    expect(resetPending(null)).toBe(false);
  });
});
