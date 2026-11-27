import { describe, expect, it } from 'vitest';
import { type Decoding, decoderStep, nativeLogLine, nativeMessage, nativeStats, nativeTransport, playbackStep, playsNatively, recordOutcome, startingPlayback } from './nativeVideo';

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
    expect(nativeStats(before, now, 15, [0, 33, 66, 109, 133])).toEqual({ fps: 30, kbps: 1000, jitterMs: 5, lossPercent: 5, lostPerSecond: 10, skippedPerSecond: 4, counters: now });
  });

  it('leaves packet loss unknown for a source that does not count its packets, rather than calling every loss 100%', () => {
    const before = { bytes: 0, lost: 0, received: 0, dropped: 0, at: 1000 };
    expect(nativeStats(before, { ...before, lost: 3, at: 1500 }, 15, []).lossPercent).toBeNull();
  });
});

describe('nativeLogLine', () => {
  it('logs what reached the player and what the decoder made of it over the window, with the transport the scheme asks for', () => {
    const since = { frames: 0, keyframes: 0, packets: 0, lost: 0, bytes: 0, decoded: 0, skipped: 0, errors: 0, jitterMs: 0, reports: 0, at: 1000 };
    const now = { frames: 250, keyframes: 3, packets: 0, lost: 14, bytes: 2_500_000, decoded: 230, skipped: 18, errors: 1, jitterMs: 180, reports: 20, at: 11_000 };
    expect(nativeLogLine({ event: 'playback_stats', stream: 3, transport: nativeTransport('rtsp://drone:8554/viewpro'), since, now, keyAt: 9200, queued: 2 })).toBe(
      'event=playback_stats stream=3 transport=udp seconds=10.0 frames=250 keyframes=3 packets=unknown lost=14 kbps=2000 jitter_ms=9 decoded=230 skipped=18 decoder_errors=1 queued=2 since_keyframe_ms=1800',
    );
    expect(nativeLogLine({ event: 'playback_stopped', stream: null, transport: nativeTransport('rtspt://drone:8554/viewpro'), since, now: { ...since, at: 3500 }, keyAt: null, queued: 0 })).toBe(
      'event=playback_stopped stream=none transport=tcp seconds=2.5 frames=0 keyframes=0 packets=0 lost=0 kbps=0 jitter_ms=0 decoded=0 skipped=0 decoder_errors=0 queued=0 since_keyframe_ms=none',
    );
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

  it('rebuilds a failed decoder a few times, then gives up and reports the error, and a decoded frame resets the count', () => {
    const playing: Decoding = { playback: { ...startingPlayback, keyed: true }, failures: 0 };
    const [once, rebuildOnce] = decoderStep(playing, { kind: 'failed', reason: 'bad frame' });
    expect([once, rebuildOnce]).toEqual([{ playback: startingPlayback, failures: 1 }, true]);
    expect(decoderStep(once, { kind: 'decoded' })[0].failures).toBe(0);
    const failing = Array.from({ length: 6 }).reduce<readonly [Decoding, boolean]>(([decoding]) => decoderStep(decoding, { kind: 'failed', reason: 'bad frame' }), [playing, true]);
    expect([failing[0].playback.error, failing[1]]).toEqual(['bad frame', false]);
  });

  it('a restart brings back a decoder that gave up, so the core retrying the stream can recover the picture', () => {
    const givenUp: Decoding = { playback: { ...startingPlayback, error: 'bad frame' }, failures: 6 };
    expect(decoderStep(givenUp, { kind: 'restarted' })).toEqual([{ playback: startingPlayback, failures: 0 }, true]);
  });
});

describe('recordOutcome', () => {
  it('a newer record or stop request supersedes one still waiting for the first frame, so a stopped recording never starts late', () => {
    expect(recordOutcome('/v/a.mp4', 1, 2, true)).toBe('superseded');
    expect(recordOutcome('/v/a.mp4', 2, 2, true)).toBe('recording');
    expect(recordOutcome(null, 3, 3, true)).toBe('stopped');
    expect(recordOutcome(null, 4, 5, true)).toBe('superseded');
  });

  it('a stream that is no longer playing records nothing, so a stop after it closed still reports the recording stopped', () => {
    expect(recordOutcome('/v/a.mp4', 6, 6, false)).toBe('stopped');
    expect(recordOutcome(null, 7, 7, false)).toBe('stopped');
  });
});
