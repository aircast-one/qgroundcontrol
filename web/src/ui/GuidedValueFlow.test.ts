import { describe, expect, it } from 'vitest';
import { GuidedAltitude } from './GuidedAltitude';
import { GuidedOffer } from './GuidedActions';
import { type GuidedReading, GuidedValueKind, altitudeReading, openedGuidedValue, speedValue, takeoffReading } from './GuidedValueFlow';

const kind = GuidedValueKind({
  offerId: 'takeoff',
  title: 'Takeoff',
  prompt: null,
  commitLabel: 'Take off',
  missingRange: 'no range',
  quickPicks: false,
  read: async () => null,
  commit: async () => undefined,
});

describe('GuidedValueFlowTest', () => {
  it('a value opens only with a usable range and a starting value', () => {
    const usable: GuidedReading = { label: 'Height', unit: 'm', range: { start: 1.0, endInclusive: 120.0 }, initial: 10.0, sentence: '', sendable: true };
    expect(openedGuidedValue(kind, usable)).not.toBeNull();
    expect(openedGuidedValue(kind, usable)?.target.value).toBeCloseTo(10.0, 9);
    expect(openedGuidedValue(kind, { ...usable, range: null })).toBeNull();
    expect(openedGuidedValue(kind, { ...usable, initial: null })).toBeNull();
    expect(openedGuidedValue(kind, null)).toBeNull();
  });

  it('each guided reading maps onto the shared shape', () => {
    const takeoff = takeoffReading({ label: 'Height', unit: 'ft', initial: 10.0, minimum: 10.0, maximum: 400.0, sentence: 'climb', targetMeters: 3.0 });
    expect(takeoff?.range).toEqual({ start: 10.0, endInclusive: 400.0 });
    expect(takeoff?.initial).toBe(10.0);
    expect(takeoffReading({ label: 'Height', unit: 'ft', initial: 10.0, minimum: 400.0, maximum: 10.0, sentence: '', targetMeters: 3.0 })?.range).toBeNull();

    const altitude = altitudeReading(GuidedAltitude({ available: true, label: 'Alt', unit: 'm', current: 25.0, minimum: 1.0, maximum: 120.0, sentence: '', deltaMeters: 0.0, sends: false }));
    expect(altitude?.initial).toBe(25.0);
    expect(altitude?.sendable).toBe(false);
  });
});

describe('GuidedValueTitleTest', () => {
  it('a guided panel title reads in sentence case like its neighbours', () => {
    const offer = GuidedOffer({ id: 'changeSpeed', title: 'Change Max Ground Speed', offer: 'ready', reason: '', prompt: '', destructive: false, carriesValue: true });
    expect(speedValue(offer).title).toBe('Change max ground speed');
    expect(speedValue(null).title).toBe('Change max ground speed');
  });
});
