import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { aircastLink, sparkRuns } from './AircastLinkIndicator';

describe('AircastLinkIndicatorTest', () => {
  it('the link shows only once cellular telemetry arrives', () => {
    expect(aircastLink(parseJson('{"shown":false}'))).toBeNull();
    const link = aircastLink(parseJson('{"shown":true,"qualityText":"73%","signalText":"73 %","network":"LTE","modem":"connected","bitrateText":"2.5 Mbit/s","qualityHistory":[73,-1],"bitrateHistory":[2048,0]}'))!;
    expect([link.cellText, link.network, link.modem, link.bitrateText]).toEqual(['73%', 'LTE', 'connected', '2.5 Mbit/s']);
    expect(link.qualityHistory).toEqual([73.0, -1.0]);
    expect([link.uplinkText, link.switches]).toEqual(['', []]);
  });

  it('the cell names the uplink in use and the sheet lists its switches newest first', () => {
    const link = aircastLink(parseJson('{"shown":true,"quality":null,"qualityText":"--","signalText":"unknown","bitrateText":"1.2 Mbit/s","uplink":"wifi","uplinkText":"Wi-Fi","cellText":"Wi-Fi","switches":[{"atMs":2000,"clock":"21:04:09","from":"LTE","to":"Wi-Fi"},{"atMs":1000,"clock":"21:03:41","from":"Starlink","to":"LTE"}]}'))!;
    expect([link.cellText, link.uplinkText]).toEqual(['Wi-Fi', 'Wi-Fi']);
    expect(link.switches).toEqual([
      { clock: '21:04:09', from: 'LTE', to: 'Wi-Fi' },
      { clock: '21:03:41', from: 'Starlink', to: 'LTE' },
    ]);
    expect(aircastLink(parseJson('{"shown":true,"uplink":null,"uplinkText":"","cellText":"87%"}'))?.uplinkText).toBe('');
  });

  it('a sparkline breaks at unknown samples and scales to its peak', () => {
    const runs = sparkRuns([50.0, 100.0, -1.0, 0.0, 25.0, -1.0, 10.0], 0.0);
    expect(runs.length).toBe(2);
    expect(runs[0]).toEqual([
      [0, 0.5],
      [1 / 6, 0],
    ]);
    expect(runs[1]).toEqual([
      [0.5, 1],
      [4 / 6, 0.75],
    ]);
    expect(sparkRuns([5.0], 100.0)).toEqual([]);
  });
});
