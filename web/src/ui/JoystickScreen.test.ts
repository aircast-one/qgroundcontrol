import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { hatBits, scaled } from '@/app/GamepadInput';
import { JoystickCalibration, MODE_STICKS, axisLabel, joystickCalibration, joystickPage, sendingText, statusLines } from './JoystickScreen';

const json = (text: string): Json => parseJson(text) ?? {};

describe('JoystickScreenTest', () => {
  it('the_page_reads_devices_settings_and_axes', () => {
    expect(joystickPage(json('{"available":false}'))).toBeNull();
    const page = joystickPage(
      json(`{"available":true,"names":["Pad"],"active":"Pad","vehicle":true,"enabled":false,"calibrated":false,
            "settings":[{"name":"useDeadband","type":"bool","label":"Use deadband","units":"","value":true}],
            "state":{"axes":[{"index":0,"raw":1200,"function":"roll"}]}}`),
    )!;
    expect(page.active).toBe('Pad');
    expect(page.settings).toHaveLength(1);
    expect(page.settings[0]!.value).toBe(true);
    expect(page.axes).toEqual([{ index: 0, raw: 1200, function: 'roll' }]);
  });

  it('gamepad_values_scale_to_the_core_range_and_the_dpad_becomes_hat_bits', () => {
    expect(scaled(1)).toBe(32767);
    expect(scaled(-2)).toBe(-32767);
    expect(hatBits(-1, -1)).toBe(0x01 | 0x08);
    expect(hatBits(1, 1)).toBe(0x04 | 0x02);
    expect(hatBits(0.2, -0.2)).toBe(0);
  });

  it('the_calibration_panel_reads_the_core_wizard', () => {
    const idle = joystickCalibration(null);
    expect(idle.nextText).toBe('Calibrate');
    const step = joystickCalibration(json('{"calibrating":true,"statusText":"Move the Throttle stick","nextText":"Next","nextEnabled":false,"cancelEnabled":true,"oneSidedVisible":true}'));
    expect(step).toEqual(JoystickCalibration({ calibrating: true, statusText: 'Move the Throttle stick', nextText: 'Next', nextEnabled: false, cancelEnabled: true, oneSidedVisible: true }));
  });

  it('buttons_show_their_action_and_whether_they_are_held', () => {
    const page = joystickPage(
      json(`{"available":true,"names":["Pad"],"active":"Pad","vehicle":true,"enabled":true,"calibrated":true,"settings":[],
            "state":{"axes":[],"buttons":[{"index":0,"action":null,"repeat":false,"event":"none"},{"index":1,"action":"Step Zoom In","repeat":true,"event":"repeat"}]},
            "assignableActions":[{"action":"No Action","canRepeat":false},{"action":"Step Zoom In","canRepeat":true}]}`),
    )!;
    expect(page.buttons).toEqual([
      { index: 0, action: 'No Action', repeat: false, pressed: false },
      { index: 1, action: 'Step Zoom In', repeat: true, pressed: true },
    ]);
    expect(page.actions[1]).toEqual({ action: 'Step Zoom In', canRepeat: true });
  });

  it('calibration reads the stick diagram positions', () => {
    const cal = joystickCalibration(json('{"calibrating":true,"stickPositions":[0,1,0,0],"singleStickDisplay":false}'));
    expect(cal.stickPositions).toEqual([0, 1, 0, 0]);
    expect(joystickCalibration(null).stickPositions).toEqual([0, 0, 0, 0]);
  });

  it('calibration reads the step count, the held stick, the live sticks and the check warning', () => {
    const cal = joystickCalibration(
      json(`{"calibrating":true,"step":4,"steps":10,"holding":0.5,"checking":true,"warning":"Roll moved the opposite way to yaw.",
            "live":{"roll":-0.9,"pitch":0.1,"yaw":0,"throttle":-1},"livePositions":[0,-1,-0.9,0.1],
            "channels":[{"index":0,"raw":0,"function":"roll"},{"index":1,"raw":0,"function":null}]}`),
    );
    expect([cal.step, cal.steps, cal.holding, cal.checking, cal.warning]).toEqual([4, 10, 0.5, true, 'Roll moved the opposite way to yaw.']);
    expect(cal.live).toEqual({ roll: -0.9, pitch: 0.1, yaw: 0, throttle: -1 });
    expect(cal.livePositions).toEqual([0, -1, -0.9, 0.1]);
    expect(cal.captured).toEqual(['roll', null]);
    const idle = joystickCalibration(null);
    expect([idle.step, idle.holding, idle.warning, idle.checking]).toEqual([null, null, null, false]);
  });

  it('axes are named by what calibration has captured so far, then by the saved mapping', () => {
    const calibrating = JoystickCalibration({ calibrating: true, statusText: '', nextText: 'Next', nextEnabled: false, cancelEnabled: true, oneSidedVisible: false, captured: ['throttle', null] });
    expect(axisLabel({ index: 0, raw: 0, function: 'roll' }, calibrating)).toBe('Throttle');
    expect(axisLabel({ index: 1, raw: 0, function: 'pitch' }, calibrating)).toBe('Axis 2');
    expect(axisLabel({ index: 1, raw: 0, function: 'pitch' }, joystickCalibration(null))).toBe('Pitch');
    expect(axisLabel({ index: 5, raw: 0, function: '' }, joystickCalibration(null))).toBe('Axis 6');
  });

  it('each stick mode says which stick carries the throttle', () => {
    expect(MODE_STICKS[2]).toBe('Throttle and yaw on the left stick, pitch and roll on the right');
    expect(Object.keys(MODE_STICKS)).toEqual(['1', '2', '3', '4']);
  });

  it('an enabled joystick says whether its sticks reach the aircraft, and why not', () => {
    const page = joystickPage(json('{"available":true,"names":["Pad"],"active":"Pad","enabled":true,"state":{"commanding":false,"notCommandingReason":"noSample"}}'))!;
    expect([page.commanding, page.notCommandingReason]).toEqual([false, 'noSample']);
    expect(sendingText(page)).toBe('Not sending: no input from the radio. Check its cable and move a stick');
    expect(sendingText({ enabled: true, commanding: true, notCommandingReason: null })).toBe('Sending your sticks to the aircraft');
    expect(sendingText({ enabled: false, commanding: false, notCommandingReason: 'notPolling' })).toBeNull();
    expect(sendingText({ enabled: true, commanding: false, notCommandingReason: 'somethingNew' })).toBe('Not sending your sticks to the aircraft');
  });

  it('a step written as bullets is shown as a list', () => {
    expect(statusLines('* Center all sticks.\n* Click Next to continue')).toEqual({ bullets: true, lines: ['Center all sticks.', 'Click Next to continue'] });
    expect(statusLines('Move the Roll stick all the way to the right')).toEqual({ bullets: false, lines: ['Move the Roll stick all the way to the right'] });
    expect(statusLines('')).toEqual({ bullets: false, lines: [''] });
  });

  it('a_setting_the_core_hides_is_not_listed', () => {
    const page = joystickPage(
      json('{"available":true,"names":["Pad"],"active":"Pad","settings":[{"name":"negativeThrust","type":"bool","label":"Negative thrust","units":"","value":false,"visible":false},{"name":"exponentialPct","type":"double","label":"Exponential","units":"%","value":0}]}'),
    )!;
    expect(page.settings.map((it) => it.name)).toEqual(['exponentialPct']);
    const ranged = joystickPage(
      json('{"available":true,"names":["Pad"],"active":"Pad","settings":[{"name":"exponentialPct","type":"double","label":"Stick Exponential","units":"%","value":0,"slider":{"from":0.0,"to":50.0,"decimals":1}}]}'),
    )!;
    expect(ranged.settings).toHaveLength(1);
    expect(ranged.settings[0]!.slider).toEqual({ from: 0, to: 50, decimals: 1, hint: '' });
  });
});
