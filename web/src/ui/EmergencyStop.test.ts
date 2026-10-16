import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { EMERGENCY_STOP, armedStopOffer, emergencyStopAction, emergencyStopOffer } from './EmergencyStop';
import { type GuidedOffer, guidedOffers } from './GuidedActions';
import { SHEET_ACTIONS, moreActions } from './MoreActions';

const json = (text: string): Json => parseJson(text) ?? {};

const offers = (...entries: readonly (readonly [string, string])[]): ReadonlyMap<string, GuidedOffer> =>
  guidedOffers(
    json(
      `{"actions":[${entries
        .map(([id, offer]) => `{"id":"${id}","title":"${id}","offer":"${offer}","reason":"","prompt":"","destructive":false,"carriesValue":false}`)
        .join(',')}]}`,
    ),
  );

describe('EmergencyStopTest', () => {
  it('hold to disarm while flying becomes the emergency stop', () => {
    const flying = offers([EMERGENCY_STOP, 'ready'], ['disarm', 'hidden']);
    expect(armedStopOffer(flying, true), 'GuidedActionsController routes a flying disarm to showEmergenyStop').not.toBeNull();
    expect(armedStopOffer(flying, false)).toBeNull();
    expect(armedStopOffer(offers([EMERGENCY_STOP, 'hidden']), true)).toBeNull();
  });

  it('the emergency stop is never in the overflow sheet', () => {
    expect(
      SHEET_ACTIONS.has(EMERGENCY_STOP),
      'cutting the motors is the one action that must not be two taps behind a menu; it is pinned, so the sheet must never also offer it',
    ).toBe(false);
  });

  it('a ready emergency stop appears in no sheet listing', () => {
    const extras = moreActions(offers([EMERGENCY_STOP, 'ready'], ['grab', 'ready']));

    expect(extras.every((it) => it.id !== EMERGENCY_STOP)).toBe(true);
    expect(extras.map((it) => it.id)).toEqual(['grab']);
  });

  it('the pinned button takes the offer the core serves', () => {
    expect(emergencyStopOffer(offers([EMERGENCY_STOP, 'ready']))).not.toBeNull();
    expect(
      emergencyStopOffer(offers([EMERGENCY_STOP, 'blocked'])),
      'a blocked stop is still drawn, with its reason - hiding it would remove the control at the moment an operator reaches for it',
    ).not.toBeNull();
  });

  it('a hidden emergency stop draws nothing', () => {
    expect(emergencyStopOffer(offers([EMERGENCY_STOP, 'hidden']))).toBeNull();
    expect(emergencyStopOffer(offers(['grab', 'ready']))).toBeNull();
  });

  it('the confirmation is always destructive, whatever the core says', () => {
    const offer = offers([EMERGENCY_STOP, 'ready']).get(EMERGENCY_STOP);
    if (offer === undefined) throw new Error('the fixture serves an emergency stop');

    expect(offer.destructive, 'the fixture deliberately says false').toBe(false);
    expect(
      emergencyStopAction(offer).destructive,
      'this one cuts the motors in flight; the confirmation must read as destructive even if the served flag does not say so',
    ).toBe(true);
  });
});
