import { describe, expect, it } from 'vitest';
import { flightGlance, lostGlance, lowestBattery } from './ArmedFlightBar';
import { FlyState } from './FlyState';
import { BatteryLevel, BatteryReading } from './StatusStrip';
import { VehicleLink, VehicleLinks } from './VehicleLinks';

const armed = FlyState({ connected: true, armed: true, contactLost: false, state: 'flying', stateText: '', staleNotice: '', mode: 'Loiter', rcSupported: true, rcSignalText: '', rcSignal: null, rcOverride: null, telemetry: null });

describe('ArmedFlightBarTest', () => {
  it('reads the mode, height, the lowest battery and the link while armed', () => {
    const batteries = [BatteryReading({ text: 'B1 70% · 11.10V', level: BatteryLevel.Caution }), BatteryReading({ text: 'B2 18%', level: BatteryLevel.Critical })];
    expect(flightGlance(armed, ['12.3', 'm'], batteries, null)).toEqual({ mode: 'Loiter', altitude: '12.3 m', battery: '18%', batteryLow: true, link: 'OK', linkLost: false, dropped: false });
  });

  it('says the link is lost and counts links the way the status strip does', () => {
    const two = VehicleLinks({ available: true, links: [VehicleLink(false), VehicleLink(true)] });
    expect(flightGlance(armed, null, [], two)?.link).toBe('1 link lost');
    expect(flightGlance({ ...armed, contactLost: true }, null, [], null)).toMatchObject({ link: 'Lost', linkLost: true, altitude: null, battery: null });
  });

  it('a battery with no percentage is never picked over one that reports its charge', () => {
    const unknown = BatteryReading({ text: 'B1 11.10V', level: BatteryLevel.Normal });
    const known = BatteryReading({ text: 'B2 64%', level: BatteryLevel.Caution });
    expect(lowestBattery([unknown, known])).toBe(known);
    expect(lowestBattery([unknown])).toBe(unknown);
    expect(lowestBattery([])).toBeNull();
  });

  it('a dropped aircraft is shown as the link lost with where it was last seen', () => {
    const lost = { name: 'Vehicle 1', at: { latitude: 47.4, longitude: 8.5 }, heading: 90, home: null, altitude: ' 20.0 m ', battery: 64, failsafe: null };
    expect(lostGlance(lost)).toEqual({ mode: 'Link lost · Vehicle 1', altitude: '20.0 m', battery: '64%', batteryLow: false, link: 'Lost', linkLost: true, dropped: true });
    expect(lostGlance({ ...lost, altitude: null, battery: null })).toMatchObject({ altitude: null, battery: null });
  });

  it('shows nothing on the ground or without a vehicle', () => {
    expect(flightGlance({ ...armed, armed: false }, null, [], null)).toBeNull();
    expect(flightGlance({ ...armed, connected: false }, null, [], null)).toBeNull();
    expect(flightGlance(null, null, [], null)).toBeNull();
  });
});
