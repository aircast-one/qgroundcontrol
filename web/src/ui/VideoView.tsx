import type { Json } from '@/bridge/Json';

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

export const videoReading = (_view: Json | null): VideoReading | null => null;
