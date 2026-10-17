import { describe, expect, it } from 'vitest';
import { ALT_MODE_MIXED, ITEM_CONTEXT, MISSION_CONTEXT, altitudeModePath, altitudeModesPath, altitudeModesView, choosable, offersChoice, refusalFor } from './AltitudeModes';

const SERVED = {
  class: 'AltitudeModes',
  context: 'mission',
  current: 2,
  holdsAltitudeAboveTerrain: true,
  modes: [
    { raw: 1, title: 'Relative To Launch', help: 'Above the launch position.', enabled: true, current: false, reason: '' },
    { raw: 2, title: 'AMSL', help: 'Above mean sea level.', enabled: true, current: true, reason: '' },
    { raw: 4, title: 'Terrain Frame', help: 'Above terrain, held by the vehicle in flight.', enabled: false, current: false, reason: 'This vehicle does not hold an altitude above terrain.' },
    { raw: 0, title: 'Mixed Modes', help: 'Each item sets its own.', enabled: true, current: false, reason: '' },
  ],
  omitted: [],
};

const EMPTY_PLAN = {
  class: 'AltitudeModes',
  context: 'mission',
  current: 0,
  holdsAltitudeAboveTerrain: true,
  modes: [
    { raw: 1, title: 'Relative To Launch', help: '', enabled: false, current: false, reason: 'Add a mission item before choosing how its altitude is measured.' },
    { raw: 2, title: 'AMSL', help: '', enabled: false, current: false, reason: 'Add a mission item before choosing how its altitude is measured.' },
    { raw: 4, title: 'Terrain Frame', help: '', enabled: false, current: false, reason: 'Add a mission item before choosing how its altitude is measured.' },
    { raw: 0, title: 'Mixed Modes', help: '', enabled: true, current: true, reason: '' },
  ],
  omitted: [],
};

const ONE_OPTION = {
  class: 'AltitudeModes',
  context: 'mission',
  current: 2,
  holdsAltitudeAboveTerrain: false,
  modes: [
    { raw: 1, title: 'Relative To Launch', help: '', enabled: false, current: false, reason: 'This vehicle reports no launch position.' },
    { raw: 2, title: 'AMSL', help: 'Above mean sea level.', enabled: true, current: true, reason: '' },
    { raw: 4, title: 'Terrain Frame', help: '', enabled: false, current: false, reason: 'This vehicle does not hold an altitude above terrain.' },
    { raw: 0, title: 'Mixed Modes', help: '', enabled: true, current: false, reason: '' },
  ],
  omitted: [],
};

describe('AltitudeModesTest', () => {
  it('one pickable mode is the one already set, so the control changes nothing', () => {
    expect(
      offersChoice(altitudeModesView(ONE_OPTION)),
      'the single enabled entry is the current mode - opening the control offers the operator what they already have, and the two refusals are printed on the row either way',
    ).toBe(false);
    expect(choosable(altitudeModesView(ONE_OPTION)).filter((it) => it.enabled).length).toBe(1);
  });

  it('a picker holding one option promises a choice it cannot deliver', () => {
    expect(
      offersChoice(altitudeModesView(EMPTY_PLAN)),
      'on an empty plan the core enables only Mixed, which is never offered, so opening the control shows entries that all refuse with the sentence the row already prints',
    ).toBe(false);
  });

  it('a plan with real alternatives keeps the picker live', () => {
    expect(offersChoice(altitudeModesView(SERVED))).toBe(true);
  });

  it("the refused modes are still counted by the core's judgement, never by re-deriving why", () => {
    const view = altitudeModesView(EMPTY_PLAN);
    expect(choosable(view).length).toBe(3);
    expect(refusalFor(view, 1)).toBe('Add a mission item before choosing how its altitude is measured.');
  });

  it('mixed modes is a state the plan can be in, never a mode to pick', () => {
    const picks = choosable(altitudeModesView(SERVED));
    expect(picks.map((it) => it.raw)).toEqual([1, 2, 4]);
    expect(
      picks.every((it) => it.raw !== ALT_MODE_MIXED),
      'offering Mixed would ask the operator to set every item at once',
    ).toBe(true);
  });

  it("a mode the vehicle cannot fly carries the core's reason rather than vanishing", () => {
    expect(refusalFor(altitudeModesView(SERVED), 4)).toBe('This vehicle does not hold an altitude above terrain.');
    expect(refusalFor(altitudeModesView(SERVED), 2), 'a mode that is offered has nothing to explain').toBeNull();
  });

  it('the path carries the context and the current mode, which is what the core keys off', () => {
    expect(altitudeModesPath(MISSION_CONTEXT, 2)).toBe('view.altitudeModes(mission,2)');
    expect(altitudeModesPath(ITEM_CONTEXT, 3)).toBe('view.altitudeModes(item,3)');
    expect(altitudeModePath(3)).toBe('plan.missionController.visualItems.3.altitudeMode');
  });

  it('anything that is not the altitude modes view is refused', () => {
    expect(altitudeModesView({ class: 'Radio' })).toBeNull();
    expect(altitudeModesView(null)).toBeNull();
  });
});
