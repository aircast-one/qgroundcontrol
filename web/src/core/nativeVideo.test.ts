import { describe, expect, it } from 'vitest';
import { decoderFailed, nativeMessage, nativeStats, playbackStep, playsNatively, restarted, startingPlayback } from './nativeVideo';

const bytes = (...values: number[]): ArrayBuffer => new Uint8Array(values).buffer;

describe('nativeMessage', () => {
  it('reads the layout the desktop shell sends', () => {
    expect(nativeMessage(bytes(0, ...new TextEncoder().encode('avc1.42c01f')))).toEqual({ kind: 'config', codec: 'avc1.42c01f' });
    expect(nativeMessage(bytes(1, 1, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 2, 9))).toEqual({ kind: 'frame', key: true, sentMs: 1.5, packets: 3, lost: 2, data: new Uint8Array([9]) });
    expect(nativeMessage(bytes(2, ...new TextEncoder().encode('gone')))).toEqual({ kind: 'ended', reason: 'gone' });
    expect(nativeMessage(bytes(3, ...new TextEncoder().encode('full')))).toEqual({ kind: 'recordingStopped', reason: 'full' });
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

  it('leaves packet loss unknown for a source that does not count its packets, rather than calling every loss 100%', () => {
    const before = { bytes: 0, lost: 0, received: 0, dropped: 0, at: 1000 };
    expect(nativeStats(before, { ...before, lost: 3, at: 1500 }, 15, []).lossPercent).toBeNull();
  });
});

describe('playbackStep', () => {
  const frame = (key: boolean) => ({ kind: 'frame' as const, key, sentMs: 0, packets: 1, lost: 0, data: new Uint8Array([9]) });

  it('waits for a keyframe after the decoder is configured, then decodes every frame', () => {
    const [configured, configure] = playbackStep(startingPlayback, { kind: 'config', codec: 'avc1.42c01f' }, false);
    expect(configure).toEqual({ kind: 'configure', config: { codec: 'avc1.42c01f', optimizeForLatency: true } });
    expect(playbackStep(configured, frame(true), false)[1]).toEqual({ kind: 'skip' });
    expect(playbackStep(configured, frame(false), true)[1]).toEqual({ kind: 'skip' });
    const [keyed, decode] = playbackStep(configured, frame(true), true);
    expect(decode).toEqual({ kind: 'decode', key: true, data: new Uint8Array([9]) });
    expect(playbackStep(keyed, frame(false), true)[1].kind).toBe('decode');
    expect(playbackStep(keyed, { kind: 'config', codec: 'hev1.1.6.L93.90' }, true)[0].keyed).toBe(false);
  });

  it('a stopped recording is reported without failing the stream, and an ended stream is', () => {
    const [still, step] = playbackStep(startingPlayback, { kind: 'recordingStopped', reason: 'disk full' }, true);
    expect([still.error, step]).toEqual(['', { kind: 'recordingStopped', reason: 'disk full' }]);
    expect(playbackStep(startingPlayback, { kind: 'ended', reason: '' }, true)[0].error).toBe('The camera ended the stream.');
  });

  it('rebuilds a failed decoder a few times and then reports the error, and a restart clears it', () => {
    const keyed = { ...startingPlayback, keyed: true };
    expect(decoderFailed(keyed, 1, 'bad frame')).toEqual([{ ...startingPlayback, keyed: false }, true]);
    const [given, rebuild] = decoderFailed(keyed, 6, 'bad frame');
    expect([given.error, rebuild]).toEqual(['bad frame', false]);
    expect(restarted(given)).toEqual({ ...startingPlayback });
  });
});
