import { describe, expect, it } from 'vitest';
import type { JsonArray } from '@/bridge/Json';
import { DRAWN_KINDS, undrawnItemNames, undrawnItemsWarning } from './PlanFileRules';

const items = (...pairs: (readonly [string, string])[]): JsonArray => pairs.map(([kind, name]) => ({ kind, name }));

describe('UndrawnItemsTest', () => {
  it('the two kinds the map cannot draw are the two it warns about', () => {
    expect(
      undrawnItemNames(items(['complex', 'Orbit'], ['unreadable', 'Unknown: 1234'])),
      "missionitems.rs kind() returns complex when by_class does not recognise the item's class, and unreadable when QGC calls it neither simple nor complex - those are the only two outside DRAWN_KINDS, and they are exactly what has no shape to draw",
    ).toEqual(['Orbit', 'Unknown: 1234']);
  });

  it('Mission Start is drawn, so no plan is warned about its own first row', () => {
    expect(
      undrawnItemNames(items(['settings', 'Mission Start'], ['takeoff', 'Takeoff'])),
      'every plan carries a settings item - live capture shows kind settings, name Mission Start - so leaving it out of DRAWN_KINDS would put a warning on every plan ever opened',
    ).toEqual([]);
  });

  it('every drawable kind stays out of the warning', () => {
    [...DRAWN_KINDS].map((kind) => expect(undrawnItemNames(items([kind, 'Something'])), kind).toEqual([]));
  });

  it('one name is not repeated when several items share it', () => {
    expect(undrawnItemNames(items(['complex', 'Orbit'], ['complex', 'Orbit']))).toEqual(['Orbit']);
  });

  it('an undrawn item with no name is left out of the sentence entirely', () => {
    expect(
      undrawnItemNames(items(['complex', ''])),
      'deliberate: an out-of-date AAR yields items with no class and so no name, and naming nothing is better than naming it wrongly. The cost is that a plan whose undrawn items are ALL unnamed gets no warning at all, which is recorded rather than fixed',
    ).toEqual([]);
    expect(undrawnItemsWarning(undrawnItemNames(items(['complex', ''])))).toBeNull();
  });

  it('the sentence says the items are still flown, not merely undrawn', () => {
    expect(
      undrawnItemsWarning(['Orbit']),
      "a pilot who reads only 'the map cannot draw X' may take it as cosmetic; the item still counts in distance and duration and still uploads",
    ).toBe('The map cannot draw Orbit. Those items are still in the plan and will still be flown.');
    expect(undrawnItemsWarning([])).toBeNull();
  });
});
