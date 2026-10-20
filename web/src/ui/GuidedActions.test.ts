import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { SINGLE_GRIPPER, bayOutput, guidedOffers, multiBay, payloadOf } from './GuidedActions';
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

  it('the payload carries what would stop it dropping and how to fix it', () => {
    const payload = payloadOf(
      json(`{"payload":{"bays":4,"empty":[1],"next":2,"label":"Drop 2","problems":[
        {"code":"staysOpen","text":"The gripper stays open after the first drop.","blocking":true,"fix":"Turn on auto-close"},
        {"code":"noGripper","text":"No gripper.","blocking":true,"fix":""}]}}`),
    );
    expect(payload).toEqual({
      bays: 4,
      empty: [1],
      next: 2,
      label: 'Drop 2',
      outputs: [],
      problems: [
        { code: 'staysOpen', text: 'The gripper stays open after the first drop.', blocking: true, fix: 'Turn on auto-close' },
        { code: 'noGripper', text: 'No gripper.', blocking: true, fix: '' },
      ],
    });
    expect(payloadOf(json('{"payload":{"bays":1,"empty":[],"next":null}}'))).toEqual(SINGLE_GRIPPER);
  });

  it('a servo per bay names each bay by its output, even a single bay', () => {
    const one = payloadOf(json('{"payload":{"bays":1,"empty":[],"next":null,"label":"Drop","outputs":[9],"problems":[]}}'));
    expect([multiBay(one), multiBay(SINGLE_GRIPPER)]).toEqual([true, false]);
    const four = payloadOf(json('{"payload":{"bays":4,"empty":[],"next":1,"label":"Drop 1","outputs":[9,10,11,12],"problems":[]}}'));
    expect([bayOutput(2, four), bayOutput(2, SINGLE_GRIPPER)]).toEqual(['Bay 2 · output 10', '']);
  });

  it('no view is no offers rather than a crash', () => {
    expect(guidedOffers(null)).toEqual(new Map());
    expect(guidedOffers(json('{}'))).toEqual(new Map());
  });
});
