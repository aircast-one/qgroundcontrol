import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { packetRadioStatus } from './PacketRadioSection';

describe('PacketRadioSectionTest', () => {
  it('link quality reads like packet radio settings qml', () => {
    const status = packetRadioStatus(
      parseJson(`{"class":"PacketRadio","statusText":"Receiving on ALFA","linkActive":true,"haveSignal":true,
            "antennaRssiRaw":[60,70],"antennaSnr":[20,25],"linkScore":1800,"packetLoss":2,"videoPackets":1234,"adapters":["ALFA [1]"]}`),
    )!;
    expect(status.signal).toBe('60  |  70');
    expect(status.noise).toBe('20  |  25 dB');
    expect(status.linkScore).toBe('1800');
    expect(status.adapters).toEqual(['ALFA [1]']);
    const quiet = packetRadioStatus(parseJson('{"class":"PacketRadio","statusText":"Listening","linkActive":true,"haveSignal":false}'))!;
    expect([quiet.signal, quiet.linkScore]).toEqual(['waiting', 'waiting']);
    expect(packetRadioStatus(parseJson('{"kind":"null"}'))).toBeNull();
  });
});
