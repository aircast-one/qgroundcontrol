import { describe, expect, it } from 'vitest';
import { CHROME_INSECURE_ORIGIN_FLAG, firstAnswered, secureOrigins, securePage, usbBlocked } from './UsbAccess';

describe('UsbAccessTest', () => {
  it('a browser that reaches USB needs no note', () => {
    expect(usbBlocked(true, true, 'https://fly.aircast.one/')).toBeNull();
  });

  it('a device page over plain http first offers the same page over https', () => {
    expect(usbBlocked(false, false, 'http://falcon.local/fly/?tab=1#links')?.secureUrl).toBe('https://falcon.local/fly/?tab=1#links');
  });

  it('and names the Chrome flag with the origin to add, not the page', () => {
    const note = usbBlocked(false, false, 'http://falcon.local/fly/')?.note ?? '';
    expect(note).toContain(CHROME_INSECURE_ORIGIN_FLAG);
    expect(note).toContain('add http://falcon.local,');
  });

  it('a secure page without Web Serial asks for Chrome or Edge', () => {
    expect(usbBlocked(false, true, 'https://fly.aircast.one/')).toEqual({ secureUrl: null, note: 'This browser cannot reach USB devices. Use Chrome or Edge, or connect over a WebSocket.' });
  });

  it('opens the page on the https name the device holds a certificate for, keeping where the pilot was', () => {
    const named = 'https://192-168-1-50.6f1c2a9e-3b7d-4e21-9a55-0c8d7e6f5a41.d.aircast.one';
    expect(securePage('http://192.168.1.50/?x=1#settings', named)).toBe(`${named}/?x=1#settings`);
    expect(securePage('http://192.168.1.50/', `${named}/ignored/path`)).toBe(`${named}/`);
    expect(securePage('http://192.168.1.50:8080/', null)).toBe('https://192.168.1.50:8080/');
  });

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
});
