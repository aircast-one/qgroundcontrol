import { describe, expect, it } from 'vitest';
import { sensorSettings } from './SensorSettings';

describe('SensorSettingsTest', () => {
  it('compasses_and_priority_slots_read_from_the_core', () => {
    expect(sensorSettings({ available: false })).toBeNull();
    const read = sensorSettings({
      available: true,
      boardRotation: null,
      priorities: ['Priority 1', 'Priority 2', 'Priority 3', 'Not Set'],
      compasses: [{ index: 0, label: 'Compass 1 (primary, external)', device: 'IST8310 (I2C1)', use: null, priority: 0, orientation: null, orientationTitle: 'Orientation' }],
      boardTitle: 'Autopilot Rotation',
      compassesWhileCalibrating: true,
      helpSet: 'a',
      helpCal: 'b',
      simpleAccelHelp: 'c',
      declination: null,
    })!;
    expect(read.compasses).toEqual([{ index: 0, label: 'Compass 1 (primary, external)', device: 'IST8310 (I2C1)', use: null, priority: 0, orientation: null, orientationTitle: 'Orientation' }]);
    expect(read.priorities[3]).toBe('Not Set');
  });

  it('px4_mags_carry_their_own_titles_and_stay_out_of_the_calibration_dialog', () => {
    const read = sensorSettings({
      available: true,
      boardRotation: null,
      boardTitle: 'Autopilot Orientation',
      compassesWhileCalibrating: false,
      priorities: [],
      compasses: [{ index: 1, label: 'Mag 1', device: '', use: null, priority: null, orientation: null, orientationTitle: 'Mag 1 Orientation' }],
      helpSet: 'a',
      helpCal: 'b',
      simpleAccelHelp: '',
      declination: null,
    })!;
    expect(read.boardTitle).toBe('Autopilot Orientation');
    expect(read.compasses.map((it) => it.orientationTitle)).toEqual(['Mag 1 Orientation']);
    expect(read.compassesWhileCalibrating).toBe(false);
  });
});
