import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { makeFact } from '@/bridge/Qgc';
import type { SettingsSectionRows } from './SettingsScreen';
import { aimFraction, grabFraction, swiped, gimbalAsksForControl, gimbalCellText, gimbalIndicator, gimbalRefusal, gimbalSettingsBlocks, joystickButtonsAvailable, onScreenGimbal, screenFraction, nextVideoGimbalMode, serialAsks, VIDEO_GIMBAL_MODES, videoGimbalMode } from './GimbalIndicator';

describe('GimbalIndicatorTest', () => {
  it('the cell reads status, pitch and yaw as the toolbar does', () => {
    const state = gimbalIndicator(
      parseJson(
        '{"shown":true,"statusText":"Yaw follow","pitchText":"P: -12.3","yawText":"Y: 5.0","yawLockLabel":"Yaw Lock",' +
          '"retractOffered":true,"controlOffered":false,"controlLabel":"Acquire Control","gimbals":[{"name":"Gimbal 1","managerCompid":1,"deviceId":154,"active":true}]}',
      ),
    )!;
    expect(gimbalCellText(state)).toBe('Yaw follow · P: -12.3 · Y: 5.0');
    const two = { ...state, gimbals: [...state.gimbals, { name: '1-155', managerCompid: 1, deviceId: 155, active: false }] };
    expect(gimbalCellText(two), 'with several gimbals the toolbar names the active one').toBe('Gimbal 1 · Yaw follow · P: -12.3 · Y: 5.0');
    expect(state.retractOffered).toBe(true);
    expect(state.gimbals[0]!.deviceId).toBe(154);
    expect(gimbalIndicator(parseJson('{"shown":false}'))).toBeNull();
  });

  it("a tap maps to QGC's cooked screen fractions", () => {
    const centre = screenFraction(50, 50, 100, 100);
    expect(centre[0]).toBeCloseTo(0);
    expect(centre[1]).toBeCloseTo(0);
    const corner = screenFraction(100, 0, 100, 100);
    expect(corner[0]).toBe(1);
    expect(corner[1]).toBe(1);
    expect(screenFraction(0, 100, 100, 100)[1]).toBe(-1);
    expect(onScreenGimbal(parseJson('{"shown":true,"onScreen":{"enabled":true,"clickAndDrag":true}}'))).toEqual({ enabled: true, clickAndDrag: true });
    expect(onScreenGimbal(parseJson('{"shown":true,"onScreen":{"enabled":false,"clickAndDrag":true}}')), 'with on-screen control off the video still aims by drag, as CameraAimArea').toEqual({ enabled: false, clickAndDrag: true });
  });
});

const fact = (name: string, value: boolean | number) =>
  makeFact({
    path: `settings.gimbalControllerSettings.${name}`,
    name,
    description: name,
    units: '',
    valueString: value.toString(),
    value,
    enumStrings: [],
    enumIndex: -1,
    isBool: typeof value === 'boolean',
    isString: false,
    readOnly: false,
  });

const flyView: readonly SettingsSectionRows[] = [
  { title: 'Fly View', group: 'flyViewSettings', note: '', blocks: [{ title: '', facts: [fact('showGimbalOnScreenControl', true)] }] },
  {
    title: 'Gimbal Controller',
    group: 'gimbalControllerSettings',
    note: '',
    blocks: [
      { title: 'On-Screen Control', facts: [fact('enableOnScreenControl', true), fact('clickAndDrag', false)] },
      { title: 'Zoom speed', facts: [fact('zoomMaxSpeed', 100), fact('zoomMinSpeed', 1)] },
      { title: '', facts: [fact('joystickButtonsSpeed', 30), fact('showAzimuthIndicatorOnMap', false)] },
    ],
  },
];

describe('GimbalSettingsTest', () => {
  it("the sheet shows the gimbal section's headed blocks like GimbalIndicator's expanded page", () => {
    expect(gimbalSettingsBlocks(flyView, true).map((block) => block.title)).toEqual(['On-Screen Control', 'Zoom speed', '']);
    expect(gimbalSettingsBlocks(flyView, true)[0]!.facts.map((it) => it.name)).toEqual(['enableOnScreenControl', 'clickAndDrag']);
  });

  it('joystick buttons speed is read-only without a joystick enabled for the vehicle', () => {
    const speed = (buttons: boolean) =>
      gimbalSettingsBlocks(flyView, buttons)
        .flatMap((block) => block.facts)
        .find((it) => it.name === 'joystickButtonsSpeed')!;
    expect(speed(false).enabled).toBe(false);
    expect(speed(true).enabled).toBe(true);
    expect(joystickButtonsAvailable(parseJson('{"active":null,"vehicle":true,"enabled":true}'))).toBe(false);
    expect(joystickButtonsAvailable(parseJson('{"active":"Pad","vehicle":true,"enabled":true}'))).toBe(true);
  });
});

describe('GimbalTakeControlTest', () => {
  it('another holder of the gimbal asks to take control instead of failing quietly', () => {
    gimbalAsksForControl.value = false;
    expect(gimbalRefusal(parseJson('{"ok":false,"refusal":"othersHaveControl","reason":"Command not sent. Another user has control of the gimbal."}'))).toBeNull();
    expect(gimbalAsksForControl.value).toBe(true);
    expect(gimbalRefusal(parseJson('{"ok":false,"refusal":"notReady","reason":"The gimbal is not ready yet."}'))).toBe('The gimbal is not ready yet.');
    gimbalAsksForControl.value = false;
  });

  it('a joystick refusal raised in the core asks once the serial moves, never on the first read', () => {
    expect(serialAsks(null, 3)).toBe(false);
    expect(serialAsks(3, 3)).toBe(false);
    expect(serialAsks(3, 4)).toBe(true);
  });

  it("a drag aims by its move over half the view width, as CameraAimArea's sensitivity", () => {
    expect(aimFraction(100, 400)).toBe(0.5);
    expect(aimFraction(-50, 400)).toBe(-0.25);
  });

  it('each way a drag on the video can steer the gimbal maps to its two settings and back', () => {
    expect(VIDEO_GIMBAL_MODES.map((mode) => videoGimbalMode({ enabled: mode.onScreen, clickAndDrag: mode.clickAndDrag }))).toEqual(VIDEO_GIMBAL_MODES.map((mode) => mode.mode));
    expect(videoGimbalMode({ enabled: false, clickAndDrag: true })).toBe('drag');
    expect(VIDEO_GIMBAL_MODES.map((mode) => mode.label)).toEqual(['Drag to move', 'Tap to point', 'Hold and drag']);
  });

  it('the switch on the video steps through every mode and back to the first', () => {
    expect(VIDEO_GIMBAL_MODES.map((mode) => nextVideoGimbalMode(mode.mode))).toEqual(['point', 'hold', 'drag']);
  });

  it('dragging the picture turns the camera the other way, so what you grabbed stays under your finger', () => {
    const [pan, tilt] = grabFraction(50, 25, 200, 100);
    expect(pan, 'the picture dragged right means the camera turns left').toBe(-0.5);
    expect(tilt, 'the picture dragged down means the camera tilts up').toBe(0.5);
  });

  it('a two-finger swipe scrolls the picture like a page and its steps add up to one drag', () => {
    const first = swiped(null, 20, 0, 1000, 200, 100);
    expect(first.begin).toBe(true);
    expect(grabFraction(first.swipe.x, first.swipe.y, 200, 100)[0], 'scrolling right turns the camera right').toBeCloseTo(0.2);
    const next = swiped(first.swipe, 20, 10, 1100, 200, 100);
    expect(next.begin).toBe(false);
    expect(next.swipe).toEqual({ x: -40, y: -10, at: 1100 });
    expect(swiped(next.swipe, 20, 0, 1500, 200, 100).begin, 'a pause starts a new swipe').toBe(true);
    const far = swiped({ x: -195, y: 0, at: 1000 }, 20, 0, 1100, 200, 100);
    expect(far, 'a swipe past the edge of the picture carries on as a fresh drag').toEqual({ swipe: { x: -20, y: -0, at: 1100 }, begin: true });
  });
});
