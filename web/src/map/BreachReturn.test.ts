import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { breachAltitudeText } from './BreachReturnDialog';
import { breachReturn } from './FenceBridge';

describe('BreachReturnTest', () => {
  it('the breach return point and its altitude read from the fences view', () => {
    const read = breachReturn(
      parseJson(
        '{"breachReturnPoint":{"latitude":47.4,"longitude":8.5},' + '"breachReturnAltitude":{"value":98.4,"units":"ft","path":"plan.geoFenceController.breachReturnAltitude"}}',
      ),
    );
    expect(read).toEqual({ point: { latitude: 47.4, longitude: 8.5 }, altitude: 98.4, units: 'ft', altitudePath: 'plan.geoFenceController.breachReturnAltitude' });
    expect(breachReturn(parseJson('{"breachReturnPoint":null}'))).toBeNull();
    expect(breachAltitudeText(98.4)).toBe('98.4');
  });
});
