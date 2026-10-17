import { describe, expect, it } from 'vitest';
import { stepsFor } from './Inputs';

describe('stepsFor gives a slider the keyboard step its caller rounds to', () => {
  it('counts the stops between the ends', () => {
    expect(stepsFor(0, 500, 1)).toBe(499);
    expect(stepsFor(0, 359, 1)).toBe(358);
    expect(stepsFor(0, 10, 0.01)).toBe(999);
    expect(stepsFor(5, 5, 1)).toBe(0);
  });
});
