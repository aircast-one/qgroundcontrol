import { describe, expect, it } from 'vitest';
import { CHECKLIST, deckIds, railLabel, slideProgress } from './FlyDeck';
import { PAUSE } from './MoreActions';

describe('FlyDeckTest', () => {
  it('groundOffersChecklistAndTakeoffAsPrimary', () => {
    expect(deckIds(new Set([CHECKLIST, 'arm', 'takeoff', 'changeSpeed']), false)).toEqual([
      [CHECKLIST, false],
      ['takeoff', true],
    ]);
  });

  it('groundWithoutTakeoffArmsAsPrimary', () => {
    expect(deckIds(new Set(['arm']), false)).toEqual([['arm', true]]);
  });

  it('flyingOffersPauseReturnLandWithReturnPrimary', () => {
    expect(deckIds(new Set(['arm', PAUSE, 'rtl', 'land', 'changeAltitude']), true)).toEqual([
      [PAUSE, false],
      ['rtl', true],
      ['land', false],
    ]);
  });

  it('armedOnTheGroundKeepsTheGroundDeck', () => {
    expect(deckIds(new Set(['arm', 'takeoff']), true)).toEqual([['takeoff', true]]);
  });

  it('sliding across most of the button confirms as surely as holding it', () => {
    expect(slideProgress(300, 1000)).toBeCloseTo(0.5, 6);
    expect(slideProgress(600, 1000)).toBeCloseTo(1, 6);
    expect(slideProgress(900, 1000)).toBeCloseTo(1, 6);
    expect(slideProgress(-200, 1000), 'sliding back the other way never confirms').toBeCloseTo(0, 6);
  });
});

describe('RailLabelTest', () => {
  it('rail labels drop the hold instruction', () => {
    expect(['Slide to land', 'Return', 'Slide to take off'].map(railLabel)).toEqual(['Land', 'Return', 'Take off']);
  });
});
