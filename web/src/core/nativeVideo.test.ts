import { describe, expect, it } from 'vitest';
import { nativeMessage, playsNatively } from './nativeVideo';

const bytes = (...values: number[]): ArrayBuffer => new Uint8Array(values).buffer;

describe('nativeMessage', () => {
  it('reads the layout the desktop shell sends', () => {
    expect(nativeMessage(bytes(0, 0, 11, ...new TextEncoder().encode('avc1.42c01f'), 0, 0, 5, 0, 0, 0, 2, 208, 1, 2))).toEqual({ kind: 'config', codec: 'avc1.42c01f', width: 1280, height: 720, description: new Uint8Array([1, 2]) });
    expect(nativeMessage(bytes(1, 1, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0, 9))).toEqual({ kind: 'frame', key: true, sentMs: 1.5, data: new Uint8Array([9]) });
    expect(nativeMessage(bytes(2, ...new TextEncoder().encode('gone')))).toEqual({ kind: 'ended', reason: 'gone' });
    expect(nativeMessage(bytes(7))).toBeNull();
  });

  it('plays RTSP natively and leaves WHEP to the browser', () => {
    expect(playsNatively('rtsp://10.0.0.5:554/live')).toBe(true);
    expect(playsNatively('http://falcon:8889/front/whep')).toBe(false);
  });
});
