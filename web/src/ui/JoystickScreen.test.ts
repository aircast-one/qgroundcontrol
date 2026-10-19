import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { hatBits, scaled } from '@/app/GamepadInput';
import { JoystickCalibration, joystickCalibration, joystickPage, statusLines } from './JoystickScreen';

const json = (text: string): Json => parseJson(text) ?? {};

describe('JoystickScreenTest', () => {
  it('the_standby_line_comes_from_the_core', () => {
    const standing = joystickPage(json('{"available":true,"names":["Radio"],"active":"Radio","standby":{"capable":true,"enabled":true,"text":"The radio flies the aircraft. 4G is standing by."}}'));
    expect(standing?.standbyText).toBe('The radio flies the aircraft. 4G is standing by.');
    expect(joystickPage(json('{"available":true,"names":["Pad"],"active":"Pad","standby":{"capable":false,"enabled":false,"text":null}}'))?.standbyText).toBeNull();
  });

  it('the_page_reads_devices_settings_and_axes', () => {
    expect(joystickPage(json('{"available":false}'))).toBeNull();
    const page = joystickPage(
      json(`{"available":true,"names":["Pad"],"active":"Pad","vehicle":true,"enabled":false,"calibrated":false,
            "settings":[{"name":"useDeadband","type":"bool","label":"Use deadband","units":"","value":true}],
            "state":{"axes":[{"index":0,"raw":1200,"function":"roll","label":"Roll"}]}}`),
    )!;
    expect(page.active).toBe('Pad');
    expect(page.settings).toHaveLength(1);
    expect(page.settings[0]!.value).toBe(true);
    expect(page.axes).toEqual([{ index: 0, raw: 1200, label: 'Roll' }]);
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
    const step = joystickCalibration(json('{"calibrating":true,"status":{"bullets":false,"lines":["Move the Throttle stick"]},"nextText":"Next","nextEnabled":false,"cancelEnabled":true,"oneSidedVisible":true}'));
    expect(step).toEqual(
      JoystickCalibration({ calibrating: true, status: { bullets: false, lines: ['Move the Throttle stick'] }, nextText: 'Next', nextEnabled: false, cancelEnabled: true, oneSidedVisible: true }),
    );
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
            "live":{"roll":-0.9,"pitch":0.1,"yaw":0,"throttle":-1},"livePositions":[0,-1,-0.9,0.1]}`),
    );
    expect([cal.step, cal.steps, cal.holding, cal.checking, cal.warning]).toEqual([4, 10, 0.5, true, 'Roll moved the opposite way to yaw.']);
    expect(cal.live).toEqual({ roll: -0.9, pitch: 0.1, yaw: 0, throttle: -1 });
    expect(cal.livePositions).toEqual([0, -1, -0.9, 0.1]);
    const idle = joystickCalibration(null);
    expect([idle.step, idle.holding, idle.warning, idle.checking]).toEqual([null, null, null, false]);
  });

  it('the stick mode and whether the sticks reach the aircraft are worded by the core', () => {
    const page = joystickPage(
      json(`{"available":true,"names":["Pad"],"active":"Pad","enabled":true,"transmitterMode":2,"transmitterModeText":"Throttle and yaw on the left stick, pitch and roll on the right",
            "sendingText":"Not sending: no input from the radio. Check its cable and move a stick","state":{"commanding":false,"notCommandingReason":"noSample"}}`),
    )!;
    expect(page.transmitterModeText).toBe('Throttle and yaw on the left stick, pitch and roll on the right');
    expect([page.commanding, page.sendingText]).toEqual([false, 'Not sending: no input from the radio. Check its cable and move a stick']);
    expect(joystickPage(json('{"available":true,"names":["Pad"],"active":"Pad","enabled":false,"sendingText":null}'))?.sendingText).toBeNull();
  });

  it('a step the core hands as a list is read as one', () => {
    expect(statusLines(json('{"bullets":true,"lines":["Center all sticks.","Click Next to continue"]}'))).toEqual({ bullets: true, lines: ['Center all sticks.', 'Click Next to continue'] });
    expect(statusLines(json('{"bullets":false,"lines":["Move the Roll stick all the way to the right"]}'))).toEqual({ bullets: false, lines: ['Move the Roll stick all the way to the right'] });
    expect(joystickCalibration(null).status).toEqual({ bullets: false, lines: [] });
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
