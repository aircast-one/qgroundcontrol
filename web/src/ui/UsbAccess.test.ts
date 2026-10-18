import { describe, expect, it } from 'vitest';
import { CHROME_INSECURE_ORIGIN_FLAG, usbBlockedNote } from './UsbAccess';

describe('UsbAccessTest', () => {
  it('a browser that reaches USB needs no note', () => {
    expect(usbBlockedNote(true, true, 'https://fly.aircast.one')).toBeNull();
  });

  it('a device page over plain http names the Chrome flag and the exact address to add', () => {
    const note = usbBlockedNote(false, false, 'http://falcon.local');
    expect(note).toContain(CHROME_INSECURE_ORIGIN_FLAG);
    expect(note).toContain('add http://falcon.local,');
  });

  it('a secure page without Web Serial asks for Chrome or Edge', () => {
    expect(usbBlockedNote(false, true, 'https://fly.aircast.one')).toBe('This browser cannot reach USB devices. Use Chrome or Edge, or connect over a WebSocket.');
  });
});
