import { type Json, isJson, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { type DeviceBadge, parseBadge } from '@/ui/DeviceKit';
import { readStored, writeStored } from '@/ui/DeviceConnect';
import { DeviceLevel } from '@/ui/DeviceLevel';

export type FlightLine = { readonly index: number; readonly clock: string; readonly text: string; readonly level: number; readonly alert: boolean; readonly repeats: string; readonly heldBack: boolean };

export type SortieShare = { readonly label: string; readonly confirm: string; readonly camera: string; readonly from: string; readonly to: string };

export type SortieVideo = { readonly camera: string; readonly label: string; readonly url: string; readonly share: SortieShare | null };

export type FlightPart = { readonly title: string; readonly span: string; readonly videos: readonly SortieVideo[]; readonly lines: readonly FlightLine[] };

export type FlightEntry = {
  readonly id: string;
  readonly title: string;
  readonly summary: string;
  readonly badge: DeviceBadge;
  readonly open: boolean;
  readonly sections: readonly FlightPart[];
  readonly trace: Json | null;
  readonly dropped: string;
  readonly csv: string;
  readonly json: string;
};

export type FlightsView = { readonly flights: readonly FlightEntry[] };

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
          trace: optJSONObject(it, 'trace'),
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

export const composeZoom = (current: Zoom | null, a: number, b: number): Zoom => {
  const low = Math.min(a, b);
  const high = Math.max(a, b);
  if (current === null) return [low, high];
  const span = current[1] - current[0];
  return [current[0] + low * span, current[0] + high * span];
};

export const MIN_BRUSH = 0.02;

export type ChartGrid = { readonly y: number; readonly label: string };

export type ChartTick = { readonly x: number; readonly label: string };

export type ChartMarker = { readonly x: number; readonly label: string; readonly level: number; readonly index: number };

export type ChartLane = {
  readonly key: string;
  readonly label: string;
  readonly chip: string;
  readonly peak: string;
  readonly default: boolean;
  readonly lines: readonly string[];
  readonly grid: readonly ChartGrid[];
  readonly markers: readonly ChartMarker[];
  readonly readings: readonly string[];
  readonly ticks: readonly ChartTick[];
  readonly span: string;
};

export type ChartSpan = { readonly label: string; readonly from: number; readonly to: number };

export type FlightChart = { readonly lanes: readonly ChartLane[]; readonly spans: readonly ChartSpan[] };

const FLIGHT_CHART = 'deviceFlight.chart';

const polyline = (run: unknown): string => (Array.isArray(run) ? run.map((point) => jsonStrings(Array.isArray(point) ? point : []).join(',')).join(' ') : '');

export const parseFlightChart = (json: unknown): FlightChart | null =>
  isJson(json)
    ? {
        lanes: jsonObjects(optJSONArray(json, 'lanes')).map((it) => ({
          key: optText(it, 'key'),
          label: optText(it, 'label'),
          chip: optText(it, 'chip'),
          peak: optText(it, 'peak'),
          default: optBoolean(it, 'default'),
          lines: (optJSONArray(it, 'lines') ?? []).map(polyline),
          grid: jsonObjects(optJSONArray(it, 'grid')).map((line) => ({ y: optDouble(line, 'y', 0), label: optText(line, 'label') })),
          markers: jsonObjects(optJSONArray(it, 'markers')).map((marker) => ({ x: optDouble(marker, 'x', 0), label: optText(marker, 'label'), level: optInt(marker, 'level', DeviceLevel.Neutral), index: optInt(marker, 'index') })),
          readings: jsonStrings(optJSONArray(it, 'readings')),
          ticks: jsonObjects(optJSONArray(it, 'ticks')).map((tick) => ({ x: optDouble(tick, 'x', 0), label: optText(tick, 'label') })),
          span: optText(it, 'span'),
        })),
        spans: jsonObjects(optJSONArray(json, 'spans')).map((it) => ({ label: optText(it, 'label'), from: optDouble(it, 'from', 0), to: optDouble(it, 'to', 1) })),
      }
    : null;

export const flightChart = async (trace: Json, zoom: Zoom | null): Promise<FlightChart | null> => parseFlightChart(await Qgc.invokeResult(FLIGHT_CHART, trace, ...(zoom ?? [])));

export const sampleAt = (count: number, fraction: number): { readonly index: number; readonly x: number } | null => {
  if (count === 0) return null;
  const index = Math.max(0, Math.min(count - 1, Math.round(fraction * (count - 1))));
  return { index, x: count > 1 ? (index / (count - 1)) * 100 : 0 };
};

export type FlightRender = { readonly canStart: boolean; readonly progress: string; readonly error: string; readonly url: string; readonly expiry: string };

const FLIGHT_RENDER = 'deviceFlight.render';

export const renderPath = (share: SortieShare): string => `view.deviceRender(${share.camera},${share.from},${share.to})`;

export const parseRender = (json: Json | null): FlightRender => ({
  canStart: optBoolean(json, 'canStart', true),
  progress: optText(json, 'progress'),
  error: optText(json, 'error'),
  url: optText(json, 'url'),
  expiry: optText(json, 'expiry'),
});

export const requestRender = (share: SortieShare): Promise<boolean> => Qgc.invoke(FLIGHT_RENDER, share.camera, share.from, share.to);
