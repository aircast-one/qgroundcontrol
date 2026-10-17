import { describe, expect, it } from 'vitest';
import { motorCount, motorCountNotice, reportedMotors } from '@/ui/MotorsScreen';

describe('MotorsTest', () => {
  it('a vehicle that names its motor count is believed', () => {
    expect(motorCount(6)).toBe(6);
    expect(motorCountNotice(6)).toBeNull();
  });

  it('an airframe with no published layout gets eight buttons rather than none', () => {
    expect(motorCount(null)).toBe(8);
  });

  it("an unknown count warns with MotorComponent's text", () => {
    expect(motorCountNotice(null)).toBe('Warning: Unable to determine motor count');
  });

  it('an absent count is a null rather than a number to be sifted', () => {
    expect(
      reportedMotors({ connected: true, motorCount: null }),
      'view.frame already drops the -1 QGC answers for an airframe it has no layout for, and holds the count back for a submarine until its parameters arrive - a rule this head cannot express from a raw motorCount',
    ).toBeNull();
    expect(reportedMotors({ connected: true, motorCount: 4 })).toBe(4);
    expect(reportedMotors({ connected: false })).toBeNull();
    expect(reportedMotors(null)).toBeNull();
  });

  it('a count below one is not a layout', () => {
    expect(reportedMotors({ motorCount: 0 })).toBeNull();
  });
});
