import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { linkAwaitingVehicle, vehicleConnected } from './ConnectionLocks';

describe('ConnectionLocksTest', () => {
  it("the locks follow MultiVehicleManager's first vehicle in, last vehicle out", () => {
    expect(vehicleConnected(parseJson('{"count":1}'))).toBe(true);
    expect(vehicleConnected(parseJson('{"count":3}'))).toBe(true);
    expect(vehicleConnected(parseJson('{"count":0}'))).toBe(false);
    expect(vehicleConnected(null)).toBe(false);
  });

  it('a link the pilot opened keeps the app on the network before the vehicle answers, an automatic listener does not', () => {
    const opened = parseJson('{"configured":[{"index":0,"name":"SITL","connected":true,"heardVehicle":false}],"links":[]}');
    const closed = parseJson('{"configured":[{"index":0,"name":"SITL","connected":false}],"links":[]}');
    const listening = parseJson('{"configured":[],"links":[{"index":1,"name":"UDP","dynamic":true,"connected":true}]}');
    expect(linkAwaitingVehicle(opened)).toBe(true);
    expect(linkAwaitingVehicle(closed)).toBe(false);
    expect(linkAwaitingVehicle(listening)).toBe(false);
    expect(linkAwaitingVehicle(null)).toBe(false);
  });
});
