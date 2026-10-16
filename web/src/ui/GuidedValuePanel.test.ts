import { describe, expect, it } from 'vitest';
import { Readiness } from './FlyState';
import { holdLabel } from './HoldToConfirm';
import { guidedLabel } from './GuidedValueFlow';
import {
  guidedBounds,
  guidedCommitLabel,
  guidedCompact,
  guidedPresets,
  guidedQuickPicks,
  guidedReading,
  guidedRounded,
  guidedStepped,
  guidedTyped,
  guidedValueText,
  guidedVehicle,
} from './GuidedValuePanel';

describe('GuidedStepperTest', () => {
  it('the hold names its target and says anyway only for a non-blocking warning', () => {
    expect(guidedCommitLabel('Take off · 3.0 m', null)).toBe('Take off · 3.0 m');
    expect(guidedCommitLabel('Take off · 3.0 m', Readiness({ text: 'GPS off', blocks: false }))).toBe('Take off anyway · 3.0 m');
    expect(guidedCommitLabel('Take off · 3.0 m', Readiness({ text: 'GPS off', blocks: true }))).toBe('Take off · 3.0 m');
    expect(holdLabel(guidedCommitLabel('Take off · 3.0 m', Readiness({ text: 'GPS off', blocks: false })))).toBe('Slide to take off anyway · 3.0 m');
  });

  it('takeoff presets are the common heights the vehicle allows', () => {
    expect(guidedPresets('m', 3.0, 121.9)).toEqual([5.0, 10.0, 20.0, 50.0]);
    expect(guidedPresets('m', 3.0, 15.0)).toEqual([5.0, 10.0]);
    expect(guidedPresets('ft', 10.0, 400.0)).toEqual([15.0, 30.0, 60.0, 150.0]);
  });

  it('quick picks climb ten and twenty from the target and stop at the maximum', () => {
    expect(guidedQuickPicks(42.0, 1.0, 120.0, 'm')).toEqual([
      ['+10 m', 52.0],
      ['+20 m', 62.0],
      ['Max 120 m', 120.0],
    ]);
    expect(guidedQuickPicks(115.0, 1.0, 120.0, 'm')[1]?.[1]).toBeCloseTo(120.0, 9);
  });

  it('a step moves one unit and stays inside the range, like GuidedValueSlider step', () => {
    expect(guidedStepped(25.0, 1, 1.0, 120.0, 'm')).toBeCloseTo(26.0, 9);
    expect(guidedStepped(119.6, 1, 1.0, 120.0, 'm')).toBeCloseTo(120.0, 9);
    expect(guidedStepped(1.4, -1, 1.0, 120.0, 'm')).toBeCloseTo(1.0, 9);
  });

  it('values round to tenths in metric and whole numbers otherwise', () => {
    expect(guidedRounded(25.43, 'm')).toBeCloseTo(25.4, 9);
    expect(guidedRounded(25.43, 'ft')).toBeCloseTo(25.0, 9);
    expect(guidedValueText(25.4, 'm')).toBe('25.4');
    expect(guidedValueText(25.0, 'ft')).toBe('25');
  });

  it('a typed value is clamped and rounded, and garbage is ignored', () => {
    expect(guidedTyped('500', 1.0, 120.0, 'm')).toBeCloseTo(120.0, 9);
    expect(guidedTyped('12,46', 1.0, 120.0, 'm')).toBeCloseTo(12.5, 9);
    expect(guidedTyped('x', 1.0, 120.0, 'm')).toBeNull();
    expect(guidedBounds(null, 10.0)).toBeNull();
  });

  it('the reading carries its unit on one line', () => {
    expect(guidedReading(10.0, 'ft')).toBe('10 ft');
    expect(guidedReading(12.5, 'm')).toBe('12.5 m');
    expect(guidedReading(3.0, '')).toBe('3');
  });

  it('the panel names the vehicle only when there is a choice of vehicles', () => {
    expect(guidedVehicle(1, 'Quadrotor 128')).toBeNull();
    expect(guidedVehicle(2, 'Quadrotor 128')).toBe('Quadrotor 128');
    expect(guidedVehicle(2, null)).toBeNull();
  });
});

describe('GuidedCompactTest', () => {
  it('a landscape phone folds the panel so the hold button never pushes the value out of sight', () => {
    expect([guidedCompact(false, 344), guidedCompact(false, 600), guidedCompact(true, 344)]).toEqual([true, false, false]);
  });

  it('the folded panel carries the range beside the label instead of a line of its own', () => {
    expect(guidedLabel('Height above launch', '3.0 to 100.0 m', true)).toBe('Height above launch · 3.0 to 100.0 m');
    expect(guidedLabel('Height above launch', '3.0 to 100.0 m', false)).toBe('Height above launch');
    expect(guidedLabel('Height above launch', null, true)).toBe('Height above launch');
  });
});
