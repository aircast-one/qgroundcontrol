import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { RadarSector, proximityRadar } from './ProximityRadar';

describe('ProximityRadarTest', () => {
  it('the radar shows only while distance telemetry arrives', () => {
    expect(proximityRadar(parseJson('{"shown":false,"sectors":[]}'))).toBeNull();
    const radar = proximityRadar(parseJson('{"shown":true,"rangeMeters":6,"sectors":[{"bearing":0,"meters":2.5,"text":"2.50"},{"bearing":45,"meters":null,"text":"–.––"}]}'))!;
    expect(radar.range).toBe(6.0);
    expect(radar.sectors).toEqual([RadarSector(0, 2.5, '2.50'), RadarSector(45, null, '–.––')]);
  });
});
