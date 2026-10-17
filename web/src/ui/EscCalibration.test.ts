import { describe, expect, it } from 'vitest';
import { EscCalibrationState, escCalibration } from '@/ui/EscCalibration';

describe('EscCalibrationTest', () => {
  it("the dialog shows the core's highlighted prefix and text", () => {
    const state = escCalibration({ open: true, highlight: 'ESC Calibration failed. ', text: 'timeout', running: false });
    expect(state).toEqual(EscCalibrationState({ highlight: 'ESC Calibration failed. ', text: 'timeout', running: false }));
    expect(escCalibration({ open: false })).toBeNull();
  });
});
