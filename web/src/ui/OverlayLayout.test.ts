import { describe, expect, it } from 'vitest';
import { RESET_ARM_MILLIS, ResetTap, clampedInto, droppedOrder, hiddenKeys, jiggleDegrees, movedAmongShown, movedKey, onScreenCorrection, orderedKeys, orientedKey, resetPillText, resetTap, shownCorrection, storedIndicatorOrder, storedOffsets, visibleArea, withHidden } from './OverlayLayout';

const rect = (left: number, top: number, right: number, bottom: number) => ({ left, top, right, bottom });

describe('OverlayLayoutTest', () => {
  it('a small widget jiggles the full angle and a wide one turns less, so no corner swings more than a few pixels', () => {
    expect(jiggleDegrees({ width: 48, height: 48 })).toBe(1.2);
    const wide = { width: 1100, height: 200 };
    expect(((Math.hypot(wide.width, wide.height) / 2) * jiggleDegrees(wide) * Math.PI) / 180).toBeCloseTo(4);
  });

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
    const screen = rect(0, 0, 1000, 2000);
    expect(clampedInto(rect(100, 40, 200, 140), screen, 30, -80)).toEqual([30, -40]);
    expect(clampedInto(rect(100, 40, 200, 140), screen, -500, 5000)).toEqual([-100, 1860]);
    expect(clampedInto(rect(100, 40, 200, 140), screen, 900, 0)).toEqual([800, 0]);
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
    const root = rect(0, 0, 1000, 500);
    expect(onScreenCorrection(rect(900, 400, 1300, 600), root, [400, 300])).toEqual([-300, -100]);
    expect(onScreenCorrection(rect(900, 0, 1300, 100), root, [100, 0])).toEqual([-100, 0]);
    expect(onScreenCorrection(rect(-40, 0, 60, 100), root, [-80, 0])).toEqual([40, 0]);
    expect(onScreenCorrection(rect(1100, 10, 1300, 200), root, [0, 0])).toEqual([0, 0]);
    expect(onScreenCorrection(rect(10, 10, 200, 200), root, [50, 50])).toEqual([0, 0]);
    expect(onScreenCorrection(rect(20, 10, 1600, 200), root, [30, 0])).toEqual([0, 0]);
  });

  it('a widget dragged past the status row stops below it and clear of the safe area', () => {
    const area = visibleArea({ width: 1280, height: 800 }, rect(44, 48, 44, 21));
    expect(area).toEqual(rect(44, 48, 1236, 779));
    expect(clampedInto(rect(1000, 300, 1200, 500), area, -50, -400)).toEqual([-50, -252]);
    expect(clampedInto(rect(1000, 300, 1200, 500), area, 200, 400)).toEqual([36, 279]);
  });

  it('the on-screen correction is worked out from the stored offset, so a window shrunk and restored shows the widget where the pilot left it', () => {
    const wide = rect(0, 0, 1280, 800);
    const small = rect(0, 0, 900, 560);
    expect(shownCorrection(rect(900, 500, 1100, 700), small, [274, 160], [0, 0])).toEqual([-200, -140]);
    expect(shownCorrection(rect(700, 360, 900, 560), small, [274, 160], [-200, -140])).toEqual([-200, -140]);
    expect(shownCorrection(rect(700, 360, 900, 560), wide, [274, 160], [-200, -140])).toEqual([0, 0]);
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

  it('an indicator steps past empty ones and they keep their slots', () => {
    const keys = ['battery', 'rc', 'esc', 'gps', 'joystick'];
    const shown = new Set(['battery', 'gps']);
    expect(movedAmongShown(keys, shown, 'gps', -1)).toEqual(['gps', 'rc', 'esc', 'battery', 'joystick']);
    expect(movedAmongShown(keys, shown, 'battery', 1)).toEqual(['gps', 'rc', 'esc', 'battery', 'joystick']);
    expect(movedAmongShown(keys, shown, 'gps', 1)).toBeNull();
    expect(movedAmongShown(keys, shown, 'rc', 1)).toBeNull();
  });

  it('a dropped indicator lands where it was let go', () => {
    const keys = ['battery', 'rc', 'gps', 'links'];
    const centres = new Map([
      ['battery', 20],
      ['gps', 80],
      ['links', 140],
    ]);
    expect(droppedOrder(keys, centres, 'battery', 90)).toEqual(['gps', 'rc', 'battery', 'links']);
    expect(droppedOrder(keys, centres, 'links', -200)).toEqual(['links', 'rc', 'battery', 'gps']);
    expect(droppedOrder(keys, centres, 'gps', 10)).toEqual(keys);
  });
});
