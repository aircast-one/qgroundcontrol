import { describe, expect, it } from 'vitest';
import { CALIBRATION_BLOCK, LOG_DOWNLOAD_BLOCK, navigationRefusal } from './AppNavigation';

describe('NavigationBlockTest', () => {
  it("a set reason refuses leaving with QGC's words, like MainWindow allowViewSwitch", () => {
    expect(navigationRefusal(LOG_DOWNLOAD_BLOCK, true)).toBe(LOG_DOWNLOAD_BLOCK);
    expect(navigationRefusal(null, true)).toBeNull();
    expect(navigationRefusal(CALIBRATION_BLOCK, false)).toBeNull();
    expect(CALIBRATION_BLOCK).toBe('Complete or cancel the current calibration first');
  });
});
