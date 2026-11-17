import { describe, expect, it } from 'vitest';
import { keptSighting, lastSeenArmed } from '@/map/LostVehicle';
import { VehicleChoice } from '@/map/VehiclePicker';
import { LossFailsafe } from './VehicleLinks';
import { lostFailsafe, lostPlace } from './LinkLost';

const flying = VehicleChoice({ id: 1, name: 'Vehicle 1', state: 'Guided · Flying', link: 'Mock', contactLost: false, active: true, armed: true, latitude: 47.397742, longitude: 8.545594, heading: 90, home: { latitude: 47.3977, longitude: 8.5455 }, telemetry: [['Alt (Rel)', '20.0 m'], ['Ground speed', '3.0 m/s']] });

describe('LinkLost', () => {
  it('remembersTheLastFlyingSightingWithItsAltitudeBatteryAndFailsafe', () => {
    const seen = lastSeenArmed(flying, 64, LossFailsafe('Return home', 5));
    expect(seen).not.toBeNull();
    if (seen === null) return;
    expect(lostPlace(seen)).toBe('Last seen at 47.39774, 8.54559 · 20.0 m up · 64% battery');
    expect(lostFailsafe(seen)).toBe('Failsafe on link loss: Return home.');
    expect(lostFailsafe({ ...seen, failsafe: null })).toBe('Failsafe on link loss: unknown.');
  });

  it('aDisarmedVehicleLeavesNothingToLose', () => {
    expect(lastSeenArmed({ ...flying, armed: false }, 64, null)).toBeNull();
  });

  it('theSightingSurvivesTheVehicleListEmptyingBeforeTheConnectionFlagDrops', () => {
    const seen = lastSeenArmed(flying, 64, null);
    expect(keptSighting(seen, null, null, null)).toBe(seen);
    expect(keptSighting(seen, { ...flying, armed: false }, 64, null)).toBeNull();
    expect(keptSighting(seen, flying, null, null)?.battery).toBe(64);
  });

  it('anUnplacedVehicleSaysItsPositionIsUnknown', () => {
    const seen = lastSeenArmed({ ...flying, latitude: NaN, longitude: NaN, telemetry: [] }, null, null);
    expect(seen === null ? null : lostPlace(seen)).toBe('Last seen position unknown');
  });
});
