import { describe, expect, it } from 'vitest';
import type { SharedPreferences } from '@/kotlin/Platform';
import { ARROW_STEP_PX, OverlayLayoutState, RESET_ARM_MILLIS, ResetTap, arrowStep, clampedInto, droppedIntoBar, homeRows, intersected, liftedBounds, rowsCovered, droppedOrder, hiddenKeys, jiggleDegrees, movedAmongShown, movedKey, onScreenCorrection, orderedKeys, orientedKey, resetPillText, resetTap, shownCorrection, storedIndicatorOrder, storedOffsets, visibleArea, withHidden } from './OverlayLayout';

const rect = (left: number, top: number, right: number, bottom: number) => ({ left, top, right, bottom });

describe('OverlayLayoutTest', () => {
  it('a small widget jiggles the full angle and a wide one turns less, so no corner swings more than a few pixels', () => {
    expect(jiggleDegrees({ width: 48, height: 48 })).toBe(1.2);
    const wide = { width: 1100, height: 200 };
    expect(((Math.hypot(wide.width, wide.height) / 2) * jiggleDegrees(wide) * Math.PI) / 180).toBeCloseTo(4);
  });

  it('hidden widgets are read from the stored rig keys only', () => {
    const stored = { 'OverlayRigHidden-orbit': true, 'OverlayRigHidden-traffic': false, other: true };
    expect(hiddenKeys(stored)).toEqual(new Set(['minimap', 'orbit']));
  });

  it('the minimap starts hidden and stays shown once the pilot shows it', () => {
    expect(hiddenKeys({})).toEqual(new Set(['minimap']));
    expect(hiddenKeys({ 'OverlayRigHidden-minimap': false })).toEqual(new Set());
  });

  it('hiding and showing toggles one key', () => {
    expect(withHidden(new Set(['a']), 'b', true)).toEqual(new Set(['a', 'b']));
    expect(withHidden(new Set(['a']), 'a', false)).toEqual(new Set());
  });

  it('reset layout arms on the first tap and resets on the second, like resetPill', () => {
    expect(resetTap(false)).toEqual(ResetTap({ reset: false, armed: true }));
    expect(resetTap(true)).toEqual(ResetTap({ reset: true, armed: false }));
    expect(resetPillText(false)).toBe('Reset positions and hidden items');
    expect(resetPillText(true)).toBe('Tap again to reset');
    expect(RESET_ARM_MILLIS).toBe(4000);
  });

  it('a dragged widget stops at the screen edges', () => {
    const screen = rect(0, 0, 1000, 2000);
    expect(clampedInto(rect(100, 40, 200, 140), screen, 30, -80)).toEqual([30, -40]);
    expect(clampedInto(rect(100, 40, 200, 140), screen, -500, 5000)).toEqual([-100, 1860]);
    expect(clampedInto(rect(100, 40, 200, 140), screen, 900, 0)).toEqual([800, 0]);
  });

  it('a held widget stops where its lifted, larger self still fits, so the edge never cuts it', () => {
    const screen = rect(0, 0, 1000, 800);
    expect(liftedBounds(rect(100, 100, 240, 240))).toEqual(rect(93, 93, 247, 247));
    expect(liftedBounds(rect(100, 100, 124, 124))).toEqual(rect(96, 96, 128, 128));
    expect(clampedInto(liftedBounds(rect(100, 100, 240, 240)), screen, 5000, 5000)).toEqual([753, 553]);
  });

  it('stored offsets are read back per widget and malformed ones are dropped', () => {
    const stored = { 'OverlayRigOffset-instrumentPanel': '12.5,-30.0', 'OverlayRigOffset-orbit': 'bad', 'OverlayRigHidden-traffic': true };
    expect(storedOffsets(stored)).toEqual(new Map([['instrumentPanel', [12.5, -30]]]));
  });

  it('the stored layout yields hidden widgets, offsets and the indicator order', () => {
    const stored = { 'OverlayRigOffset-instrumentPanel': '10.0,20.0', 'OverlayRigHidden-orbit': true, FlyViewIndicatorOrder: 'gps,battery' };
    expect(storedOffsets(stored)).toEqual(new Map([['instrumentPanel', [10, 20]]]));
    expect(hiddenKeys(stored)).toEqual(new Set(['minimap', 'orbit']));
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

const memoryPrefs = (): SharedPreferences & { names: () => string[] } => {
  const stored = new Map<string, string>();
  return {
    getString: (name, fallback) => stored.get(name) ?? fallback,
    getBoolean: (name, fallback) => (stored.has(name) ? stored.get(name) === 'true' : fallback),
    getInt: (_, fallback) => fallback,
    getFloat: (_, fallback) => fallback,
    getStringSet: (_, fallback) => fallback,
    contains: (name) => stored.has(name),
    putString: (name, value) => (value === null ? stored.delete(name) : stored.set(name, value)) && undefined,
    putBoolean: (name, value) => stored.set(name, String(value)) && undefined,
    putInt: (name, value) => stored.set(name, String(value)) && undefined,
    putFloat: (name, value) => stored.set(name, String(value)) && undefined,
    putStringSet: () => undefined,
    remove: (name) => stored.delete(name) && undefined,
    getAll: () => Object.fromEntries(stored),
    names: () => [...stored.keys()].toSorted(),
  };
};

describe('ReadingPlacementTest', () => {
  const pitch = 26;
  const height = 24;

  it('readings nobody moved stack up from the bottom in reverse, so the first chosen sits on top as before', () => {
    expect(homeRows(['distance', 'altitude'], new Set())).toEqual(new Map([['altitude', 0], ['distance', 1]]));
  });

  it('an added reading takes the bottom row and the readings above it move up a row, never onto each other', () => {
    expect(homeRows(['distance', 'altitude', 'airspeed'], new Set())).toEqual(new Map([['airspeed', 0], ['altitude', 1], ['distance', 2]]));
  });

  it('a reading moved away keeps its row, so the readings nobody moved stay where they stand', () => {
    expect(homeRows(['distance', 'altitude'], new Set()).get('distance')).toBe(1);
  });

  it('a removed reading leaves no gap among the ones nobody moved', () => {
    expect(homeRows(['altitude'], new Set())).toEqual(new Map([['altitude', 0]]));
    expect(homeRows([], new Set())).toEqual(new Map());
  });

  it('a row a moved reading covers is skipped, so an added reading does not land on it', () => {
    expect(homeRows(['distance', 'airspeed'], new Set([0]))).toEqual(new Map([['airspeed', 1], ['distance', 2]]));
    expect(homeRows(['distance', 'airspeed'], new Set([1]))).toEqual(new Map([['airspeed', 0], ['distance', 2]]));
  });

  it('a moved reading covers the rows it overlaps in its own column and none when it is moved away from it', () => {
    expect(rowsCovered([0, 0], pitch, height)).toEqual([0]);
    expect(rowsCovered([10, -pitch], pitch, height)).toEqual([1]);
    expect(rowsCovered([0, -13], pitch, height)).toEqual([0, 1]);
    expect(rowsCovered([0, 100], pitch, height)).toEqual([]);
    expect(rowsCovered([300, -pitch], pitch, height)).toEqual([]);
  });
});

describe('LayoutStateTest', () => {
  it('forgetting a reading drops its place in both orientations and its hidden mark, and keeps everyone else', () => {
    const prefs = memoryPrefs();
    const layout = new OverlayLayoutState(prefs);
    layout.nudge('reading:vehicle/distanceToHome', 4, 5);
    layout.saveOffset('reading:vehicle/distanceToHome');
    layout.nudge('reading:vehicle/distanceToHome@landscape', 40, -26, [0, -26]);
    layout.saveOffset('reading:vehicle/distanceToHome@landscape');
    layout.nudge('orbit', 1, 1);
    layout.saveOffset('orbit');
    layout.setHidden('reading:vehicle/distanceToHome', true);
    expect(layout.offsets.value.get('reading:vehicle/distanceToHome@landscape'), 'the first move starts from its home row').toEqual([40, -52]);
    expect(prefs.names()).toEqual(['OverlayRigHidden-reading:vehicle/distanceToHome', 'OverlayRigOffset-orbit', 'OverlayRigOffset-reading:vehicle/distanceToHome', 'OverlayRigOffset-reading:vehicle/distanceToHome@landscape']);
    layout.forget(['reading:vehicle/distanceToHome']);
    expect(prefs.names()).toEqual(['OverlayRigOffset-orbit']);
    expect([...layout.offsets.value.keys()]).toEqual(['orbit']);
    expect(layout.hidden.value.has('reading:vehicle/distanceToHome')).toBe(false);
  });

  it('reset forgets every reading placement with the rest of the layout', () => {
    const layout = new OverlayLayoutState(memoryPrefs());
    layout.nudge('reading:vehicle/groundSpeed@landscape', 10, 10);
    layout.setHidden('reading:vehicle/climbRate', true);
    layout.reset();
    expect(layout.offsets.value).toEqual(new Map());
    expect(layout.hidden.value).toEqual(new Set(['minimap']));
  });

  it('a reading docked in the top bar leaves it when the reading is forgotten, and a reset empties the bar of readings and brings floated items home', () => {
    const layout = new OverlayLayoutState(memoryPrefs());
    layout.saveDocked(['reading:vehicle/distanceToHome', 'reading:wind/speed']);
    layout.saveFloated(['flightTime']);
    layout.forget(['reading:vehicle/distanceToHome']);
    expect(layout.docked.value).toEqual(['reading:wind/speed']);
    expect(layout.floated.value).toEqual(['flightTime']);
    layout.reset();
    expect([layout.docked.value, layout.floated.value]).toEqual([[], []]);
  });

  it('an item dropped on the top bar goes before the first shown item to its right, and last when there is none', () => {
    const cells = [
      { key: 'battery', centre: 100, shown: true },
      { key: 'flightTime', centre: 200, shown: true },
      { key: 'gps', centre: 300, shown: false },
      { key: 'health', centre: 400, shown: true },
    ];
    expect(droppedIntoBar(cells, 'reading:wind/speed', 150)).toEqual(['battery', 'reading:wind/speed', 'flightTime', 'gps', 'health']);
    expect(droppedIntoBar(cells, 'reading:wind/speed', 350), 'an item with nothing to show is not a place to drop before').toEqual(['battery', 'flightTime', 'gps', 'reading:wind/speed', 'health']);
    expect(droppedIntoBar(cells, 'reading:wind/speed', 999)).toEqual(['battery', 'flightTime', 'gps', 'health', 'reading:wind/speed']);
    expect(droppedIntoBar(cells, 'battery', 350), 'an item already listed moves').toEqual(['flightTime', 'gps', 'battery', 'health']);
  });

  it('the arrow keys step a widget a few pixels, or four times as far with shift, and other keys do nothing', () => {
    expect(arrowStep('ArrowLeft', false)).toEqual([-ARROW_STEP_PX, 0]);
    expect(arrowStep('ArrowDown', true)).toEqual([0, ARROW_STEP_PX * 4]);
    expect(arrowStep('Enter', false)).toBeNull();
  });

  it('a widget held inside the picture is clamped to where the picture and the free screen overlap', () => {
    expect(intersected(rect(0, 48, 1280, 800), rect(72, 0, 1280, 704))).toEqual(rect(72, 48, 1280, 704));
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
    const centres = new Map<string, readonly [number, number]>([
      ['battery', [20, 10]],
      ['gps', [80, 10]],
      ['links', [140, 10]],
    ]);
    expect(droppedOrder(keys, centres, 'battery', 90, 0)).toEqual(['gps', 'rc', 'battery', 'links']);
    expect(droppedOrder(keys, centres, 'links', -200, 0)).toEqual(['links', 'rc', 'battery', 'gps']);
    expect(droppedOrder(keys, centres, 'gps', 10, 30)).toEqual(keys);
  });

  it('on a strip wrapped onto two lines, an indicator dropped on the second line goes after everything on the first', () => {
    const keys = ['battery', 'gps', 'links', 'rc'];
    const centres = new Map<string, readonly [number, number]>([
      ['battery', [20, 10]],
      ['gps', [200, 10]],
      ['links', [20, 50]],
      ['rc', [80, 50]],
    ]);
    expect(droppedOrder(keys, centres, 'battery', 30, 40)).toEqual(['gps', 'links', 'battery', 'rc']);
    expect(droppedOrder(keys, centres, 'rc', 100, -40)).toEqual(['battery', 'rc', 'gps', 'links']);
  });
});
