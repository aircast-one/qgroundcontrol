import { describe, expect, it, vi } from 'vitest';
import { makeFact } from '@/bridge/Qgc';
import { calibrationState } from '@/ui/CalibrationView';
import { SENSORS } from '@/ui/SetupPages';
import { SetupComponent, splitDesktopOnly } from '@/ui/SetupScreen';
import { SettingsGroup } from '@/ui/SettingsScreen';
import { FENCE_CIRCLE, FENCE_MAX_ALTITUDE, fenceOff, pilotOwner, fenceRestore, fenceTurnOn, batteryPrefix, firstReported, offlineSettings, type ParameterWriter, readingOf, writeAllOrNone, holdWord, pilotChoice, pilotSearchHits, pilotSections, pilotSettings, pilotWorded, sensorChecks, sensorHealthy } from './PilotSettings';

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

const returnHome = pilotSettings(SettingsGroup.Safety).find((it) => it.label === 'Return altitude')!;

describe('PilotSettingsTest', () => {
  it("a pilot setting uses whichever firmware parameter the aircraft reports, under the pilot's word", () => {
    const px4 = firstReported(returnHome, (name) => (name === 'RTL_RETURN_ALT' ? parameter(name) : null));
    const ardupilot = firstReported(returnHome, (name) => (name === 'RTL_ALT_M' ? parameter(name) : null));
    const versionless = firstReported(returnHome, (name) => (name === 'RTL_ALT' ? parameter(name) : null));
    expect(px4?.name).toBe('RTL_RETURN_ALT');
    expect(ardupilot?.name, 'the 4.7 name, which the core also reads as RTL_ALT on older ArduCopter').toBe('RTL_ALT_M');
    expect(versionless?.name, 'the old name still answers before the firmware version is known').toBe('RTL_ALT');
    expect(px4?.heading).toBe('Return altitude');
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
    expect(pilotWorded(parameter('goToLocationRequiresConfirmInGuided')).heading).toBe('Confirm fly-to first');
    expect(pilotWorded(parameter('somethingElse')).shortLabel).toBe('');
  });

  it('the offline map zoom rows are named, not described with a full stop', () => {
    expect([pilotWorded(parameter('minZoomLevelDownload')).heading, pilotWorded(parameter('maxZoomLevelDownload')).heading]).toEqual(['Lowest zoom to download', 'Highest zoom to download']);
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
    expect(pilotChoice('Hold mode', 'Hover')).toBe('Hover');
    expect(pilotChoice('Return mode', 'Hover')).toBe('Return home');
    expect(pilotChoice('Terminate', 'Hover')).toBe('Stop motors');
    expect(pilotChoice('Disabled/NoAction', 'Hover'), 'ArduPilot words its off choice differently').toBe('Do nothing');
    expect(pilotChoice('Enabled always RTL', 'Hover'), "ArduPilot's radio failsafe reads like the others").toBe('Return home');
    expect(pilotChoice('Enabled always SmartRTL or Land', 'Hover')).toBe('Retrace the path home, or land');
    expect(pilotChoice('Enabled always Brake or Land', 'Hover')).toBe('Brake, or land');
    expect(pilotChoice('Something a newer firmware adds', 'Hover'), 'a choice with no plain wording yet keeps its own').toBe('Something a newer firmware adds');
  });

  it('holding in place is named for what the vehicle does: a copter hovers, a plane circles', () => {
    const shape = { fixedWing: false, vtol: false, rover: false, sub: false };
    const plane = holdWord({ ...shape, fixedWing: true });
    expect(holdWord(shape), 'a multirotor or helicopter').toBe('Hover');
    expect(plane).toBe('Circle');
    expect(holdWord({ ...shape, fixedWing: true, vtol: true }), 'a VTOL hovers until it is in forward flight').toBe('Hover, or circle in forward flight');
    expect(holdWord({ ...shape, rover: true })).toBe('Stop');
    expect(holdWord({ ...shape, sub: true })).toBe('Hold position');
    expect(['Hold mode', 'Loiter mode'].map((it) => pilotChoice(it, plane))).toEqual(['Circle', 'Circle']);
  });

  it("search finds a pilot setting by its words and by either firmware's parameter name", () => {
    expect(pilotSearchHits('return').map((it) => it.label)).toEqual(['Return altitude']);
    expect(pilotSearchHits('rtl_alt').map((it) => it.label)).toEqual(['Return altitude']);
    expect(pilotSearchHits('MPC_XY_VEL_MAX').map((it) => it.label)).toEqual(['Max horizontal speed']);
    expect(pilotSearchHits('return').map((it) => it.section)).toEqual(['Safety › Return home']);
    expect(pilotSearchHits('  ')).toEqual([]);
  });

  it("offline, safety still lists its sections in DJI's order so the pilot knows they exist", () => {
    expect([...pilotSections(pilotSettings(SettingsGroup.Safety)).keys()]).toEqual(['Return home', 'Flight protection', 'If something goes wrong']);
  });

  it('both ArduPilot batteries take their failsafe actions from Settings › Safety, so Failsafes links to them alike', () => {
    expect(
      ['BATT_FS_LOW_ACT', 'BATT_FS_CRT_ACT', 'BATT2_FS_LOW_ACT', 'BATT2_FS_CRT_ACT'].map((name) => pilotOwner(name)?.group),
      "battery 2's low and critical actions were edited inline in Failsafes while battery 1's low action lived in Settings › Safety",
    ).toEqual([SettingsGroup.Safety, SettingsGroup.Safety, SettingsGroup.Safety, SettingsGroup.Safety]);
    expect(pilotOwner('COM_LOW_BAT_ACT')?.setting.label, 'PX4 has one action for every battery').toBe('Battery low');
    const offline = offlineSettings(pilotSettings(SettingsGroup.Safety)).map((it) => it.label);
    expect(offline, 'with no aircraft only the rows every vehicle has are listed').toContain('Battery low');
    expect(offline).not.toContain('Battery 2 low');
  });

  it('every battery ArduPilot can report links to Settings › Safety, named as ArduPilot prefixes them', () => {
    expect([0, 1, 8, 9, 15].map(batteryPrefix)).toEqual(['BATT', 'BATT2', 'BATT9', 'BATTA', 'BATTG']);
    expect(
      ['BATT3_FS_LOW_ACT', 'BATT9_FS_CRT_ACT', 'BATTA_FS_LOW_ACT', 'BATTG_FS_CRT_ACT'].map((name) => pilotOwner(name)?.setting.label),
      'batteries 3 and up still edited their actions inline in Failsafes',
    ).toEqual(['Battery 3 low', 'Battery 9 critical', 'Battery 10 low', 'Battery 16 critical']);
    expect(pilotOwner('BATTH_FS_LOW_ACT'), 'ArduPilot stops at sixteen batteries').toBeNull();
    expect(offlineSettings(pilotSettings(SettingsGroup.Safety)).map((it) => it.label)).not.toContain('Battery 3 low');
  });
});

describe('a limit the geofence does not enforce reads as off', () => {
  const reading = (values: Readonly<Record<string, number>>) => (name: string) => values[name] ?? null;

  it('an ArduPilot limit is off while the fence is disabled or its type leaves the limit out', () => {
    expect(fenceOff(FENCE_MAX_ALTITUDE, reading({ FENCE_ENABLE: 0, FENCE_TYPE: 7 }))).toBe(true);
    expect(fenceOff(FENCE_MAX_ALTITUDE, reading({ FENCE_ENABLE: 1, FENCE_TYPE: 6 }))).toBe(true);
    expect(fenceOff(FENCE_CIRCLE, reading({ FENCE_ENABLE: 1, FENCE_TYPE: 3 }))).toBe(false);
  });

  it('a PX4 limit is off while the geofence action is none', () => {
    expect(fenceOff(FENCE_CIRCLE, reading({ GF_ACTION: 0 }))).toBe(true);
    expect(fenceOff(FENCE_CIRCLE, reading({ GF_ACTION: 2 }))).toBe(false);
    expect(fenceOff(FENCE_CIRCLE, reading({}))).toBeNull();
  });

  it('turning a limit on enables the fence for that limit alone, and undo puts both back', () => {
    const before = reading({ FENCE_ENABLE: 0, FENCE_TYPE: 7 });
    const writes = fenceTurnOn(FENCE_MAX_ALTITUDE, before);
    expect(writes, 'an uploaded polygon or the minimum altitude stays off').toEqual([
      ['FENCE_TYPE', FENCE_MAX_ALTITUDE],
      ['FENCE_ENABLE', 1],
    ]);
    expect(fenceRestore(writes, before)).toEqual([
      ['FENCE_ENABLE', 0],
      ['FENCE_TYPE', 7],
    ]);
    expect(fenceTurnOn(FENCE_CIRCLE, reading({ FENCE_ENABLE: 0, FENCE_TYPE: 12 }))).toEqual([
      ['FENCE_TYPE', FENCE_CIRCLE],
      ['FENCE_ENABLE', 1],
    ]);
  });

  it('turning a limit on beside a running fence keeps the limits it already enforces', () => {
    expect(fenceTurnOn(FENCE_CIRCLE, reading({ FENCE_ENABLE: 1, FENCE_TYPE: 5 }))).toEqual([
      ['FENCE_TYPE', 7],
      ['FENCE_ENABLE', 1],
    ]);
    expect(fenceTurnOn(FENCE_CIRCLE, reading({ GF_ACTION: 0 }))).toEqual([['GF_ACTION', 2]]);
  });

  it('a refused write puts back the ones before it, so the fence is never left half changed', async () => {
    const before = reading({ FENCE_ENABLE: 0, FENCE_TYPE: 7 });
    const write = vi.fn<ParameterWriter>().mockResolvedValueOnce(null).mockResolvedValueOnce('Denied').mockResolvedValue(null);
    expect(await writeAllOrNone(fenceTurnOn(FENCE_MAX_ALTITUDE, before), before, write)).toBe('Denied');
    expect(write.mock.calls).toEqual([
      ['FENCE_TYPE', 1],
      ['FENCE_ENABLE', 1],
      ['FENCE_TYPE', 7],
    ]);
  });

  it('a write that goes through leaves nothing to put back', async () => {
    const before = reading({ FENCE_ENABLE: 0, FENCE_TYPE: 7 });
    const write = vi.fn<ParameterWriter>().mockResolvedValue(null);
    const writes = fenceTurnOn(FENCE_MAX_ALTITUDE, before);
    expect(await writeAllOrNone(writes, before, write)).toBeNull();
    expect(write.mock.calls).toEqual(writes);
    expect(readingOf(writes)('FENCE_TYPE')).toBe(1);
    expect(readingOf(writes)('GF_ACTION')).toBeNull();
  });
});

describe('each pilot setting has one home tab', () => {
  it('names the tab that owns a parameter, and nothing for one it does not', () => {
    expect(pilotOwner('FENCE_ALT_MAX')?.group).toBe(SettingsGroup.Safety);
    expect(pilotOwner('FENCE_ALT_MAX')?.setting.label).toBe('Max altitude');
    expect(pilotOwner('LOIT_SPEED_MS')?.group).toBe(SettingsGroup.Control);
    expect(pilotOwner('RTL_LOIT_TIME')).toBeNull();
  });
});
