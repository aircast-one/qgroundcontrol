import { Channel, invoke } from '@tauri-apps/api/core';
import type { WhepCounters, WhepReport, WhepStats } from './whep';

export type NativeMessage =
  | { kind: 'config'; codec: string }
  | { kind: 'frame'; key: boolean; sentMs: number; packets: number; lost: number; data: Uint8Array }
  | { kind: 'ended'; reason: string }
  | { kind: 'recordingStopped'; reason: string };

export type NativePlayer = { stop: () => void; restart: () => Promise<void>; record: (file: string | null) => Promise<boolean> };

export type Playback = { config: VideoDecoderConfig | null; keyed: boolean; error: string };

export type PlaybackStep =
  | { kind: 'configure'; config: VideoDecoderConfig }
  | { kind: 'decode'; key: boolean; data: Uint8Array }
  | { kind: 'skip' }
  | { kind: 'recordingStopped'; reason: string }
  | { kind: 'none' };

type FrameSize = { width: number; height: number };

const CONFIG = 0;
const FRAME = 1;
const ENDED = 2;
const RECORDING_STOPPED = 3;
const FRAME_HEADER = 14;
const REPORT_MS = 500;
const MAX_DECODER_FAILURES = 5;
const ENDED_WITHOUT_REASON = 'The camera ended the stream.';
const NOT_RECORDING: FrameSize = { width: 0, height: 0 };

const text = (bytes: Uint8Array): string => new TextDecoder().decode(bytes);

export const nativeMessage = (buffer: ArrayBuffer): NativeMessage | null => {
  const view = new DataView(buffer);
  const kind = view.getUint8(0);
  if (kind === CONFIG) return { kind: 'config', codec: text(new Uint8Array(buffer, 1)) };
  if (kind === FRAME) return { kind: 'frame', key: view.getUint8(1) === 1, sentMs: view.getFloat64(2), packets: view.getUint16(10), lost: view.getUint16(12), data: new Uint8Array(buffer, FRAME_HEADER) };
  if (kind === ENDED) return { kind: 'ended', reason: text(new Uint8Array(buffer, 1)) };
  if (kind === RECORDING_STOPPED) return { kind: 'recordingStopped', reason: text(new Uint8Array(buffer, 1)) };
  return null;
};

export const playsNatively = (uri: string): boolean => /^(rtspt?|udp|udp265|mpegts|tcp):\/\//.test(uri);

export const startingPlayback: Playback = { config: null, keyed: false, error: '' };

export const playbackStep = (playback: Playback, message: NativeMessage, configured: boolean): readonly [Playback, PlaybackStep] => {
  switch (message.kind) {
    case 'config': {
      const config: VideoDecoderConfig = { codec: message.codec, optimizeForLatency: true };
      return [{ ...playback, config, keyed: false }, { kind: 'configure', config }];
    }
    case 'frame':
      return configured && (playback.keyed || message.key) ? [{ ...playback, keyed: true }, { kind: 'decode', key: message.key, data: message.data }] : [playback, { kind: 'skip' }];
    case 'ended':
      return [{ ...playback, error: message.reason || ENDED_WITHOUT_REASON }, { kind: 'none' }];
    case 'recordingStopped':
      return [playback, { kind: 'recordingStopped', reason: message.reason }];
  }
};

export const decoderFailed = (playback: Playback, failures: number, reason: string): readonly [Playback, boolean] =>
  failures > MAX_DECODER_FAILURES ? [{ ...playback, keyed: false, error: reason }, false] : [{ ...playback, keyed: false }, true];

export const restarted = (playback: Playback): Playback => ({ ...playback, keyed: false, error: '' });

export const arrivalJitterMs = (arrivals: readonly number[]): number => {
  const gaps = arrivals.slice(1).map((at, index) => at - arrivals[index]!);
  const mean = gaps.reduce((sum, gap) => sum + gap, 0) / Math.max(gaps.length, 1);
  return gaps.length === 0 ? 0 : Math.round(gaps.reduce((sum, gap) => sum + Math.abs(gap - mean), 0) / gaps.length);
};

export const nativeStats = (before: WhepCounters, now: WhepCounters, decoded: number, arrivals: readonly number[]): WhepStats => {
  const elapsedMs = Math.max(now.at - before.at, 1);
  const lost = now.lost - before.lost;
  const received = now.received - before.received;
  return {
    fps: Math.round((decoded * 1000) / elapsedMs),
    kbps: Math.round(((now.bytes - before.bytes) * 8) / elapsedMs),
    jitterMs: arrivalJitterMs(arrivals),
    lossPercent: received > 0 ? (100 * lost) / (lost + received) : null,
    skippedPerSecond: (1000 * (now.dropped - before.dropped)) / elapsedMs,
    counters: now,
  };
};

export const playNative = (canvas: HTMLCanvasElement, uri: string, report: (reading: WhepReport) => void, recordingStopped: (reason: string) => void): NativePlayer => {
  const context = canvas.getContext('2d');
  const session = { id: null as number | null, closed: false, frames: 0, failures: 0, first: true, playback: startingPlayback };
  const totals = { bytes: 0, lost: 0, received: 0, dropped: 0, arrivals: [] as number[], reported: { bytes: 0, lost: 0, received: 0, dropped: 0, at: performance.now() } as WhepCounters, reportedFrames: 0 };
  const sizing: { resolve: (size: FrameSize) => void; reject: (reason: Error) => void } = { resolve: () => undefined, reject: () => undefined };
  const firstFrame = new Promise<FrameSize>((resolve, reject) => Object.assign(sizing, { resolve, reject }));
  firstFrame.catch(() => undefined);
  const output = (frame: VideoFrame) => {
    if (canvas.width !== frame.displayWidth) canvas.width = frame.displayWidth;
    if (canvas.height !== frame.displayHeight) canvas.height = frame.displayHeight;
    context?.drawImage(frame, 0, 0);
    sizing.resolve({ width: frame.displayWidth, height: frame.displayHeight });
    frame.close();
    session.frames += 1;
    session.failures = 0;
  };
  const fresh = (): VideoDecoder => {
    const made = new VideoDecoder({ output, error: (error) => recover(error.message) });
    if (session.playback.config !== null) made.configure(session.playback.config);
    return made;
  };
  const decoders = { current: fresh() };
  const recover = (reason: string) => {
    if (session.closed) return;
    session.failures += 1;
    const [playback, rebuild] = decoderFailed(session.playback, session.failures, reason);
    session.playback = playback;
    if (rebuild) decoders.current = fresh();
  };
  const apply = (decoder: VideoDecoder, step: PlaybackStep) => {
    if (step.kind === 'configure') decoder.configure(step.config);
    else if (step.kind === 'decode') decoder.decode(new EncodedVideoChunk({ type: step.key ? 'key' : 'delta', timestamp: Math.round(performance.now() * 1000), data: step.data }));
    else if (step.kind === 'skip') totals.dropped += 1;
    else if (step.kind === 'recordingStopped') recordingStopped(step.reason);
  };
  const frames = new Channel<ArrayBuffer>((buffer) => {
    const message = nativeMessage(buffer);
    const decoder = decoders.current;
    if (message === null || decoder.state === 'closed') return;
    if (message.kind === 'frame') {
      totals.bytes += message.data.length;
      totals.received += message.packets;
      totals.lost += message.lost;
      totals.arrivals = [...totals.arrivals, performance.now()];
    }
    const [playback, step] = playbackStep(session.playback, message, decoder.state === 'configured');
    session.playback = playback;
    apply(decoder, step);
  });
  const opened = invoke<number>('video_open', { url: uri, frames }).then(
    (id) => {
      session.id = id;
      if (session.closed) void invoke('video_close', { id });
      return id;
    },
    (error: unknown) => {
      session.playback = { ...session.playback, error: error instanceof Error ? error.message : String(error) };
      return null;
    },
  );
  const timer = setInterval(() => {
    const now: WhepCounters = { bytes: totals.bytes, lost: totals.lost, received: totals.received, dropped: totals.dropped, at: performance.now() };
    const stats = nativeStats(totals.reported, now, session.frames - totals.reportedFrames, totals.arrivals);
    totals.reported = now;
    totals.reportedFrames = session.frames;
    totals.arrivals = totals.arrivals.slice(-1);
    report({ running: session.frames > 0 && session.playback.error === '', frames: session.frames, width: canvas.width, height: canvas.height, error: session.playback.error, restarted: session.first, stats });
    session.first = false;
  }, REPORT_MS);
  return {
    stop: () => {
      session.closed = true;
      clearInterval(timer);
      sizing.reject(new Error('The stream stopped before its first frame.'));
      if (session.id !== null) void invoke('video_close', { id: session.id });
      if (decoders.current.state !== 'closed') decoders.current.close();
    },
    restart: async () => {
      const id = await opened;
      if (id === null || session.closed) return;
      session.playback = restarted(session.playback);
      session.first = true;
      await invoke('video_restart', { id });
    },
    record: async (file) => {
      const id = await opened;
      const size = file === null ? NOT_RECORDING : await firstFrame.catch(() => null);
      if (id === null || size === null || session.closed) return false;
      return invoke('video_record', { id, file, width: size.width, height: size.height }).then(
        () => file !== null,
        () => false,
      );
    },
  };
};
