import { describe, expect, it } from 'vitest';
import { nativeMessage, nativeStats, playsNatively } from './nativeVideo';

const bytes = (...values: number[]): ArrayBuffer => new Uint8Array(values).buffer;

describe('nativeMessage', () => {
  it('reads the layout the desktop shell sends', () => {
    expect(nativeMessage(bytes(0, ...new TextEncoder().encode('avc1.42c01f')))).toEqual({ kind: 'config', codec: 'avc1.42c01f' });
    expect(nativeMessage(bytes(1, 1, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 2, 9))).toEqual({ kind: 'frame', key: true, sentMs: 1.5, packets: 3, lost: 2, data: new Uint8Array([9]) });
    expect(nativeMessage(bytes(2, ...new TextEncoder().encode('gone')))).toEqual({ kind: 'ended', reason: 'gone' });
    expect(nativeMessage(bytes(7))).toBeNull();
  });

  it('plays RTSP, RTP and MPEG-TS natively and leaves WHEP to the browser', () => {
    expect(['rtsp://10.0.0.5:554/live', 'rtspt://10.0.0.5:554/live', 'udp://0.0.0.0:5600', 'udp265://0.0.0.0:5600', 'mpegts://0.0.0.0:5600', 'tcp://10.0.0.5:5600'].every(playsNatively)).toBe(true);
    expect(playsNatively('http://falcon:8889/front/whep')).toBe(false);
  });
});

describe('nativeStats', () => {
  it('fills the video stats pill from what the shell counted over the last half second', () => {
    const before = { bytes: 0, lost: 0, received: 0, dropped: 0, at: 1000 };
    const now = { bytes: 62_500, lost: 5, received: 95, dropped: 2, at: 1500 };
    expect(nativeStats(before, now, 15, [0, 33, 66, 109, 133])).toEqual({ fps: 30, kbps: 1000, jitterMs: 5, lossPercent: 5, skippedPerSecond: 4, counters: now });
  });
});
