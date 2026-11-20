import { describe, expect, it, vi } from 'vitest';
import { Qgc } from '@/bridge/Qgc';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { GuidedAltitude } from './GuidedAltitude';
import { GuidedOffer } from './GuidedActions';
import { type GuidedReading, GuidedValueKind, altitudeReading, altitudeValue, openedGuidedValue, speedValue, takeoffReading } from './GuidedValueFlow';

const kind = GuidedValueKind({
  offerId: 'takeoff',
  title: 'Takeoff',
  prompt: null,
  commitLabel: 'Take off',
  missingRange: 'no range',
  quickPicks: false,
  read: async () => null,
  commit: async () => null,
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
    const takeoff = takeoffReading({ label: 'Height', unit: 'ft', initial: 10.0, minimum: 10.0, maximum: 400.0, sentence: 'climb', circles: false, targetMeters: 3.0 });
    expect(takeoff?.range).toEqual({ start: 10.0, endInclusive: 400.0 });
    expect(takeoff?.initial).toBe(10.0);
    expect(takeoffReading({ label: 'Height', unit: 'ft', initial: 10.0, minimum: 400.0, maximum: 10.0, sentence: '', circles: false, targetMeters: 3.0 })?.range).toBeNull();

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
    const depth = GuidedOffer({ id: 'changeAltitude', title: 'Change Depth', offer: 'ready', reason: '', prompt: '', destructive: false, carriesValue: true });
    expect([altitudeValue(false, depth).title, altitudeValue(false).title, altitudeValue(true, depth).title]).toEqual(['Change depth', 'Change altitude', 'Pause']);
  });
});

describe('GuidedValueRefusalTest', () => {
  it('a height change or pause the vehicle refuses hands the refusal back instead of dropping it', async () => {
    const view = vi.spyOn(Qgc, 'get').mockResolvedValue({ available: true, sends: true, deltaMeters: -2.0, label: 'Depth', unit: 'm' });
    const refused = vi.spyOn(VehicleCommands, 'changeAltitudeRefusal').mockResolvedValue('Brake is not a mode of this vehicle');
    expect(await altitudeValue(true).commit(12.0)).toBe('Brake is not a mode of this vehicle');
    expect(refused).toHaveBeenCalledWith(-2.0, true);
    view.mockRestore();
    refused.mockRestore();
  });
});

describe('WingTakeoffTest', () => {
  it('a wing take-off says it circles at the height, a copter take-off stays a bare height', () => {
    const read = (circles: boolean) => takeoffReading({ label: 'Height above launch', unit: 'm', initial: 50.0, minimum: 3.0, maximum: 121.0, sentence: 'The aircraft throttles up, climbs to 30.0 m above launch and circles there.', circles, targetMeters: 30.0 })?.sentence;
    expect(read(true)).toBe('The aircraft throttles up, climbs to 30.0 m above launch and circles there.');
    expect(read(false)).toBe('');
  });
});
