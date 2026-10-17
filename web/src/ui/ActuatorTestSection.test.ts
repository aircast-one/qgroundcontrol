import { describe, expect, it } from 'vitest';
import { TestChannel, actuatorActions, sentValue, snapped } from './ActuatorTestSection';

describe('ActuatorTestSectionTest', () => {
  const motor = TestChannel('Motor 1', 101, 0.0, 1.0, null, true);
  const servo = TestChannel('Servo 1', 201, -1.0, 1.0, 0.0, false);

  it('a motor slider has a stop zone below min that snaps like ActuatorSlider', () => {
    expect(motor.from).toBeCloseTo(-0.15, 9);
    expect(motor.rest).toBeCloseTo(-0.15, 9);
    expect(snapped(motor, -0.1)).toBeCloseTo(-0.15, 9);
    expect(snapped(motor, -0.05)).toBeCloseTo(0.0, 9);
    expect(sentValue(motor, -0.15)).toBeNull();
    expect(sentValue(motor, 0.4)).toBeCloseTo(0.4, 9);
  });

  it('a servo slider starts at its default and has no stop zone', () => {
    expect(servo.from).toBeCloseTo(-1.0, 9);
    expect(servo.rest).toBeCloseTo(0.0, 9);
    expect(snapped(servo, -1.0)).toBeCloseTo(-1.0, 9);
  });
});

describe('ActuatorActionsTest', () => {
  it('action groups read from the outputs view', () => {
    const groups = actuatorActions(JSON.parse('{"actions":[{"label":"Set Spin Direction 1","type":4,"actions":[{"label":"Motor 1","function":101}]}]}'));
    expect(groups).toEqual([{ label: 'Set Spin Direction 1', type: 4, actions: [{ label: 'Motor 1', function: 101 }] }]);
  });
});
