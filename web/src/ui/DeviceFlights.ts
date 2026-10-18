import { type Json, isJson, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, parseBadge } from '@/ui/DeviceKit';
import { readStored, writeStored } from '@/ui/DeviceConnect';
import { deviceUrl } from '@/ui/DeviceView';
import { DeviceLevel } from '@/ui/DeviceLevel';

export type FlightLine = { readonly index: number; readonly clock: string; readonly text: string; readonly level: number; readonly alert: boolean; readonly repeats: string; readonly heldBack: boolean };

export type SortieShare = { readonly label: string; readonly confirm: string; readonly camera: string; readonly from: string; readonly to: string };

export type SortieVideo = { readonly camera: string; readonly label: string; readonly url: string; readonly share: SortieShare | null };

export type FlightPart = { readonly title: string; readonly span: string; readonly videos: readonly SortieVideo[]; readonly lines: readonly FlightLine[] };

export type TraceLane = {
  readonly key: string;
  readonly label: string;
  readonly chip: string;
  readonly values: readonly number[];
  readonly notes: readonly string[];
  readonly unit: string;
  readonly max: number;
  readonly default: boolean;
  readonly peak: string;
};

export type TraceMarker = { readonly offset: number; readonly label: string; readonly level: number };

export type TraceSpan = { readonly label: string; readonly from: number; readonly to: number };

export type FlightTrace = {
  readonly step: number;
  readonly startUnix: number;
  readonly tzOffset: number;
  readonly lanes: readonly TraceLane[];
  readonly markers: readonly TraceMarker[];
  readonly spans: readonly TraceSpan[];
};

export type FlightEntry = {
  readonly id: string;
  readonly title: string;
  readonly summary: string;
  readonly badge: DeviceBadge;
  readonly open: boolean;
  readonly sections: readonly FlightPart[];
  readonly trace: FlightTrace | null;
  readonly dropped: string;
  readonly csv: string;
  readonly json: string;
};

export type FlightsView = { readonly flights: readonly FlightEntry[] };

const numbers = (json: Json | null, key: string): readonly number[] => (optJSONArray(json, key) ?? []).map((value) => (typeof value === 'number' ? value : -1));

const parseShare = (json: Json | null): SortieShare | null =>
  json === null ? null : { label: optText(json, 'label'), confirm: optText(json, 'confirm'), camera: optText(json, 'camera'), from: optText(json, 'from'), to: optText(json, 'to') };

const parsePart = (json: Json): FlightPart => ({
  title: optText(json, 'title'),
  span: optText(json, 'span'),
  videos: jsonObjects(optJSONArray(json, 'videos')).map((it) => ({ camera: optText(it, 'camera'), label: optText(it, 'label'), url: optText(it, 'url'), share: parseShare(optJSONObject(it, 'share')) })),
  lines: jsonObjects(optJSONArray(json, 'lines')).map((it) => ({
    index: optInt(it, 'index'),
    clock: optText(it, 'clock'),
    text: optText(it, 'text'),
    level: optInt(it, 'level', DeviceLevel.Neutral),
    alert: optBoolean(it, 'alert'),
    repeats: optText(it, 'repeats'),
    heldBack: optBoolean(it, 'heldBack'),
  })),
});

const parseTrace = (json: Json | null): FlightTrace | null =>
  json === null
    ? null
    : {
        step: optInt(json, 'step'),
        startUnix: optInt(json, 'startUnix'),
        tzOffset: optInt(json, 'tzOffset'),
        lanes: jsonObjects(optJSONArray(json, 'lanes')).map((it) => ({
          key: optText(it, 'key'),
          label: optText(it, 'label'),
          chip: optText(it, 'chip'),
          values: numbers(it, 'values'),
          notes: jsonStrings(optJSONArray(it, 'notes')),
          unit: optText(it, 'unit'),
          max: optDouble(it, 'max', 0),
          default: optBoolean(it, 'default'),
          peak: optText(it, 'peak'),
        })),
        markers: jsonObjects(optJSONArray(json, 'markers')).map((it) => ({ offset: optInt(it, 'offset'), label: optText(it, 'label'), level: optInt(it, 'level', DeviceLevel.Neutral) })),
        spans: jsonObjects(optJSONArray(json, 'spans')).map((it) => ({ label: optText(it, 'label'), from: optDouble(it, 'from', 0), to: optDouble(it, 'to', 1) })),
      };

export const parseFlights = (json: unknown): FlightsView | null =>
  isJson(json) && Array.isArray(json.flights)
    ? {
        flights: jsonObjects(json.flights).map((it) => ({
          id: optText(it, 'id'),
          title: optText(it, 'title'),
          summary: optText(it, 'summary'),
          badge: parseBadge(it.badge),
          open: optBoolean(it, 'open'),
          sections: jsonObjects(optJSONArray(it, 'sections')).map(parsePart),
          trace: parseTrace(optJSONObject(it, 'trace')),
          dropped: optText(it, 'dropped'),
          csv: optText(it, 'csv'),
          json: optText(it, 'json'),
        })),
      }
    : null;

export const hasLines = (flight: FlightEntry): boolean => flight.sections.some((part) => part.lines.length > 0);

export const flightLink = (base: string, id: string): string => `${base}#device=${encodeURIComponent(`/flights?flight=${id}`)}`;

export const LANES_KEY = 'flight-lanes';

export const laneKeys = (saved: readonly string[] | null, defaults: readonly string[]): readonly string[] => saved ?? defaults;

export const toggled = (keys: readonly string[], key: string): readonly string[] => (keys.includes(key) ? keys.filter((it) => it !== key) : [...keys, key]);

export const storedLanes = (): readonly string[] | null => {
  const raw = readStored(LANES_KEY);
  return raw === null ? null : raw.split(',').filter((key) => key !== '');
};

export const rememberLanes = (keys: readonly string[]): void => writeStored(LANES_KEY, keys.join(','));

export type Zoom = readonly [number, number];

export const PLOT_TOP = 1;
export const PLOT_HEIGHT = 18;
export const PLOT_BOTTOM = PLOT_TOP + PLOT_HEIGHT;
export const PLOT_BOX = 20;

export const ceilingOf = (values: readonly number[], max: number): number => {
  const peak = values.reduce((top, value) => Math.max(top, value), 0);
  return max > 0 ? max : peak > 0 ? peak : 1;
};

export const segments = (values: readonly number[], max: number): readonly string[] => {
  const ceiling = ceilingOf(values, max);
  const step = values.length > 1 ? 100 / (values.length - 1) : 100;
  return values
    .reduce<readonly (readonly string[])[]>(
      (runs, value, index) =>
        value < 0 ? [...runs, []] : [...runs.slice(0, -1), [...(runs.at(-1) ?? []), `${(index * step).toFixed(1)},${(PLOT_BOTTOM - (PLOT_HEIGHT * Math.min(value, ceiling)) / ceiling).toFixed(1)}`]],
      [[]],
    )
    .filter((run) => run.length > 1)
    .map((run) => run.join(' '));
};

export const formatRate = (bytesPerSecond: number): string => {
  const bits = bytesPerSecond * 8;
  return bits >= 1e6 ? `${(bits / 1e6).toFixed(1)} Mbps` : bits >= 1e3 ? `${Math.round(bits / 1e3)} kbps` : `${Math.round(bits)} bps`;
};

const UNIT_FORMATS: Readonly<Record<string, (value: number) => string>> = {
  rate: formatRate,
  meters: (value) => (value >= 1000 ? `${(value / 1000).toFixed(1)} km` : `${Math.round(value)} m`),
  count: (value) => `${Math.round(value)}`,
  mb: (value) => (value >= 1024 ? `${(value / 1024).toFixed(1)} GB` : `${Math.round(value)} MB`),
  seconds: (value) => (value >= 60 ? `${Math.round(value / 60)} m` : `${Math.round(value)} s`),
  number: (value) => `${Math.round(value * 10) / 10}`,
};

export const formatValue = (value: number, unit: string): string => (UNIT_FORMATS[unit] ?? ((it: number) => `${Math.round(it)}%`))(value);

export const ticks = (startUnix: number, step: number, count: number, clock: (unix: number) => string): readonly { x: number; label: string }[] => {
  if (!(count > 1) || !(step > 0)) return [];
  const span = step * (count - 1);
  const want = Math.min(5, Math.max(2, Math.floor(span / 60) + 1));
  return Array.from({ length: want }, (_, i) => ({ x: (i / (want - 1)) * 100, label: clock(startUnix + (span * i) / (want - 1)) }));
};

export type MarkerLine = { readonly x: number; readonly label: string; readonly level: number; readonly index: number };

export const markerLines = (markers: readonly TraceMarker[], step: number, count: number, shift = 0): readonly MarkerLine[] => {
  const span = step * (count - 1);
  return span > 0
    ? markers
        .map((marker, index) => ({ ...marker, offset: marker.offset - shift, index }))
        .filter((marker) => marker.offset >= 0 && marker.offset <= span)
        .map((marker) => ({ x: (marker.offset / span) * 100, label: marker.label, level: marker.level, index: marker.index }))
    : [];
};

export const zoomSlice = (count: number, zoom: Zoom | null): { readonly from: number; readonly to: number } => {
  if (zoom === null || count < 2) return { from: 0, to: count - 1 };
  const from = Math.max(0, Math.min(count - 2, Math.round(zoom[0] * (count - 1))));
  return { from, to: Math.max(from + 1, Math.min(count - 1, Math.round(zoom[1] * (count - 1)))) };
};

export const composeZoom = (current: Zoom | null, a: number, b: number): Zoom => {
  const low = Math.min(a, b);
  const high = Math.max(a, b);
  if (current === null) return [low, high];
  const span = current[1] - current[0];
  return [current[0] + low * span, current[0] + high * span];
};

export const MIN_BRUSH = 0.02;

export const spanLabel = (seconds: number): string => (seconds >= 3600 ? `${Math.floor(seconds / 3600)}h ${Math.round((seconds % 3600) / 60)}m` : `${Math.round(seconds / 60)}m`);

const two = (n: number): string => String(n).padStart(2, '0');

export const clockAt = (unix: number, tzOffset: number, withSeconds: boolean): string => {
  const at = new Date((unix + tzOffset) * 1000);
  const base = `${two(at.getUTCHours())}:${two(at.getUTCMinutes())}`;
  return withSeconds ? `${base}:${two(at.getUTCSeconds())}` : base;
};

export const gridLines = (ceiling: number, unit: string): readonly { y: number; label: string }[] =>
  ceiling > 0 ? [0.25, 0.5, 0.75].map((fraction) => ({ y: PLOT_BOTTOM - PLOT_HEIGHT * fraction, label: formatValue(ceiling * fraction, unit) })) : [];

export const readoutAt = (values: readonly number[], notes: readonly string[], fraction: number, unit: string): { index: number; x: number; text: string } | null => {
  if (values.length === 0) return null;
  const index = Math.max(0, Math.min(values.length - 1, Math.round(fraction * (values.length - 1))));
  const value = values[index] ?? -1;
  const note = notes[index] ?? '';
  return { index, x: values.length > 1 ? (index / (values.length - 1)) * 100 : 0, text: `${value < 0 ? 'no reading' : formatValue(value, unit)}${note !== '' ? ` · ${note}` : ''}` };
};

export type RenderJob = { readonly id: string; readonly state: string; readonly sent: number; readonly total: number; readonly url: string; readonly expiresAt: string; readonly error: string };

export const RENDER_POLL_MS = 2000;

export const IDLE_RENDER: RenderJob = { id: '', state: '', sent: 0, total: 0, url: '', expiresAt: '', error: '' };

export const parseRenderJob = (json: unknown): RenderJob => {
  const it = isJson(json) ? json : null;
  const state = optText(it, 'state');
  return {
    id: optText(it, 'id'),
    state,
    sent: optInt(it, 'sent'),
    total: optInt(it, 'total'),
    url: optText(it, 'url'),
    expiresAt: optText(it, 'expiresAt'),
    error: state === 'failed' ? optText(it, 'error') || 'The cloud could not render this video.' : '',
  };
};

const failedRender = (error: string): RenderJob => ({ ...IDLE_RENDER, state: 'failed', error });

export const renderRunning = (job: RenderJob): boolean => job.state === 'uploading' || job.state === 'rendering';

export const renderPercent = (job: RenderJob): number => (job.total > 0 ? Math.floor((job.sent / job.total) * 100) : 0);

export const renderExpiry = (job: RenderJob): string =>
  job.expiresAt === '' ? '' : `Link works until ${new Date(job.expiresAt).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}`;

export const startRender = async (share: SortieShare): Promise<RenderJob> => {
  const response = await fetch(deviceUrl('/api/renders'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({ camera: share.camera, from: share.from, to: share.to }),
  }).catch(() => null);
  const body: unknown = await response?.json().catch(() => null);
  return response?.ok ? parseRenderJob(body) : failedRender(optText(isJson(body) ? body : null, 'error') || 'Could not send the video.');
};

export const readRender = async (id: string, signal: AbortSignal): Promise<RenderJob> => {
  const response = await fetch(deviceUrl(`/api/renders/${encodeURIComponent(id)}`), { headers: { Accept: 'application/json' }, signal }).catch(() => null);
  return response?.ok ? parseRenderJob(await response.json().catch(() => null)) : failedRender('Lost track of the render.');
};
