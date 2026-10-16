import type { Json } from '@/bridge/Json';

export const CAMERAS_VIEW = 'view.cameras';
export const VIDEO_SOURCES_PAGE = 'Video sources';

export type CameraStatus = { readonly name: 'Live' | 'Connecting' | 'NoSignal' | 'Idle'; readonly ordinal: number; readonly token: string; readonly label: string };

export const CameraStatus = {
  Live: { name: 'Live', ordinal: 0, token: 'live', label: 'Live' },
  Connecting: { name: 'Connecting', ordinal: 1, token: 'connecting', label: 'Connecting' },
  NoSignal: { name: 'NoSignal', ordinal: 2, token: 'noSignal', label: 'No signal' },
  Idle: { name: 'Idle', ordinal: 3, token: 'idle', label: 'Not playing' },
} as const satisfies Record<string, CameraStatus>;

export type CameraKind = { raw: string; label: string; group: string; needsUrl: boolean; hint: string; description: string };

export type CameraEntry = {
  slot: number;
  stored: number | null;
  title: string;
  short: string;
  name: string;
  source: string;
  url: string;
  tilt: number | null;
  problem: string | null;
  fromDrone: boolean;
  active: boolean;
  status: CameraStatus;
};

export type CameraPip = { enabled: boolean; slot: number | null };

export type CamerasReading = { readable: boolean; reason: string; pip: CameraPip; cameras: readonly CameraEntry[]; kinds: readonly CameraKind[] };

export const camerasReading = (_view: Json | null): CamerasReading | null => null;

export const camerasGlance = (reading: CamerasReading | null): string => {
  const cameras = reading?.cameras ?? [];
  const active = cameras.find((it) => it.active) ?? cameras[0] ?? null;
  return active === null ? '' : cameras.length === 1 ? active.title : `${active.title} · ${cameras.length} cameras`;
};
