import { describe, expect, it } from 'vitest';
import { makeFact } from '@/bridge/Qgc';
import { calibrationState } from '@/ui/CalibrationView';
import { SENSORS } from '@/ui/SetupPages';
import { SetupComponent, splitDesktopOnly } from '@/ui/SetupScreen';
import { SettingsGroup } from '@/ui/SettingsScreen';
import { firstReported, pilotChoice, pilotSearchHits, pilotSections, pilotSettings, pilotWorded, sensorChecks, sensorHealthy } from './PilotSettings';

const parameter = (name: string, description = 'Engineer description') =>
  makeFact({
    path: `vehicle.parameterManager.getParameter(-1,${name})`,
    name,
    description,
    units: 'm',
    valueString: '30',
    value: 30,
    enumStrings: [],
    enumIndex: -1,
    isBool: false,
    isString: false,
    readOnly: false,
    minString: '',
    maxString: '',
    minIsDefaultForType: true,
    maxIsDefaultForType: true,
    defaultValueString: '',
  });

const returnHome = pilotSettings(SettingsGroup.Safety).find((it) => it.label === 'Return-to-home altitude')!;

describe('PilotSettingsTest', () => {
  it("a pilot setting uses whichever firmware parameter the aircraft reports, under the pilot's word", () => {
    const px4 = firstReported(returnHome, (name) => (name === 'RTL_RETURN_ALT' ? parameter(name) : null));
    const ardupilot = firstReported(returnHome, (name) => (name === 'RTL_ALT_M' ? parameter(name) : null));
    const versionless = firstReported(returnHome, (name) => (name === 'RTL_ALT' ? parameter(name) : null));
    expect(px4?.name).toBe('RTL_RETURN_ALT');
    expect(ardupilot?.name, 'the 4.7 name, which the core also reads as RTL_ALT on older ArduCopter').toBe('RTL_ALT_M');
    expect(versionless?.name, 'the old name still answers before the firmware version is known').toBe('RTL_ALT');
    expect(px4?.heading).toBe('Return-to-home altitude');
  });

  it('the engineer description moves behind help once the row has a pilot label', () => {
    expect(firstReported(returnHome, (name) => parameter(name))?.detail).toBe('Engineer description');
  });

  it('nothing shows when the aircraft reports none of the parameters', () => {
    expect(firstReported(returnHome, () => null)).toBeNull();
  });

  it('only safety and control carry a pilot list', () => {
    expect(pilotSettings(SettingsGroup.Safety).length).toBeGreaterThan(0);
    expect(pilotSettings(SettingsGroup.Control).length).toBeGreaterThan(0);
    expect(pilotSettings(SettingsGroup.Camera)).toEqual([]);
  });

  it('settings rows read in pilot words and unknown rows keep theirs', () => {
    expect(pilotWorded(parameter('maxGoToLocationDistance')).heading).toBe('Max fly-to distance');
    expect(pilotWorded(parameter('somethingElse')).shortLabel).toBe('');
  });

  it('desktop-only setup pages fold away from the ones that open here', () => {
    const components = ['Airframe', 'Sensors', 'PID Tuning'].map((name, index) => SetupComponent({ index, name, needsAttention: false }));
    const [here, desktop] = splitDesktopOnly(components, (it: SetupComponent) => it.name === SENSORS);
    expect(here.map((it: SetupComponent) => it.name)).toEqual([SENSORS]);
    expect(desktop.map((it: SetupComponent) => it.name)).toEqual(['Airframe', 'PID Tuning']);
  });

  it("safety lists compass, IMU and gyro in DJI's order with their health", () => {
    const view = {
      class: 'Calibration',
      routines: [
        { id: 'gyro', title: 'Gyroscope', status: 'Calibrated' },
        { id: 'levelHorizon', title: 'Level Horizon', status: '' },
        { id: 'compass', title: 'Compass', status: 'Not calibrated' },
        { id: 'accelerometer', title: 'Accelerometer', status: 'Calibrated' },
      ],
    };
    const checks = sensorChecks(calibrationState(view));
    expect(checks.map((it) => it.id)).toEqual(['compass', 'accelerometer', 'gyro']);
    expect(checks.map(sensorHealthy)).toEqual([false, true, true]);
    expect(sensorChecks(null)).toEqual([]);
  });

  it("failsafe choices read in DJI's words", () => {
    expect(pilotChoice('Hold mode')).toBe('Hover');
    expect(pilotChoice('Return mode')).toBe('Return home');
    expect(pilotChoice('Terminate')).toBe('Stop motors');
    expect(pilotChoice('Enabled always RTL')).toBe('Enabled always RTL');
  });

  it("search finds a pilot setting by its words and by either firmware's parameter name", () => {
    expect(pilotSearchHits('return').map((it) => it.label)).toEqual(['Return-to-home altitude']);
    expect(pilotSearchHits('rtl_alt').map((it) => it.label)).toEqual(['Return-to-home altitude']);
    expect(pilotSearchHits('MPC_XY_VEL_MAX').map((it) => it.label)).toEqual(['Max horizontal speed']);
    expect(pilotSearchHits('return').map((it) => it.section)).toEqual(['Safety › Return to home']);
    expect(pilotSearchHits('  ')).toEqual([]);
  });

  it("offline, safety still lists its sections in DJI's order so the pilot knows they exist", () => {
    expect([...pilotSections(pilotSettings(SettingsGroup.Safety)).keys()]).toEqual(['Return to home', 'Flight protection', 'If something goes wrong']);
  });
});
