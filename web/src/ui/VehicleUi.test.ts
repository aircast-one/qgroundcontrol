import { describe, expect, it, vi } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { multiVehiclePanelEnabled, osdModeText, osdStatusNote } from './ActiveVehicle';
import { CHECKLIST, HOLD_TO_TAKE_OFF, TAKE_OFF } from './FlyDeck';
import { Readiness } from './FlyState';
import { GuidedOffer, type Payload, SINGLE_GRIPPER, guidedOffers } from './GuidedActions';
import type { GuidedValueKind } from './GuidedValueFlow';
import { PAUSE } from './MoreActions';
import {
  FlightDeckContext,
  type GuidedAction,
  Instrument,
  checklistStateValue,
  flightDeckEntries,
  instruments,
  offerWithdrawn,
  operatorDistance,
  osdIsSpeed,
  osdLabel,
  rowWidth,
} from './VehicleUi';

const json = (text: string): Json => parseJson(text) ?? {};

describe('FlightDeckEntriesTest', () => {
  const offer = (id: string, state = 'ready', carriesValue = true) =>
    GuidedOffer({ id, title: '', offer: state, reason: '', prompt: '', destructive: false, carriesValue });

  const deck = (
    offers: readonly GuidedOffer[],
    { armed = false, openChecklist = null, openValue = () => undefined, readiness = null, payload = SINGLE_GRIPPER, confirm = () => undefined }: { armed?: boolean; openChecklist?: (() => void) | null; openValue?: (kind: GuidedValueKind) => void; readiness?: Readiness | null; payload?: Payload; confirm?: (action: GuidedAction) => void } = {},
  ) =>
    FlightDeckContext({
      offers: new Map(offers.map((it) => [it.id, it] as const)),
      armed,
      confirm,
      openValue,
      report: () => undefined,
      openChecklist,
      readiness,
      payload,
    });

  it('a basket drops its next loaded bay from the deck and says which, and when the core counts it empty', () => {
    const confirmed = vi.fn((action: GuidedAction) => action);
    const drop = (empty: readonly number[], next: number, label: string, title: string) =>
      flightDeckEntries(deck([{ ...offer('release'), title }], { confirm: confirmed, payload: { bays: 3, empty, next, label, outputs: [], problems: [] } })).find((it) => it.id === 'release');
    expect([drop([], 1, 'Drop 1', 'Drop bay 1')?.label, drop([1], 2, 'Drop 2', 'Drop bay 2')?.label]).toEqual(['Slide to drop 1', 'Slide to drop 2']);
    const emptied = drop([1, 2, 3], 1, 'Drop 1 · empty', 'Drop bay 1');
    expect([emptied?.label, emptied?.enabled]).toEqual(['Slide to drop 1 · empty', true]);
    emptied?.onClick();
    expect(confirmed.mock.results.at(-1)?.value.name, 'the confirm sheet names the bay as the Payload sheet does').toBe('Drop bay 1');
  });

  it('a drop confirmed from the deck waits for the core to answer, so a refused drop is never reported as sent', async () => {
    const refusal = 'The gripper is still open from the last drop. It closes in 1 s.';
    const asked = vi.spyOn(VehicleCommands, 'gripperRefusal').mockResolvedValue(refusal);
    const confirmed = vi.fn((action: GuidedAction) => action);
    flightDeckEntries(deck([{ ...offer('release'), title: 'Drop bay 4' }], { confirm: confirmed, payload: { bays: 4, empty: [1, 2, 3], next: 4, label: 'Drop 4', outputs: [], problems: [] } }))
      .find((it) => it.id === 'release')
      ?.onClick();
    const action = confirmed.mock.results[0]?.value as GuidedAction;
    action.run();
    expect(asked).not.toHaveBeenCalled();
    expect(await action.answer?.()).toBe(refusal);
    expect(asked).toHaveBeenCalledWith(0, 4);
    asked.mockRestore();
  });

  it('a vehicle with a gripper gets a drop on the deck that lands on a hold, like land', () => {
    const entries = flightDeckEntries(deck([offer('arm'), { ...offer('release'), title: 'Drop' }]));
    const drop = entries.find((it) => it.id === 'release');
    expect(entries.map((it) => it.id)).toEqual(['arm', 'release']);
    expect(drop?.label).toBe('Slide to drop');
    expect(drop?.onHold).not.toBeNull();
    expect(drop?.icon).toBe('ic_drop');
    expect(flightDeckEntries(deck([offer('arm'), offer('release', 'hidden')])).map((it) => it.id)).toEqual(['arm']);
  });

  it('the deck shows the offers the vehicle shows, and the checklist only on the ground when offered', () => {
    const offers = [offer('arm'), offer('takeoff'), offer('rtl', 'hidden'), offer('land', 'disabled')];
    expect(flightDeckEntries(deck(offers)).map((it) => it.id)).toEqual(['arm', 'takeoff', 'land']);
    expect(flightDeckEntries(deck(offers, { openChecklist: () => undefined })).map((it) => it.id)).toEqual(['arm', 'takeoff', 'land', CHECKLIST]);
    expect(flightDeckEntries(deck(offers, { armed: true, openChecklist: () => undefined })).map((it) => it.id)).toEqual(['takeoff', 'land']);
  });

  it('value actions open the shared guided value flow', () => {
    const openValue = vi.fn((kind: GuidedValueKind) => kind.offerId);
    const entries = flightDeckEntries(deck([offer('takeoff'), offer('changeSpeed'), offer('changeAltitude'), offer(PAUSE)], { openValue }));
    const ids = ['takeoff', 'changeSpeed', 'changeAltitude', PAUSE];
    ids.map((id) => entries.find((it) => it.id === id)?.onClick());
    expect(openValue.mock.results.map((it) => it.value)).toEqual(ids);
  });

  it('a vehicle that is not fully ready cannot be held straight into the air, the tap opens the warning first', () => {
    const ready = flightDeckEntries(deck([offer('takeoff')])).find((it) => it.id === 'takeoff');
    expect([ready?.label, (ready?.onHold ?? null) !== null]).toEqual([HOLD_TO_TAKE_OFF, true]);
    const warned = flightDeckEntries(deck([offer('takeoff')], { readiness: Readiness({ text: 'GPS turned off.', blocks: false }) })).find((it) => it.id === 'takeoff');
    expect([warned?.label, (warned?.onHold ?? null) !== null]).toEqual([TAKE_OFF, false]);
  });
});

describe('OsdLabelTest', () => {
  it("the flying readings shorten to DJI's letters and anything else keeps its name", () => {
    expect(['Distance to home', 'Alt (Rel)', 'Ground Speed', 'Climb Rate'].map(osdLabel)).toEqual(['D', 'H', 'H.S', 'V.S']);
    expect(osdLabel('Flight time')).toBe('FLIGHT TIME');
    expect(osdLabel('From you')).toBe('D.OP');
    expect(['distanceToHome', 'altitudeRelative', 'groundSpeed', 'climbRate'].map(osdLabel)).toEqual(['D', 'H', 'H.S', 'V.S']);
  });
});

describe('DjiChromeTest', () => {
  it('the status title splits into plain mode text and a note for the pill', () => {
    expect(osdModeText('Manual · Not fully ready')).toBe('Manual');
    expect(osdStatusNote('Manual · Not fully ready')).toBe('Not fully ready');
    expect(osdStatusNote('Manual')).toBeNull();
  });

  it('speeds go on the small top row and the rest on the large one', () => {
    expect(['Ground Speed', 'Climb Rate', 'Alt (Rel)', 'Distance to home'].map(osdIsSpeed)).toEqual([true, true, false, false]);
  });
});

describe('InstrumentsTest', () => {
  it('a reading joins its units and a unitless one does not', () => {
    const shown = instruments(
      json(`{"items":[
        {"label":"Alt (Rel)","value":"25.0","units":"m","missing":false},
        {"label":"Heading","value":"356","units":"","missing":false}
      ]}`),
    );

    expect(shown.map((it) => it.reading)).toEqual(['25.0 m', '356']);
    expect(shown.map((it) => it.label)).toEqual(['Alt (Rel)', 'Heading']);
  });

  it('a missing instrument is left out rather than shown blank', () => {
    const shown = instruments(
      json(`{"items":[
        {"label":"Alt (Rel)","value":"25.0","units":"m","missing":false},
        {"label":"Distance to Home","value":"","units":"","missing":true}
      ]}`),
    );

    expect(shown.map((it) => it.label)).toEqual(['Alt (Rel)']);
  });

  it('no view at all is an empty row, not a crash', () => {
    expect(instruments(null)).toEqual([]);
    expect(instruments(json('{}'))).toEqual([]);
  });
});

describe('InstrumentMissingTest', () => {
  const item = (reason: string) =>
    instruments(
      json(`{"items":[{"label":"Alt (Rel)","value":"25.0","units":"m","missing":false},
        {"label":"Distance to Home","value":"","units":"","missing":true,
         "missingReason":${reason}}]}`),
    );

  it('a fact the vehicle has not reported yet keeps its place and waits', () => {
    const shown = item('"notReported"');

    expect(shown.map((it) => it.label)).toEqual(['Alt (Rel)', 'Distance to Home']);
    expect(shown[1]?.reading).toBe('—');
  });

  it('a fact this vehicle does not have is left out, because it will never fill in', () => {
    expect(item('"noSuchFact"').map((it) => it.label)).toEqual(['Alt (Rel)']);
  });

  it('a missing item with no reason is left out rather than waiting forever', () => {
    expect(item('null').map((it) => it.label)).toEqual(['Alt (Rel)']);
  });
});

describe('OperatorDistanceTest', () => {
  it('the served text is shown as its own reading, split into number and unit like every other reading', () => {
    const view = json('{"distanceToVehicleText":"316.8 m"}');

    expect(operatorDistance(view)).toEqual([Instrument({ label: 'From you', reading: '316.8 m', value: '316.8', units: 'm' })]);
    expect(operatorDistance(json('{"distanceToVehicleText":"316.8"}'))).toEqual([Instrument({ label: 'From you', reading: '316.8', value: '316.8', units: '' })]);
  });

  it('nothing is shown when the core withholds the distance', () => {
    expect(operatorDistance(json('{"distanceToVehicleText":null}'))).toEqual([]);
    expect(operatorDistance(json('{}'))).toEqual([]);
    expect(operatorDistance(null)).toEqual([]);
  });
});

describe('RowWidthTest', () => {
  it('four or fewer readings stay on one line', () => {
    expect(rowWidth(4)).toBe(4);
    expect(rowWidth(3)).toBe(3);
  });

  it('more than four are split evenly rather than leaving an orphan', () => {
    expect(rowWidth(5)).toBe(3);
    expect(rowWidth(6)).toBe(3);
    expect(rowWidth(7)).toBe(4);
  });
});

describe('GuidedOffersTest', () => {
  it("the rtl offer carries the core's Smart RTL option", () => {
    const offers = guidedOffers(json('{"actions":[{"id":"rtl","offer":"ready","option":"Smart RTL"},{"id":"land","offer":"ready","option":""}]}'));
    expect(offers.get('rtl')?.option).toBe('Smart RTL');
    expect(offers.get('land')?.option).toBe('');
  });

  it('the multi-vehicle panel is on unless the setting turns it off', () => {
    expect([null, json('{"value":true}'), json('{"value":false}')].map(multiVehiclePanelEnabled)).toEqual([true, true, false]);
  });

  it("a confirm closes once its action stops being offered, like GuidedActionConfirm's hideTrigger", () => {
    const offers = guidedOffers(json('{"actions":[{"id":"land","offer":"ready"},{"id":"takeoff","offer":"hidden"}]}'));
    expect([offerWithdrawn('land', offers), offerWithdrawn('takeoff', offers), offerWithdrawn('orbit', offers), offerWithdrawn(null, offers)]).toEqual([false, true, true, false]);
  });

  it("the checklist result is written as Vehicle's CheckListPassed or CheckListFailed", () => {
    expect([checklistStateValue(true), checklistStateValue(false)]).toEqual([1, 2]);
  });
});
