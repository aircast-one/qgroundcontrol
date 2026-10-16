import { describe, expect, it } from 'vitest';
import { RESET_ARM_MILLIS, ResetTap, clampedDrag, hiddenKeys, movedKey, onScreenCorrection, orderedKeys, orientedKey, resetPillText, resetTap, storedIndicatorOrder, storedOffsets, withHidden } from './OverlayLayout';

const rect = (left: number, top: number, right: number, bottom: number) => ({ left, top, right, bottom });

describe('OverlayLayoutTest', () => {
  it('hidden widgets are read from the stored rig keys only', () => {
    const stored = { 'OverlayRigHidden-orbit': true, 'OverlayRigHidden-traffic': false, other: true };
    expect(hiddenKeys(stored)).toEqual(new Set(['orbit']));
  });

  it('hiding and showing toggles one key', () => {
    expect(withHidden(new Set(['a']), 'b', true)).toEqual(new Set(['a', 'b']));
    expect(withHidden(new Set(['a']), 'a', false)).toEqual(new Set());
  });

  it('reset layout arms on the first tap and resets on the second, like resetPill', () => {
    expect(resetTap(false)).toEqual(ResetTap({ reset: false, armed: true }));
    expect(resetTap(true)).toEqual(ResetTap({ reset: true, armed: false }));
    expect(resetPillText(false)).toBe('Reset layout');
    expect(resetPillText(true)).toBe('Tap again to reset');
    expect(RESET_ARM_MILLIS).toBe(4000);
  });

  it('a dragged widget stops at the screen edges', () => {
    expect(clampedDrag(100, 40, 200, 140, 1000, 2000, 30, -80)).toEqual([30, -40]);
    expect(clampedDrag(100, 40, 200, 140, 1000, 2000, -500, 5000)).toEqual([-100, 1860]);
    expect(clampedDrag(100, 40, 200, 140, 1000, 2000, 900, 0)).toEqual([800, 0]);
  });

  it('stored offsets are read back per widget and malformed ones are dropped', () => {
    const stored = { 'OverlayRigOffset-instrumentPanel': '12.5,-30.0', 'OverlayRigOffset-orbit': 'bad', 'OverlayRigHidden-traffic': true };
    expect(storedOffsets(stored)).toEqual(new Map([['instrumentPanel', [12.5, -30]]]));
  });

  it('the stored layout yields hidden widgets, offsets and the indicator order', () => {
    const stored = { 'OverlayRigOffset-instrumentPanel': '10.0,20.0', 'OverlayRigHidden-orbit': true, FlyViewIndicatorOrder: 'gps,battery' };
    expect(storedOffsets(stored)).toEqual(new Map([['instrumentPanel', [10, 20]]]));
    expect(hiddenKeys(stored)).toEqual(new Set(['orbit']));
    expect(storedIndicatorOrder(stored)).toEqual(['gps', 'battery']);
  });

  it('portrait and landscape keep separate offsets', () => {
    expect(orientedKey('instrumentPanel', false)).toBe('instrumentPanel');
    expect(orientedKey('instrumentPanel', true)).toBe('instrumentPanel@landscape');
  });

  it('a widget its own offset pushed off screen is pulled back, a scrolled or oversized one is left alone', () => {
    const root = { width: 1000, height: 500 };
    expect(onScreenCorrection(rect(900, 400, 1300, 600), root, [400, 300])).toEqual([-300, -100]);
    expect(onScreenCorrection(rect(900, 0, 1300, 100), root, [100, 0])).toEqual([-100, 0]);
    expect(onScreenCorrection(rect(-40, 0, 60, 100), root, [-80, 0])).toEqual([40, 0]);
    expect(onScreenCorrection(rect(1100, 10, 1300, 200), root, [0, 0])).toEqual([0, 0]);
    expect(onScreenCorrection(rect(10, 10, 200, 200), root, [50, 50])).toEqual([0, 0]);
    expect(onScreenCorrection(rect(20, 10, 1600, 200), root, [30, 0])).toEqual([0, 0]);
  });
});

describe('IndicatorOrderTest', () => {
  it('saved order comes first and new indicators follow in their own order', () => {
    expect(orderedKeys(['battery', 'gps', 'rc'], ['gps', 'gone', 'battery'])).toEqual(['gps', 'battery', 'rc']);
  });

  it('an indicator moves one slot and not past either end', () => {
    expect(movedKey(['battery', 'gps', 'rc'], 'battery', 1)).toEqual(['gps', 'battery', 'rc']);
    expect(movedKey(['battery', 'gps', 'rc'], 'rc', -1)).toEqual(['battery', 'rc', 'gps']);
    expect(movedKey(['battery', 'gps'], 'battery', -1)).toBeNull();
    expect(movedKey(['battery', 'gps'], 'nope', 1)).toBeNull();
  });
});
