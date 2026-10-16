import { type Json, isNull, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { sentenceCase } from '@/map/SentenceCase';
import { SYNTHETIC_LABEL, SYNTHETIC_SOURCE } from '@/ui/SyntheticView';

export const CAMERAS_VIEW = 'view.cameras';
export const VIDEO_SOURCES_PAGE = 'Video sources';
const CAMERA_GROUP_STREAMS = 'Video streams';
const CAMERA_GROUP_DEVICE = 'This device';
const CAMERA_GROUP_ORDER: readonly string[] = [CAMERA_GROUP_STREAMS, 'Vehicle and radio presets', CAMERA_GROUP_DEVICE];
const FROM_THE_DRONE = 'From the drone';

export type CameraStatus = { readonly name: 'Live' | 'Connecting' | 'NoSignal' | 'Idle'; readonly ordinal: number; readonly token: string; readonly label: string };

export const CameraStatus = {
  Live: { name: 'Live', ordinal: 0, token: 'live', label: 'Live' },
  Connecting: { name: 'Connecting', ordinal: 1, token: 'connecting', label: 'Connecting' },
  NoSignal: { name: 'NoSignal', ordinal: 2, token: 'noSignal', label: 'No signal' },
  Idle: { name: 'Idle', ordinal: 3, token: 'idle', label: 'Not playing' },
} as const satisfies Record<string, CameraStatus>;

export const CameraStatusEntries: readonly CameraStatus[] = [CameraStatus.Live, CameraStatus.Connecting, CameraStatus.NoSignal, CameraStatus.Idle];

export type CameraKind = { raw: string; label: string; group: string; needsUrl: boolean; hint: string; description: string; defaultAddress: string };

export const CameraKind = (fields: { raw: string; label: string; group: string; needsUrl: boolean; hint: string; description?: string; defaultAddress?: string }): CameraKind => ({
  description: '',
  defaultAddress: '',
  ...fields,
});

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

export type CamerasReading = {
  readable: boolean;
  reason: string;
  pip: CameraPip;
  cameras: readonly CameraEntry[];
  kinds: readonly CameraKind[];
  readonly stored: readonly CameraEntry[];
};

export const CamerasReading = (fields: Omit<CamerasReading, 'stored'>): CamerasReading => ({ ...fields, stored: fields.cameras.filter((it) => it.stored !== null) });

const objects = (json: Json, key: string): Json[] => jsonObjects(optJSONArray(json, key));

const textOrNull = (json: Json, key: string): string | null => {
  const text = isNull(json, key) ? '' : optText(json, key);
  return text.trim() === '' ? null : text;
};

const intOrNull = (json: Json, key: string): number | null => (isNull(json, key) ? null : optInt(json, key));

const doubleOrNull = (json: Json, key: string): number | null => {
  const value = isNull(json, key) ? NaN : optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

export const camerasReading = (view: Json | null): CamerasReading | null => {
  if (view === null || optText(view, 'class') !== 'Cameras') return null;
  const pip = optJSONObject(view, 'pip');
  return CamerasReading({
    readable: optBoolean(view, 'readable', true),
    reason: optText(view, 'reason'),
    pip: { enabled: pip !== null && optBoolean(pip, 'enabled'), slot: pip === null ? null : intOrNull(pip, 'slot') },
    cameras: objects(view, 'cameras').map((camera) => ({
      slot: optInt(camera, 'slot'),
      stored: intOrNull(camera, 'stored'),
      title: optText(camera, 'title'),
      short: optText(camera, 'short'),
      name: optText(camera, 'name'),
      source: optText(camera, 'source'),
      url: optText(camera, 'url'),
      tilt: doubleOrNull(camera, 'tilt'),
      problem: textOrNull(camera, 'problem'),
      fromDrone: optBoolean(camera, 'fromDrone'),
      active: optBoolean(camera, 'active'),
      status: CameraStatusEntries.find((it) => it.token === optText(camera, 'status')) ?? CameraStatus.Idle,
    })),
    kinds: objects(view, 'kinds').map((kind) =>
      CameraKind({
        raw: optText(kind, 'raw'),
        label: optText(kind, 'label'),
        group: optText(kind, 'group'),
        needsUrl: optBoolean(kind, 'needsUrl'),
        hint: optText(kind, 'hint'),
        description: optText(kind, 'description'),
        defaultAddress: optText(kind, 'defaultAddress'),
      }),
    ),
  });
};

export const cameraDetail = (camera: CameraEntry): string =>
  camera.fromDrone
    ? FROM_THE_DRONE
    : camera.url.includes('://')
      ? camera.url
      : camera.url.trim() !== ''
        ? `${kindLabel(camera.source)} · ${camera.url}`
        : kindLabel(camera.source);

const groupRank = (group: string): number => {
  const at = CAMERA_GROUP_ORDER.indexOf(group);
  return at < 0 ? CAMERA_GROUP_ORDER.length : at;
};

export const addableKinds = (reading: CamerasReading | null, keeping: string | null = null): readonly CameraKind[] =>
  (reading?.kinds ?? [])
    .filter((kind) => kind.needsUrl || kind.raw === keeping || !(reading?.stored ?? []).some((it) => it.source === kind.raw))
    .toSorted((a, b) => groupRank(a.group) - groupRank(b.group));

export const kindTitle = (kind: CameraKind): string => (kind.raw === SYNTHETIC_SOURCE ? SYNTHETIC_LABEL : kindLabel(kind.label));

export const addedName = (kind: CameraKind): string => (kind.raw === SYNTHETIC_SOURCE ? SYNTHETIC_LABEL : '');

export const kindLabel = (label: string): string => {
  const trimmed = label.endsWith(' Video Stream') ? label.slice(0, -' Video Stream'.length) : label;
  return sentenceCase(trimmed.trim() === '' ? label : trimmed);
};

export const camerasGlance = (reading: CamerasReading | null): string => {
  const cameras = reading?.cameras ?? [];
  const active = cameras.find((it) => it.active) ?? cameras[0] ?? null;
  return active === null ? '' : cameras.length === 1 ? active.title : `${active.title} · ${cameras.length} cameras`;
};
