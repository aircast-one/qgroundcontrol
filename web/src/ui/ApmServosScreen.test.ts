import { describe, expect, it } from 'vitest';
import { apmServos, stepped } from './ApmServosScreen';

describe('ApmServosScreenTest', () => {
  it('an_idle_output_has_no_reading_and_the_steppers_move_by_one', () => {
    const servos = apmServos(
      JSON.parse(`{"servos":[{"index":3,"pwm":null,"position":null,
        "min":{"kind":"control","control":"number","name":"SERVO3_MIN","path":"p.min","value":1100,"valueString":"1100"},
        "function":null,"trim":null,"max":null,"reversed":null}]}`),
    );
    expect(servos).toHaveLength(1);
    const servo = servos[0];
    expect(servo?.index).toBe(3);
    expect(servo?.pwm).toBeNull();
    expect(servo?.position).toBeNull();
    expect(servo?.min === null || servo?.min === undefined ? null : stepped(servo.min, -1)).toBe(1099.0);
    expect(apmServos(null)).toEqual([]);
  });
});
