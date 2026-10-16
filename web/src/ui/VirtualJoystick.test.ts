import { describe, expect, it } from 'vitest';
import type { Offset } from './FlyShell';
import { flyState } from './FlyState';
import {
  CENTRED,
  type Sticks,
  type VirtualJoystickState,
  dragged,
  joystickValues,
  knobOffset,
  knobTravel,
  onTheGround,
  released,
  shownSticks,
  stickAxes,
  stickPad,
  stickValues,
  sticksReset,
} from './VirtualJoystick';

const at = (x: number, y: number): Offset => ({ x, y });

describe('VirtualJoystickTest', () => {
  const copter: VirtualJoystickState = { enabled: true, show: true, sending: true, autoCenterThrottle: true, leftHandedMode: false, leftPositiveOnly: true, rightPositiveOnly: false, periodMs: 40 };

  const fly = (state: string, connected = true) => flyState({ class: 'FlyState', connected, state });

  const thrust = (state: VirtualJoystickState, sticks: Sticks) => stickValues(state, sticks)[3]!;

  it('the pad maps its position the way JoystickThumbPad does', () => {
    expect(stickAxes(0.5, 0.5, false)).toEqual({ x: 0, y: 0 });
    expect(stickAxes(0, 0, false)).toEqual({ x: -1, y: 1 });
    expect(stickAxes(1, 0.5, true)).toEqual({ x: 1, y: 0.5 });
    expect(stickAxes(0.5, 1, true)).toEqual({ x: 0, y: 0 });
  });

  it('the right stick flies roll and pitch unless the layout is left handed', () => {
    const throttle = { x: 0.1, y: 0.9 };
    const attitude = { x: 0.3, y: -0.4 };
    expect(joystickValues(throttle, attitude, false)).toEqual([0.3, -0.4, 0.1, 0.9]);
    expect(joystickValues(throttle, attitude, true)).toEqual([0.1, 0.9, 0.3, -0.4]);
  });

  it('untouched sticks rest centred, so in the air the throttle holds whatever the centring setting, and values follow the current layout', () => {
    const state = { ...copter, autoCenterThrottle: false };
    expect(stickValues(state, shownSticks(state, null, null, false, false, false))).toEqual(joystickValues(stickAxes(0.5, 0.5, true), stickAxes(0.5, 0.5, false), false));
    const held = at(0.2, 0.3);
    const leftHanded = { ...state, leftHandedMode: true };
    expect(stickValues(leftHanded, shownSticks(leftHanded, held, CENTRED, false, true, false))).toEqual(joystickValues(stickAxes(0.2, 0.3, true), stickAxes(0.5, 0.5, false), true));
  });

  it('on the ground the throttle rests at zero so ArduCopter will arm, and once flying it centres again', () => {
    const centred = shownSticks(copter, null, null, true, false, false);
    expect(thrust(copter, centred), 'ArduPilot refuses to arm with the throttle off zero: Throttle (RC3) is not neutral').toBe(0);
    expect(centred.left, 'the knob is drawn where the throttle is').toEqual(at(0.5, 1));
    expect(thrust(copter, shownSticks(copter, at(0.5, 0.2), null, true, true, false)), 'a held stick sends what the thumb holds').toBeCloseTo(0.8, 3);
    expect(thrust(copter, shownSticks(copter, null, null, false, false, false)), 'in the air an untouched auto-centred throttle holds altitude').toBe(0.5);
    const rover = { ...copter, leftPositiveOnly: false };
    expect(thrust(rover, shownSticks(rover, null, null, true, false, false)), "a rover's centred throttle already means stop, so it is not parked at full reverse").toBe(0);
  });

  it('only a vehicle known to be on the ground parks the throttle', () => {
    expect(onTheGround(fly('disarmed'))).toBe(true);
    expect(onTheGround(fly('armed'))).toBe(true);
    expect(onTheGround(fly('flying'))).toBe(false);
    expect(onTheGround(fly('landing'))).toBe(false);
    expect(onTheGround(fly('contactLost')), 'a lost link may be in the air: zero throttle there would drop it').toBe(false);
    expect(onTheGround(fly('disarmed', false))).toBe(false);
    expect(onTheGround(null)).toBe(false);
  });

  it('the throttle stick follows the auto-centre setting and the attitude stick always centres, whichever hand flies it', () => {
    const leftHanded = { ...copter, autoCenterThrottle: false, leftHandedMode: true, leftPositiveOnly: false, rightPositiveOnly: true };
    const untouched = shownSticks(leftHanded, null, null, false, false, false);
    expect(untouched.left, 'pitch and roll rest centred, not at full pitch').toEqual(CENTRED);
    expect(untouched.right).toEqual(CENTRED);
    expect(stickValues(leftHanded, untouched)).toEqual([0, 0, 0, 0.5]);
    expect(stickValues(leftHanded, shownSticks(leftHanded, null, null, true, false, false)), 'on the ground the throttle on the right hand parks at zero').toEqual([0, 0, 0, 0]);
    expect(released(at(0.9, 0.1), false, false, false)).toEqual(CENTRED);
    expect(released(at(0.9, 0.1), true, true, true)).toEqual(CENTRED);
    expect(released(at(0.2, 0.3), true, false, false), 'without auto-centre the throttle stays where it was let go').toEqual(at(0.5, 0.3));
    expect(released(at(0.2, 0.3), true, false, true), 'let go on the ground it is kept centred, so it holds rather than drops once the aircraft is flying').toEqual(CENTRED);
  });

  it('held sticks are dropped when the joystick is turned off or its layout changes, and kept otherwise', () => {
    const shown = { ...copter, autoCenterThrottle: false };
    expect(sticksReset(shown, { ...shown, enabled: false })).toBe(true);
    expect(sticksReset(shown, { ...shown, show: false }), 'sticks hidden while the autopilot flies keep their released positions').toBe(false);
    expect(sticksReset(shown, { ...shown, autoCenterThrottle: true })).toBe(true);
    expect(sticksReset(shown, { ...shown, leftHandedMode: true }), 'a throttle held low must not become a full pitch on the other hand').toBe(true);
    expect(sticksReset(null, null)).toBe(true);
    expect(sticksReset(shown, { ...shown, sending: false })).toBe(false);
  });

  it('a pad is DJI sized, never too small to steer or so big it crowds the screen', () => {
    expect(stickPad(411)).toBeCloseTo(172.6, 1);
    expect(stickPad(200)).toBe(96);
    expect(stickPad(800)).toBe(200);
    expect(stickPad(411, 150), 'a pad shrinks to the room the layout has for it').toBe(150);
  });

  it('the knob follows the thumb one to one and stays inside the ring', () => {
    const travel = knobTravel(100);
    expect(travel).toBeCloseTo(28, 3);
    expect(dragged(CENTRED, at(0, 0), travel)).toEqual(CENTRED);
    expect(dragged(CENTRED, at(travel, 0), travel), 'a thumb moved the full travel deflects the stick fully').toEqual(at(1, 0.5));
    expect(dragged(CENTRED, at(-500, 500), travel)).toEqual(at(0, 1));
    expect(knobOffset(at(1, 0.5), travel)).toEqual(at(travel, 0));
    const corner = knobOffset(at(1, 0), travel);
    expect(Math.hypot(corner.x, corner.y), 'a corner deflection draws on the ring, not outside it').toBeCloseTo(travel, 3);
  });
});
