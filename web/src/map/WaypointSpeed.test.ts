import { describe, expect, it } from 'vitest';
import { waypointHold, waypointSpeed } from './WaypointSpeed';

describe('WaypointSpeedTest', () => {
  it("the speed row reads the item's speed section and skips a vehicle that has none", () => {
    const served = { speedSection: { available: true, specified: true, value: 8.0, units: 'm/s', path: 'p', specifyPath: 's' } };
    expect(waypointSpeed(served)).toEqual({ specified: true, value: 8.0, units: 'm/s', path: 'p', specifyPath: 's' });
    expect(waypointSpeed({ speedSection: { available: false } })).toBeNull();
  });

  it("a waypoint's hold is read in seconds, and an item without one shows no row", () => {
    expect(waypointHold({ hold: { value: 5.0, units: 's', path: 'p' } })).toEqual({ seconds: 5.0, units: 's', path: 'p' });
    expect(waypointHold({ hold: null })).toBeNull();
  });
});
