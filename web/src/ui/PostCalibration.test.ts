import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { CALIBRATION_BLOCK } from '@/ui/AppNavigation';
import { COMPASS_ROUTINE, type CalibrationState, calibrationState } from '@/ui/CalibrationView';
import { ACCEL_ROUTINE, CALIBRATION_COMPLETE, UNCANCELLABLE_BLOCK, calibrationLeaveBlock, postCalibrationPrompt, postCalibrationTitle } from '@/ui/SensorsScreen';

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

describe('CalibrationLeaveBlockTest', () => {
  const running = (fields: Json): CalibrationState | null => calibrationState({ class: 'Calibration', inProgress: true, ...fields });

  it('a calibration the autopilot cannot cancel keeps the pilot on the page instead of being silently left running', () => {
    expect(calibrationLeaveBlock(running({ cancelEnabled: false, px4: false }))).toBe(UNCANCELLABLE_BLOCK);
    expect(calibrationLeaveBlock(running({ cancelEnabled: false, px4: true }))).toBe(UNCANCELLABLE_BLOCK);
    expect(calibrationLeaveBlock(running({ cancelEnabled: true, px4: true }))).toBe(CALIBRATION_BLOCK);
    expect(calibrationLeaveBlock(running({ cancelEnabled: true, px4: false })), 'an ArduPilot compass run is cancelled on the way out').toBeNull();
    expect(calibrationLeaveBlock(calibrationState({ class: 'Calibration', inProgress: false, cancelEnabled: false }))).toBeNull();
    expect(calibrationLeaveBlock(null)).toBeNull();
  });
});
