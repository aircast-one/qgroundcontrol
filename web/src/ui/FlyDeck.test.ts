import { describe, expect, it } from 'vitest';
import { CHECKLIST, type DeckIds, bottomDeck, deckIds, deckLabel, railDeck, railLabel, slideProgress } from './FlyDeck';
import { PAUSE, RELEASE } from './MoreActions';

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

  it('a drop sits on the deck beside the flight actions in flight and never takes the primary slot', () => {
    expect(deckIds(new Set(['arm', PAUSE, 'rtl', 'land', RELEASE]), true)).toEqual([
      [PAUSE, false],
      ['rtl', true],
      ['land', false],
      [RELEASE, false],
    ]);
  });

  it('on the ground the deck is for taking off; the drop stays in the payload sheet', () => {
    expect(deckIds(new Set(['arm', 'takeoff', RELEASE]), false)).toEqual([['takeoff', true]]);
  });

  it('sliding across most of the button confirms as surely as holding it', () => {
    expect(slideProgress(300, 1000)).toBeCloseTo(0.5, 6);
    expect(slideProgress(600, 1000)).toBeCloseTo(1, 6);
    expect(slideProgress(900, 1000)).toBeCloseTo(1, 6);
    expect(slideProgress(-200, 1000), 'sliding back the other way never confirms').toBeCloseTo(0, 6);
  });
});

describe('RailLabelTest', () => {
  it('a crowded deck drops the slide instruction so five buttons fit a phone', () => {
    expect([deckLabel('Slide to land', 4), deckLabel('Slide to land', 5), deckLabel('Slide to drop 2', 5)]).toEqual(['Slide to land', 'Land', 'Drop 2']);
  });

  it('rail labels drop the hold instruction', () => {
    expect(['Slide to land', 'Return', 'Slide to take off'].map(railLabel)).toEqual(['Land', 'Return', 'Take off']);
  });
});

describe('DeckFitTest', () => {
  const flying = deckIds(new Set(['arm', PAUSE, 'rtl', 'land', RELEASE]), true);
  const ids = (deck: DeckIds) => deck.map(([id]) => id);

  it('on a phone every flight action stays on the deck and the camera moves above it', () => {
    expect([358, 328].map((width) => bottomDeck(flying, width, 112))).toEqual([
      { deck: flying, centreInline: false },
      { deck: flying, centreInline: false },
    ]);
  });

  it('a deck with room keeps the camera between the actions', () => {
    expect(bottomDeck(flying, 700, 112)).toEqual({ deck: flying, centreInline: true });
    expect(bottomDeck(flying, 328, 0)).toEqual({ deck: flying, centreInline: true });
  });

  it('when the actions run out of room the least critical move into More first', () => {
    expect([288, 222, 156, 90, 40].map((width) => ids(bottomDeck(flying, width, 112).deck))).toEqual([['rtl', 'land', RELEASE], ['rtl', 'land'], ['rtl'], [], []]);
  });

  it('the ground deck gives up the checklist before taking off', () => {
    expect(ids(bottomDeck(deckIds(new Set([CHECKLIST, 'arm', 'takeoff']), false), 140, 0).deck)).toEqual(['takeoff']);
  });

  it('a deck not measured yet shows every action', () => {
    expect(bottomDeck(flying, 0, 112)).toEqual({ deck: flying, centreInline: true });
    expect(railDeck(flying, 0)).toEqual({ deck: flying, labelled: true });
  });

  it('a sideways phone drops the rail labels before it drops an action', () => {
    const noDrop = flying.filter(([id]) => id !== RELEASE);
    expect(railDeck(flying, 286)).toEqual({ deck: flying, labelled: false });
    expect(railDeck(noDrop, 286)).toEqual({ deck: noDrop, labelled: true });
    expect(ids(railDeck(flying, 271).deck)).toEqual(['rtl', 'land', RELEASE]);
  });
});
