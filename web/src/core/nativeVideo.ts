import { Channel, invoke } from '@tauri-apps/api/core';
import type { WhepCounters, WhepReport, WhepStats } from './whep';

export type NativeMessage =
  | { kind: 'config'; codec: string }
  | { kind: 'frame'; key: boolean; sentMs: number; packets: number; lost: number; data: Uint8Array }
  | { kind: 'ended'; reason: string };

export type NativePlayer = { stop: () => void; record: (file: string | null) => Promise<boolean> };

const CONFIG = 0;
const FRAME = 1;
const ENDED = 2;
const FRAME_HEADER = 14;
const REPORT_MS = 500;
const MAX_DECODER_FAILURES = 5;

const text = (bytes: Uint8Array): string => new TextDecoder().decode(bytes);

export const nativeMessage = (buffer: ArrayBuffer): NativeMessage | null => {
  const view = new DataView(buffer);
  const kind = view.getUint8(0);
  if (kind === CONFIG) return { kind: 'config', codec: text(new Uint8Array(buffer, 1)) };
  if (kind === FRAME) return { kind: 'frame', key: view.getUint8(1) === 1, sentMs: view.getFloat64(2), packets: view.getUint16(10), lost: view.getUint16(12), data: new Uint8Array(buffer, FRAME_HEADER) };
  if (kind === ENDED) return { kind: 'ended', reason: text(new Uint8Array(buffer, 1)) };
  return null;
};

export const playsNatively = (uri: string): boolean => /^(rtspt?|udp|udp265|mpegts|tcp):\/\//.test(uri);

export const arrivalJitterMs = (arrivals: readonly number[]): number => {
  const gaps = arrivals.slice(1).map((at, index) => at - arrivals[index]!);
  const mean = gaps.reduce((sum, gap) => sum + gap, 0) / Math.max(gaps.length, 1);
  return gaps.length === 0 ? 0 : Math.round(gaps.reduce((sum, gap) => sum + Math.abs(gap - mean), 0) / gaps.length);
};

export const nativeStats = (before: WhepCounters, now: WhepCounters, decoded: number, arrivals: readonly number[]): WhepStats => {
  const elapsedMs = Math.max(now.at - before.at, 1);
  const lost = now.lost - before.lost;
  const packets = lost + now.received - before.received;
  return {
    fps: Math.round((decoded * 1000) / elapsedMs),
    kbps: Math.round(((now.bytes - before.bytes) * 8) / elapsedMs),
    jitterMs: arrivalJitterMs(arrivals),
    lossPercent: packets > 0 ? (100 * lost) / packets : 0,
    skippedPerSecond: (1000 * (now.dropped - before.dropped)) / elapsedMs,
    counters: now,
  };
};

export const playNative = (canvas: HTMLCanvasElement, uri: string, report: (reading: WhepReport) => void): NativePlayer => {
  const context = canvas.getContext('2d');
  const session = { id: null as number | null, closed: false, keyed: false, frames: 0, failures: 0, error: '', first: true, config: null as VideoDecoderConfig | null };
  const totals = { bytes: 0, lost: 0, received: 0, dropped: 0, arrivals: [] as number[], reported: { bytes: 0, lost: 0, received: 0, dropped: 0, at: performance.now() } as WhepCounters, reportedFrames: 0 };
  const output = (frame: VideoFrame) => {
    if (canvas.width !== frame.displayWidth) canvas.width = frame.displayWidth;
    if (canvas.height !== frame.displayHeight) canvas.height = frame.displayHeight;
    context?.drawImage(frame, 0, 0);
    frame.close();
    session.frames += 1;
    session.failures = 0;
  };
  const fresh = (): VideoDecoder => {
    const made = new VideoDecoder({ output, error: (error) => recover(error.message) });
    if (session.config !== null) made.configure(session.config);
    return made;
  };
  const decoders = { current: fresh() };
  const recover = (reason: string) => {
    session.failures += 1;
    session.keyed = false;
    if (session.closed || session.failures > MAX_DECODER_FAILURES) session.error = reason;
    else decoders.current = fresh();
  };
  const frames = new Channel<ArrayBuffer>((buffer) => {
    const message = nativeMessage(buffer);
    const decoder = decoders.current;
    if (message === null || decoder.state === 'closed') return;
    if (message.kind === 'config') {
      session.config = { codec: message.codec, optimizeForLatency: true };
      session.keyed = false;
      decoder.configure(session.config);
    } else if (message.kind === 'frame') {
      totals.bytes += message.data.length;
      totals.received += message.packets;
      totals.lost += message.lost;
      totals.arrivals = [...totals.arrivals, performance.now()];
      if (decoder.state !== 'configured' || !(session.keyed || message.key)) {
        totals.dropped += 1;
        return;
      }
      session.keyed = true;
      decoder.decode(new EncodedVideoChunk({ type: message.key ? 'key' : 'delta', timestamp: Math.round(performance.now() * 1000), data: message.data }));
    } else {
      session.error = message.reason || 'The camera ended the stream.';
    }
  });
  const opened = invoke<number>('video_open', { url: uri, frames }).then(
    (id) => {
      session.id = id;
      if (session.closed) void invoke('video_close', { id });
      return id;
    },
    (error: unknown) => {
      session.error = error instanceof Error ? error.message : String(error);
      return null;
    },
  );
  const timer = setInterval(() => {
    const now: WhepCounters = { bytes: totals.bytes, lost: totals.lost, received: totals.received, dropped: totals.dropped, at: performance.now() };
    const stats = nativeStats(totals.reported, now, session.frames - totals.reportedFrames, totals.arrivals);
    totals.reported = now;
    totals.reportedFrames = session.frames;
    totals.arrivals = totals.arrivals.slice(-1);
    report({ running: session.frames > 0 && session.error === '', frames: session.frames, width: canvas.width, height: canvas.height, error: session.error, restarted: session.first, stats });
    session.first = false;
  }, REPORT_MS);
  return {
    stop: () => {
      session.closed = true;
      clearInterval(timer);
      if (session.id !== null) void invoke('video_close', { id: session.id });
      if (decoders.current.state !== 'closed') decoders.current.close();
    },
    record: async (file) => {
      const id = await opened;
      if (id === null || session.closed) return false;
      return invoke('video_record', { id, file, width: canvas.width, height: canvas.height }).then(
        () => file !== null,
        () => false,
      );
    },
  };
};
