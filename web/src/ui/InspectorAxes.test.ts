import { describe, expect, it } from 'vitest';
import { timeTicks, valueTicks } from './InspectorCharts';

describe('InspectorAxesTest', () => {
  it('the time axis ticks every third of the window in local mm_ss', () => {
    const utc = () => 0;
    const ticks = timeTicks(30_000, 125_000, utc);
    expect(ticks.map((it) => it[0])).toEqual([0, 1 / 3, 2 / 3, 1]);
    expect(ticks.map((it) => it[1])).toEqual(['01:35', '01:45', '01:55', '02:05']);
    const kolkata = () => 5.5 * 3_600_000;
    expect(timeTicks(30_000, 125_000, kolkata).at(-1)?.[1]).toBe('32:05');
  });

  it('value ticks climb from the bottom of the range to the top', () => {
    const ticks = valueTicks(0.0, 100.0);
    expect([ticks[0]?.[1], ticks[0]?.[0]]).toEqual(['0.000', 1]);
    expect([ticks.at(-1)?.[1], ticks.at(-1)?.[0]]).toEqual(['100.0', 0]);
  });
});
