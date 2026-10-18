import { describe, expect, it } from 'vitest';
import { CHROME_INSECURE_ORIGIN_FLAG, usbBlocked } from './UsbAccess';

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
});
