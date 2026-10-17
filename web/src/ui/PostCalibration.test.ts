import { describe, expect, it } from 'vitest';
import { COMPASS_ROUTINE } from '@/ui/CalibrationView';
import { ACCEL_ROUTINE, CALIBRATION_COMPLETE, postCalibrationPrompt, postCalibrationTitle } from '@/ui/SensorsScreen';

describe('PostCalibrationTest', () => {
  it('only a completed ardupilot accel or compass calibration asks for a reboot', () => {
    expect(postCalibrationPrompt(ACCEL_ROUTINE, CALIBRATION_COMPLETE, ACCEL_ROUTINE, false)).toBe('YOU MUST REBOOT YOUR VEHICLE AFTER EACH CALIBRATION.');
    expect(postCalibrationPrompt(COMPASS_ROUTINE, '', COMPASS_ROUTINE, false)?.startsWith('Shown in the indicator bars')).toBe(true);
    expect(postCalibrationPrompt(COMPASS_ROUTINE, CALIBRATION_COMPLETE, '', false), 'a cancelled or failed run offers nothing').toBeNull();
    expect(postCalibrationPrompt('compassMot', CALIBRATION_COMPLETE, '', false), 'CompassMot completes with the same help text but QGC opens no dialog').toBeNull();
    expect(postCalibrationPrompt(ACCEL_ROUTINE, CALIBRATION_COMPLETE, '', true)).toBeNull();
    expect(postCalibrationPrompt(COMPASS_ROUTINE, CALIBRATION_COMPLETE, '', true)).toBe('Reboot the vehicle prior to flight.');
    expect(postCalibrationTitle(COMPASS_ROUTINE, true)).toBe('Compass calibration complete');
    expect(postCalibrationTitle(COMPASS_ROUTINE, false)).toBe(CALIBRATION_COMPLETE);
  });
});
