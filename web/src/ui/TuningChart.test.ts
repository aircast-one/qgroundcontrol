import { describe, expect, it } from 'vitest';
import { type Sample as SampleType, Sample, grownRange, withSample } from './TuningChart';

describe('TuningChartTest', () => {
  it('history keeps three minutes', () => {
    const series = Array.from({ length: 201 }, (_, s) => s).reduce<readonly SampleType[]>((list, s) => withSample(list, Sample(s, s)), []);
    expect(series[0]?.seconds).toBe(20.0);
  });

  it("the y range only grows, snapped outward to fives, as PIDTuning's adjustYAxis does", () => {
    expect(grownRange(null, [])).toBeNull();
    expect(grownRange(null, [3.0]), 'the first point sets both ends').toEqual([3.0, 3.0]);
    expect(grownRange([3.0, 3.0], [-1.0, 7.0])).toEqual([-10.0, 10.0]);
    expect(grownRange([-10.0, 10.0], [2.0]), 'a value inside the range never shrinks it').toEqual([-10.0, 10.0]);
  });
});
