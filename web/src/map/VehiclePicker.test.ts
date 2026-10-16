import { describe, expect, it } from 'vitest';
import { vehicleChoices, vehicleFlightModePath } from './VehiclePicker';

describe('VehicleFlightModeTest', () => {
  it("each row carries its own settable modes and writes its own vehicle, like MultiVehicleList's FlightModeMenu", () => {
    const choices = vehicleChoices({ ambiguous: true, vehicles: [{ id: 1, index: 0, flightModes: ['Loiter', 'RTL'] }, { id: 2, index: 1, flightModes: [] }] }).choices;
    expect(choices[0]?.flightModes).toEqual(['Loiter', 'RTL']);
    expect(choices[1] === undefined ? null : vehicleFlightModePath(choices[1])).toBe('vehicles.vehicles.1.flightMode');
    expect(choices[1]?.flightModes).toEqual([]);
  });
});
