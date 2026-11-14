import { Channel, invoke } from '@tauri-apps/api/core';
import type { WhepReport } from './whep';

export type NativeMessage =
  | { kind: 'config'; codec: string; width: number; height: number; description: Uint8Array }
  | { kind: 'frame'; key: boolean; sentMs: number; data: Uint8Array }
  | { kind: 'ended'; reason: string };

const CONFIG = 0;
const FRAME = 1;
const ENDED = 2;
const FRAME_HEADER = 10;
const REPORT_MS = 500;

const text = (bytes: Uint8Array): string => new TextDecoder().decode(bytes);

export const nativeMessage = (buffer: ArrayBuffer): NativeMessage | null => {
  const view = new DataView(buffer);
  const kind = view.getUint8(0);
  if (kind === CONFIG) {
    const codecEnd = 3 + view.getUint16(1);
    return { kind: 'config', codec: text(new Uint8Array(buffer, 3, codecEnd - 3)), width: view.getUint32(codecEnd), height: view.getUint32(codecEnd + 4), description: new Uint8Array(buffer, codecEnd + 8) };
  }
  if (kind === FRAME) return { kind: 'frame', key: view.getUint8(1) === 1, sentMs: view.getFloat64(2), data: new Uint8Array(buffer, FRAME_HEADER) };
  if (kind === ENDED) return { kind: 'ended', reason: text(new Uint8Array(buffer, 1)) };
  return null;
};

export const playsNatively = (uri: string): boolean => uri.startsWith('rtsp://');

export const playNative = (canvas: HTMLCanvasElement, uri: string, report: (reading: WhepReport) => void): (() => void) => {
  const context = canvas.getContext('2d');
  const session = { id: null as number | null, closed: false, keyed: false, frames: 0, error: '', first: true };
  const decoder = new VideoDecoder({
    output: (frame) => {
      if (canvas.width !== frame.displayWidth) canvas.width = frame.displayWidth;
      if (canvas.height !== frame.displayHeight) canvas.height = frame.displayHeight;
      context?.drawImage(frame, 0, 0);
      frame.close();
      session.frames += 1;
    },
    error: (error) => {
      session.error = error.message;
    },
  });
  const frames = new Channel<ArrayBuffer>((buffer) => {
    const message = nativeMessage(buffer);
    if (message === null || decoder.state === 'closed') return;
    if (message.kind === 'config') {
      decoder.configure({ codec: message.codec, codedWidth: message.width, codedHeight: message.height, description: message.description, optimizeForLatency: true });
      session.keyed = false;
    } else if (message.kind === 'frame' && decoder.state === 'configured' && (session.keyed || message.key)) {
      session.keyed = true;
      decoder.decode(new EncodedVideoChunk({ type: message.key ? 'key' : 'delta', timestamp: Math.round(performance.now() * 1000), data: message.data }));
    } else if (message.kind === 'ended') {
      session.error = message.reason || 'The camera ended the stream.';
    }
  });
  invoke<number>('video_open', { url: uri, frames }).then(
    (id) => (session.closed ? void invoke('video_close', { id }) : (session.id = id)),
    (error: unknown) => {
      session.error = error instanceof Error ? error.message : String(error);
    },
  );
  const timer = setInterval(() => {
    report({ running: session.frames > 0 && session.error === '', frames: session.frames, width: canvas.width, height: canvas.height, error: session.error, restarted: session.first, stats: null });
    session.first = false;
  }, REPORT_MS);
  return () => {
    session.closed = true;
    clearInterval(timer);
    if (session.id !== null) void invoke('video_close', { id: session.id });
    if (decoder.state !== 'closed') decoder.close();
  };
};
