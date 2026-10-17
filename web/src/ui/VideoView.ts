import { type Json, isNull, jsonObjects, optBoolean, optInt, optJSONArray, optJSONObject, optString, optText } from '@/bridge/Json';

export const VIDEO_VIEW = 'view.video';

export type VideoCamera = { slot: number; status: string; configured: boolean };

export type SourceSize = { width: number; height: number };

export type VideoReading = {
  available: boolean;
  decoding: boolean;
  sourceSize: SourceSize | null;
  summary: string;
  activeSource: number;
  cameras: readonly VideoCamera[];
  streaming: boolean;
  streamEnabled: boolean;
  noVideoText: string;
  noVideoReason: string;
  sourceChosen: boolean;
  standIn: string | null;
};

export const VideoReading = (
  fields: Pick<VideoReading, 'available' | 'decoding' | 'sourceSize' | 'summary' | 'activeSource' | 'cameras'> & Partial<VideoReading>,
): VideoReading => ({
  streaming: false,
  streamEnabled: true,
  noVideoText: '',
  noVideoReason: '',
  sourceChosen: true,
  standIn: null,
  ...fields,
});

const sourceSizeOf = (size: Json | null): SourceSize | null => {
  const width = optInt(size, 'width');
  const height = optInt(size, 'height');
  return size !== null && width > 0 && height > 0 ? { width, height } : null;
};

export const videoReading = (view: Json | null): VideoReading | null => {
  if (view === null || optText(view, 'class') !== 'Video') return null;
  return VideoReading({
    available: optBoolean(view, 'available'),
    decoding: optBoolean(view, 'decoding'),
    sourceSize: sourceSizeOf(optJSONObject(view, 'sourceSize')),
    summary: optText(view, 'summary'),
    activeSource: optInt(view, 'activeSource'),
    streaming: optBoolean(view, 'streaming'),
    streamEnabled: optBoolean(view, 'streamEnabled', true),
    noVideoText: optText(view, 'noVideoText'),
    noVideoReason: optText(view, 'noVideoReason'),
    sourceChosen: optBoolean(view, 'sourceChosen', true),
    standIn: isNull(view, 'standIn') ? null : optString(view, 'standIn'),
    cameras: jsonObjects(optJSONArray(view, 'cameras')).map((camera) => ({
      slot: optInt(camera, 'slot'),
      status: optText(camera, 'status'),
      configured: optBoolean(camera, 'configured'),
    })),
  });
};

export const whepEndpoint = (view: Json | null, channel: number): string | null => {
  const endpoint = optJSONArray(view, 'whepEndpoints')?.[channel];
  return typeof endpoint === 'string' && endpoint !== '' ? endpoint : null;
};

export const nativeRecordingFile = (view: Json | null): string | null => {
  const file = optText(optJSONObject(view, 'nativeRecording'), 'file');
  return file.trim() === '' ? null : file;
};

export const whepRestarts = (view: Json | null, channel: number): number => {
  const restarts = optJSONArray(view, 'whepRestarts')?.[channel];
  return typeof restarts === 'number' ? restarts : 0;
};

export type PaintedRect = { left: number; top: number; width: number; height: number };

export const paintedRect = (surfaceWidth: number, surfaceHeight: number, source: SourceSize | null): PaintedRect => {
  if (source === null || surfaceWidth <= 0 || surfaceHeight <= 0) return { left: 0, top: 0, width: surfaceWidth, height: surfaceHeight };
  const scale = Math.min(surfaceWidth / source.width, surfaceHeight / source.height);
  const width = source.width * scale;
  const height = source.height * scale;
  return { left: (surfaceWidth - width) / 2, top: (surfaceHeight - height) / 2, width, height };
};
