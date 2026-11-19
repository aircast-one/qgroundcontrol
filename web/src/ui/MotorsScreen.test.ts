import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { canStop, canTest, firstRefusal, motorGate, motorLabel, motorRefusal, motorTestNote, testedMotors } from '@/ui/MotorsScreen';

const frame = ({ connected = true, armed = false, contactLost = false as boolean | null } = {}): Json => ({
  kind: 'object',
  class: 'Frame',
  connected,
  armed,
  contactLost,
  apmFirmware: false,
  motorCount: 4,
});

describe('MotorGateTest', () => {
  it('an armed vehicle is not a vehicle to spin a motor on', () => {
    expect(canTest(motorGate(frame({ armed: true })), true)).toBe(false);
    expect(motorRefusal(motorGate(frame({ armed: true })))).toBe('The vehicle is armed. Disarm it before testing a motor.');
  });

  it('a vehicle that stopped answering is not one to spin a motor on', () => {
    expect(canTest(motorGate(frame({ contactLost: true })), true)).toBe(false);
  });

  it('nobody watching is neither lost nor fine, and must not refuse', () => {
    expect(
      canTest(motorGate(frame({ contactLost: null })), true),
      'frame.rs withholds contactLost when the watch is off, because the raw flag stays false however long the vehicle has been silent. Refusing on unknown would ground a motor test that has nothing wrong with it',
    ).toBe(true);
    expect(motorRefusal(motorGate(frame({ contactLost: null })))).toBeNull();
  });

  it('no vehicle means nothing will answer', () => {
    expect(canTest(motorGate(frame({ connected: false })), true)).toBe(false);
    expect(canTest(motorGate(null), true)).toBe(false);
  });

  it('the safety switch is still required with everything else in order', () => {
    expect(canTest(motorGate(frame()), false)).toBe(false);
    expect(canTest(motorGate(frame()), true)).toBe(true);
  });

  it("stop is not a test and does not inherit a test's gate", () => {
    expect(
      canStop(motorGate(frame({ armed: true }))),
      'the two conditions that are REASONS to stop - the vehicle arming while the motors turn, or the safety switch going off - both disabled the one control an operator reaches for. A control is enabled by what its own action requires, and sending zero throttle requires only a vehicle to send it to',
    ).toBe(true);
    expect(canStop(motorGate(frame({ contactLost: true })))).toBe(true);
    expect(canStop(motorGate(frame({ connected: false })))).toBe(false);
  });

  it('ArduPilot motors are tested by letter', () => {
    expect(motorLabel(3, true)).toBe('C');
    expect(motorLabel(3, false)).toBe('3');
  });

  it('a motor that would not spin says why, so a click is never silent', () => {
    expect(firstRefusal([null, 'The vehicle did not take the motor test.', 'The vehicle did not answer.'])).toBe('The vehicle did not take the motor test.');
    expect(firstRefusal([null, null])).toBeNull();
  });
});

describe('TestedMotorsTest', () => {
  it('a rover tests the outputs the core names instead of eight unknown motors', () => {
    const rover: Json = { ...frame(), apmFirmware: true, motorCount: 2, motors: [{ motor: 1, label: 'Throttle' }, { motor: 2, label: 'Steering' }] };
    expect(testedMotors(rover, 2, true)).toEqual([
      { motor: 1, label: 'Throttle' },
      { motor: 2, label: 'Steering' },
    ]);
    expect(testedMotors(frame(), 4, true).map((it) => it.label)).toEqual(['Motor A', 'Motor B', 'Motor C', 'Motor D']);
    expect(testedMotors(frame(), null, false)).toHaveLength(8);
  });

  it('a fixed-wing plane says why it has no motor test instead of offering motors that will not answer', () => {
    expect(motorTestNote({ ...frame(), motorTestNote: 'ArduPilot only spins VTOL lift motors in a motor test.' })).toBe('ArduPilot only spins VTOL lift motors in a motor test.');
    expect(motorTestNote(frame())).toBeNull();
  });
});
