import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { guidedOffers } from './GuidedActions';
import { GuidedAction, needsConfirm } from './VehicleUi';

const json = (text: string): Json => parseJson(text) ?? {};

const served = `
  {"connected":true,"forwardFlight":false,"missionActive":false,"actions":[
    {"id":"arm","title":"Arm","offer":"blocked","reason":"Vehicle is not ready to arm.",
     "prompt":"Arming spins the propellers.","destructive":true,"carriesValue":false},
    {"id":"takeoff","title":"Takeoff","offer":"ready","reason":"",
     "prompt":"The vehicle will climb.","destructive":false,"carriesValue":true},
    {"id":"land","title":"Land","offer":"hidden","reason":"",
     "prompt":"","destructive":false,"carriesValue":false}]}
`;

describe('GuidedActionsTest', () => {
  it('an offer carries its state, reason and prompt', () => {
    const offers = guidedOffers(json(served));

    expect(offers.get('arm')?.ready).toBe(false);
    expect(offers.get('takeoff')?.ready).toBe(true);
    expect(offers.get('land')?.shown).toBe(false);
    expect(offers.get('arm')?.reason).toBe('Vehicle is not ready to arm.');
    expect(offers.get('takeoff')?.prompt).toBe('The vehicle will climb.');
    expect(offers.get('arm')?.destructive).toBe(true);
  });

  it('a disarm the core marks as unconfirmed goes out on the tap, everything else still asks', () => {
    const offers = guidedOffers(
      json(`{"actions":[
        {"id":"disarm","title":"Disarm","offer":"ready","prompt":"Disarm the vehicle","destructive":false,"confirm":false,"carriesValue":false},
        {"id":"land","title":"Land","offer":"ready","prompt":"Land","destructive":false,"confirm":true,"carriesValue":false},
        {"id":"rtl","title":"Return","offer":"ready","prompt":"Return","destructive":false,"carriesValue":false}]}`),
    );
    const action = (id: string) => GuidedAction({ name: id, confirm: '', destructive: false, offerId: id, run: () => undefined });

    expect(needsConfirm(action('disarm'), offers)).toBe(false);
    expect(needsConfirm(action('land'), offers)).toBe(true);
    expect(needsConfirm(action('rtl'), offers), 'an offer that does not say asks first').toBe(true);
    expect(needsConfirm(action('gone'), offers), 'an action with no offer behind it asks first').toBe(true);
  });

  it('no view is no offers rather than a crash', () => {
    expect(guidedOffers(null)).toEqual(new Map());
    expect(guidedOffers(json('{}'))).toEqual(new Map());
  });
});
