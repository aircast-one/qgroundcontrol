import { describe, expect, it } from 'vitest';
import { PITCH_SCALE, THROTTLE_SCALE, ladderHeight, ladderLabel, ladderMarks, ladderReach, ladderTick, ladderY } from './HudLadders';

describe('HudLaddersTest', () => {
  it('pitch runs from 30 above to 30 below in fives, labelled every ten without a sign, level marked longest', () => {
    const marks = ladderMarks(PITCH_SCALE);
    expect(marks[0]).toBe(30);
    expect(marks.at(-1)).toBe(-30);
    expect(marks).toHaveLength(13);
    expect(marks.map((it) => ladderLabel(it, PITCH_SCALE)).filter((it) => it !== null)).toEqual(['30', '20', '10', '10', '20', '30']);
    expect(ladderTick(0, PITCH_SCALE)).toBeGreaterThan(ladderTick(10, PITCH_SCALE));
    expect(ladderTick(10, PITCH_SCALE)).toBeGreaterThan(ladderTick(-5, PITCH_SCALE));
  });

  it('throttle runs from full at the top to idle at the bottom, labelled every twenty', () => {
    const marks = ladderMarks(THROTTLE_SCALE);
    expect(marks).toHaveLength(11);
    expect(marks.map((it) => ladderLabel(it, THROTTLE_SCALE)).filter((it) => it !== null)).toEqual(['100', '80', '60', '40', '20', '0']);
    expect(ladderY(100, THROTTLE_SCALE, 200)).toBeLessThan(ladderY(34, THROTTLE_SCALE, 200));
    expect(ladderY(50, THROTTLE_SCALE, 200)).toBe(100);
  });

  it('a nose-up pitch sits above level and a reading past the scale pins to its end', () => {
    expect(ladderY(0, PITCH_SCALE, 200)).toBe(100);
    expect(ladderY(15, PITCH_SCALE, 200)).toBeLessThan(100);
    expect(ladderY(-15, PITCH_SCALE, 200)).toBeGreaterThan(100);
    expect(ladderY(75, PITCH_SCALE, 200)).toBe(ladderY(30, PITCH_SCALE, 200));
    expect(ladderY(-75, PITCH_SCALE, 200)).toBe(ladderY(-30, PITCH_SCALE, 200));
    expect(ladderY(140, THROTTLE_SCALE, 200)).toBe(ladderY(100, THROTTLE_SCALE, 200));
  });

  it('the readouts round to whole numbers and never show minus zero', () => {
    expect(PITCH_SCALE.readout(-14.4)).toBe('-14°');
    expect(PITCH_SCALE.readout(-0.3)).toBe('0°');
    expect(THROTTLE_SCALE.readout(33.6)).toBe('34%');
  });

  it('the ladder grows with the screen but stays readable on a phone and bounded on a monitor', () => {
    expect(ladderHeight(390)).toBe(117);
    expect(ladderHeight(200)).toBe(110);
    expect(ladderHeight(1600)).toBe(260);
  });

  it('the ladders stand closer to the crosshair on a short phone screen than on a monitor', () => {
    expect(ladderReach(390)).toBe(148);
    expect(ladderReach(800)).toBe(304);
  });
});
