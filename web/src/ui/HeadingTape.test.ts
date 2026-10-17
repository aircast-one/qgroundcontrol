import { describe, expect, it } from 'vitest';
import { headingReadout, relativeBearing, tapeLabel, tapeLabelShown, tapeTicks, tapeX } from './HeadingTape';

describe('HeadingTapeTest', () => {
  it('aBearingIsPlacedRelativeToTheHeadingAcrossNorth', () => {
    expect(relativeBearing(10, 350)).toBeCloseTo(20);
    expect(relativeBearing(350, 10)).toBeCloseTo(-20);
    expect(relativeBearing(180, 0)).toBeCloseTo(-180);
    expect(relativeBearing(725, 5)).toBeCloseTo(0);
  });

  it('theHeadingSitsInTheMiddleAndTheSpanFillsTheWidth', () => {
    expect(tapeX(0, 320)).toBeCloseTo(160);
    expect(tapeX(-60, 320)).toBeCloseTo(0);
    expect(tapeX(60, 320)).toBeCloseTo(320);
  });

  it('cardinalPointsAreLettersAndTheRestAreDegrees', () => {
    expect(tapeLabel(360)).toBe('N');
    expect(tapeLabel(90)).toBe('E');
    expect(tapeLabel(-90)).toBe('W');
    expect(tapeLabel(-30)).toBe('330');
    expect(tapeLabel(120)).toBe('120');
  });

  it('ticksCoverTheVisibleSpanInTenDegreeSteps', () => {
    const ticks = tapeTicks(102);
    expect(ticks[0]).toBe(50);
    expect(ticks.at(-1)).toBe(160);
    expect(ticks).toHaveLength(12);
  });

  it('labelsClearTheReadoutAndTheEdges', () => {
    expect(tapeLabelShown(12)).toBe(false);
    expect(tapeLabelShown(-18)).toBe(true);
    expect(tapeLabelShown(48)).toBe(true);
    expect(tapeLabelShown(58)).toBe(false);
  });

  it('theReadoutIsWholeDegrees', () => {
    expect(headingReadout(7.4)).toBe('7°');
    expect(headingReadout(359.6)).toBe('0°');
    expect(headingReadout(-90)).toBe('270°');
  });
});
