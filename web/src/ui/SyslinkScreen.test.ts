import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { hexAddress, syslink } from './SyslinkScreen';

describe('SyslinkScreenTest', () => {
  it('the radio settings read as SyslinkComponent shows them', () => {
    const radio = syslink(
      parseJson('{"available":true,"channel":80,"address":"e7e7e7e7e7","rate":2,"rates":["750Kb/s","1Mb/s","2Mb/s"],"channelHint":"Channel can be between 0 and 125","addressHint":"Address in hex. Default is E7E7E7E7E7."}'),
    );
    if (radio === null) throw new Error('parsed');
    expect(radio.channel).toBe(80);
    expect(radio.rates[radio.rate]).toBe('2Mb/s');
    expect(syslink(parseJson('{"available":false}'))).toBeNull();
  });

  it('the address field takes hex only, as its RegExpValidator does', () => {
    expect(hexAddress('E7e7')).toBe(true);
    expect(hexAddress('')).toBe(true);
    expect(hexAddress('E7G')).toBe(false);
    expect(hexAddress('E7E7E7E7E7E')).toBe(false);
  });
});
