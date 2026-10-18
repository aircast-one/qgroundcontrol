import { isJson, jsonObjects, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, type DeviceOption, parseBadge, parseOptions } from '@/ui/DeviceKit';

export type RecordingsLimit = { readonly value: string; readonly options: readonly DeviceOption[]; readonly used: string };

export type RecordCamera = { readonly name: string; readonly recording: boolean; readonly note: string };

export type RecordingSegment = {
  readonly at: string;
  readonly started: string;
  readonly duration: string;
  readonly size: string;
  readonly url: string;
  readonly open: boolean;
  readonly noTelemetry: boolean;
};

export type RecordedCamera = {
  readonly name: string;
  readonly status: DeviceBadge;
  readonly summary: string;
  readonly full: boolean;
  readonly usage: string;
  readonly segments: readonly RecordingSegment[];
};

export type RecordingsView = { readonly limit: RecordingsLimit; readonly cameras: readonly RecordCamera[]; readonly recorded: readonly RecordedCamera[] };

export const parseRecordings = (json: unknown): RecordingsView | null => {
  if (!isJson(json) || !isJson(json.limit)) return null;
  return {
    limit: { value: optText(json.limit, 'value'), options: parseOptions(json.limit, 'options'), used: optText(json.limit, 'used') },
    cameras: jsonObjects(optJSONArray(json, 'cameras')).map((camera) => ({ name: optText(camera, 'name'), recording: optBoolean(camera, 'recording'), note: optText(camera, 'note') })),
    recorded: jsonObjects(optJSONArray(json, 'recorded')).map((camera) => ({
      name: optText(camera, 'name'),
      status: parseBadge(optJSONObject(camera, 'status')),
      summary: optText(camera, 'summary'),
      full: optBoolean(camera, 'full'),
      usage: optText(camera, 'usage'),
      segments: jsonObjects(optJSONArray(camera, 'segments')).map((segment) => ({
        at: optText(segment, 'at'),
        started: optText(segment, 'started'),
        duration: optText(segment, 'duration'),
        size: optText(segment, 'size'),
        url: optText(segment, 'url'),
        open: optBoolean(segment, 'open'),
        noTelemetry: optBoolean(segment, 'noTelemetry'),
      })),
    })),
  };
};

export const lowersLimit = (current: string, picked: string): boolean => current !== '' && Number(picked) < Number(current);

export const recordPath = (camera: string): string => `/api/streams/${encodeURIComponent(camera)}/record`;

export const recordingsPath = (camera: string): string => `/api/recordings/${encodeURIComponent(camera)}`;

export const segmentPath = (camera: string, segment: RecordingSegment): string => `${recordingsPath(camera)}?start=${encodeURIComponent(segment.at)}`;

export const segmentDetail = (segment: RecordingSegment): readonly string[] => [
  ...(segment.duration !== '' ? [segment.duration] : []),
  segment.size,
  ...(segment.noTelemetry ? ['no flight data'] : []),
  ...(segment.open ? ['downloadable once it closes'] : []),
];
