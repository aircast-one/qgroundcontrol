import { describe, expect, it } from 'vitest';
import { firstAnswered, httpsOrigin, secureOrigins } from './DeviceOrigin';

describe('finding the https address a device answers on', () => {
  it('tries the name the device goes by first, then the name for the address it was reached on', () => {
    expect(secureOrigins({ origin: 'https://falcon-k3q7.d.aircast.one', fallback: 'https://192-168-1-100.falcon-k3q7.d.aircast.one' })).toEqual(['https://falcon-k3q7.d.aircast.one', 'https://192-168-1-100.falcon-k3q7.d.aircast.one']);
    expect(secureOrigins({ origin: '' })).toEqual([]);
    expect(secureOrigins({ origin: 'http://192.168.1.100' })).toEqual([]);
    expect(secureOrigins(null)).toEqual([]);
  });

  it('takes the preferred https name that answered, or none', () => {
    const origins = ['https://falcon-gb3v.d.aircast.one', 'https://192-168-1-100.falcon-gb3v.d.aircast.one'];
    expect(firstAnswered(origins, [true, true])).toBe(origins[0]);
    expect(firstAnswered(origins, [false, true])).toBe(origins[1]);
    expect(firstAnswered(origins, [false, false])).toBeNull();
  });

  it('takes only a bare https origin as a device address', () => {
    expect(httpsOrigin('https://100-64-0-15.falcon-gb3v.d.aircast.one')).toBe('https://100-64-0-15.falcon-gb3v.d.aircast.one');
    expect(httpsOrigin('https://localhost:18785')).toBe('https://localhost:18785');
    expect(httpsOrigin('http://192.168.1.100')).toBeNull();
    expect(httpsOrigin('https://falcon.local/fly/')).toBeNull();
    expect(httpsOrigin('https://user@falcon.local')).toBeNull();
    expect(httpsOrigin('javascript:alert(1)')).toBeNull();
    expect(httpsOrigin('not a url')).toBeNull();
  });
});
