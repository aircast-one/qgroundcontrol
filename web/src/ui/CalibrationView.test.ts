import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { CANCEL_WAIT_TEXT, type CalibrationSide, calibrationState } from '@/ui/CalibrationView';
import { positionsText, routineCopy } from '@/ui/SensorsScreen';

const served: Json = {
  kind: 'object',
  class: 'Calibration',
  connected: true,
  inProgress: false,
  busy: false,
  showsSides: false,
  nextEnabled: false,
  cancelEnabled: false,
  progress: 0.0,
  progressText: '0%',
  helpText: '',
  statusText: '',
  accelNeeded: true,
  compassNeeded: true,
  needsAttention: 'The accelerometer and compass both need calibrating.',
  sides: [{ key: 'Down', title: 'Level', visible: false, stage: 'waiting', rotate: false }],
  routines: [
    {
      id: 'accelerometer',
      title: 'Accelerometer',
      invocation: 'sensorsCal.calibrateAccel',
      arguments: [false],
      blocked: false,
      enabled: true,
      description: 'Hold the vehicle in each orientation it asks for.',
      status: 'Not calibrated',
      warning: '',
    },
    {
      id: 'compass',
      title: 'Compass',
      invocation: 'sensorsCal.calibrateCompass',
      arguments: [],
      blocked: true,
      enabled: false,
      description: 'Calibrate the accelerometer first.',
      status: 'Calibrate the accelerometer first.',
      warning: '',
    },
  ],
};

const read = (json: Json) => {
  const state = calibrationState(json);
  if (state === null) throw new Error('not the calibration view');
  return state;
};

const routine = (json: Json, index: number) => {
  const found = read(json).routines[index];
  if (found === undefined) throw new Error(`no routine ${index}`);
  return found;
};

describe('CalibrationViewTest', () => {
  it('the step line counts the positions reached among the visible ones', () => {
    const side = (key: string, stage: string, visible = true): CalibrationSide => ({ key, title: key, visible, stage, rotate: false });
    const sides = [side('down', 'done'), side('up', 'done'), side('left', 'inProgress'), side('right', 'pending'), side('nose', 'pending', false)];

    expect(positionsText(sides)).toBe('3 of 4 positions');
    expect(positionsText(sides.map((it) => ({ ...it, stage: 'pending' })))).toBe('1 of 4 positions');
    expect(positionsText([])).toBeNull();
  });

  it('the routines come from the core, with the call it says to make', () => {
    const state = read(served);

    expect(state.routines.map((it) => it.id)).toEqual(['accelerometer', 'compass']);
    expect(routine(served, 0).invocation).toBe('sensorsCal.calibrateAccel');
    expect(routine(served, 0).arguments).toEqual([false]);
    expect(routine(served, 1).arguments).toEqual([]);
  });

  it("the accelerometer-first rule and each routine's status are the core's answer, not the head's", () => {
    expect(routine(served, 0).blocked).toBe(false);
    expect(routine(served, 1).blocked).toBe(true);
    expect(routine(served, 1).status).toBe('Calibrate the accelerometer first.');
    expect(routine(served, 0).status).toBe('Not calibrated');
  });

  it('a px4 routine opens with the dialog text qgc shows', () => {
    const px4Json: Json = { ...served, px4: true, settingsTitle: 'Orientations' };
    const px4 = read(px4Json);
    const gyro = { ...routine(px4Json, 0), id: 'gyro', dialogHelp: 'For Gyroscope calibration you will need to place your vehicle on a surface and leave it still.' };

    expect(px4.px4).toBe(true);
    expect(px4.settingsTitle).toBe('Orientations');
    expect(routineCopy(gyro).instruction).toBe(gyro.dialogHelp);
  });

  it("the instruction is the core's dialog text, else its description", () => {
    expect(routineCopy({ ...routine(served, 1), dialogHelp: '', description: 'Rotate the vehicle until every side is done.' }).instruction).toBe('Rotate the vehicle until every side is done.');
  });

  it('sides arrive with their stage rather than three separate flags', () => {
    const running = read({
      class: 'Calibration',
      sides: [
        { key: 'Down', title: 'Level', visible: true, stage: 'done', rotate: false },
        { key: 'Left', title: 'Left side', visible: true, stage: 'inProgress', rotate: true },
      ],
    });

    expect(running.sides.map((it) => it.stage)).toEqual(['done', 'inProgress']);
    expect(running.sides.map((it) => it.rotate)).toEqual([false, true]);
  });

  it('a payload that is not the calibration view yields nothing', () => {
    expect(calibrationState(null)).toBeNull();
    expect(calibrationState({})).toBeNull();
    expect(calibrationState({ class: 'Vehicle' })).toBeNull();
  });

  it('a payload using invented names leaves the state empty rather than plausible', () => {
    const state = read({ class: 'Calibration', calibrations: [{ id: 'accelerometer' }], orientations: [{ key: 'Down' }], inProgressNow: true });

    expect(state.routines).toEqual([]);
    expect(state.sides).toEqual([]);
    expect(state.inProgress).toBe(false);
  });

  it("a pending cancel says so, like SensorsSetup's Calibration Cancel dialog", () => {
    expect(read(served).waitingForCancel).toBe(false);
    expect(read({ ...served, waitingForCancel: true }).waitingForCancel).toBe(true);
    expect(CANCEL_WAIT_TEXT).toBe('Waiting for Vehicle to response to Cancel. This may take a few seconds.');
  });
});
