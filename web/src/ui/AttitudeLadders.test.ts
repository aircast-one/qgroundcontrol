import { describe, expect, it } from 'vitest';
import { LADDER_MARKS, ladderHeight, ladderLabel, ladderReadout, ladderTick, ladderY } from './AttitudeLadders';

describe('AttitudeLaddersTest', () => {
  it('the scale runs from 30 above to 30 below in fives, labelled every ten without a sign', () => {
    expect(LADDER_MARKS[0]).toBe(30);
    expect(LADDER_MARKS.at(-1)).toBe(-30);
    expect(LADDER_MARKS).toHaveLength(13);
    expect(LADDER_MARKS.map(ladderLabel).filter((it) => it !== null)).toEqual(['30', '20', '10', '10', '20', '30']);
    expect(ladderTick(0)).toBeGreaterThan(ladderTick(10));
    expect(ladderTick(10)).toBeGreaterThan(ladderTick(5));
  });

  it('a positive angle sits above the middle and a reading past the scale pins to its end', () => {
    expect(ladderY(0, 200)).toBe(100);
    expect(ladderY(15, 200)).toBeLessThan(100);
    expect(ladderY(-15, 200)).toBeGreaterThan(100);
    expect(ladderY(75, 200)).toBe(ladderY(30, 200));
    expect(ladderY(-75, 200)).toBe(ladderY(-30, 200));
  });

  it('the readout rounds to whole degrees and never shows minus zero', () => {
    expect(ladderReadout(-14.4)).toBe('-14°');
    expect(ladderReadout(-0.3)).toBe('0°');
    expect(ladderReadout(4.6)).toBe('5°');
  });

  it('the ladder grows with the screen but stays readable on a phone and bounded on a monitor', () => {
    expect(ladderHeight(390)).toBe(125);
    expect(ladderHeight(200)).toBe(110);
    expect(ladderHeight(1600)).toBe(260);
  });
});
