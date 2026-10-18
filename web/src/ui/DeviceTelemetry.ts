import { isJson, jsonObjects, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, type DeviceOption, parseBadge, parseBadges, parseOptions } from '@/ui/DeviceKit';

export type DeviceEndpoint = {
  readonly proto: string;
  readonly label: string;
  readonly url: string;
  readonly hint: string;
  readonly note: string;
  readonly appLink: string;
  readonly down: boolean;
  readonly primary: boolean;
};

export type MavlinkScheme = { readonly value: string; readonly label: string; readonly host: string; readonly port: string; readonly hostLabel: string; readonly placeholder: string; readonly hint: string };

export type GimbalSettings = {
  readonly title: string;
  readonly status: DeviceBadge;
  readonly about: string;
  readonly vendor: string;
  readonly model: string;
  readonly cameraAddr: string;
  readonly mavlinkAddr: string;
  readonly vendors: readonly DeviceOption[];
  readonly models: readonly DeviceOption[];
};

export type TelemetryView = {
  readonly status: DeviceBadge;
  readonly notices: readonly DeviceBadge[];
  readonly endpoints: readonly DeviceEndpoint[];
  readonly endpointNotice: DeviceBadge | null;
  readonly cloud: { readonly status: DeviceBadge; readonly text: string } | null;
  readonly settings: { readonly endpoints: string; readonly baud: string };
  readonly schemes: readonly MavlinkScheme[];
  readonly gimbal: GimbalSettings | null;
};

export const parseEndpointList = (json: unknown, key: string): readonly DeviceEndpoint[] =>
  jsonObjects(optJSONArray(isJson(json) ? json : null, key)).map((it) => ({
    proto: optText(it, 'proto'),
    label: optText(it, 'label'),
    url: optText(it, 'url'),
    hint: optText(it, 'hint'),
    note: optText(it, 'note'),
    appLink: optText(it, 'appLink'),
    down: optBoolean(it, 'down'),
    primary: optBoolean(it, 'primary'),
  }));

const parseGimbal = (json: unknown): GimbalSettings | null =>
  isJson(json)
    ? {
        title: optText(json, 'title'),
        status: parseBadge(json.status),
        about: optText(json, 'about'),
        vendor: optText(json, 'vendor'),
        model: optText(json, 'model'),
        cameraAddr: optText(json, 'cameraAddr'),
        mavlinkAddr: optText(json, 'mavlinkAddr'),
        vendors: parseOptions(json, 'vendors'),
        models: parseOptions(json, 'models'),
      }
    : null;

export const parseTelemetry = (json: unknown): TelemetryView | null => {
  if (!isJson(json) || !isJson(json.status)) return null;
  const cloud = optJSONObject(json, 'cloud');
  const settings = optJSONObject(json, 'settings');
  return {
    status: parseBadge(json.status),
    notices: parseBadges(json, 'notices'),
    endpoints: parseEndpointList(json, 'endpoints'),
    endpointNotice: isJson(json.endpointNotice) ? parseBadge(json.endpointNotice) : null,
    cloud: cloud === null ? null : { status: parseBadge(cloud.status), text: optText(cloud, 'text') },
    settings: { endpoints: optText(settings, 'endpoints'), baud: optText(settings, 'baud') },
    schemes: jsonObjects(optJSONArray(json, 'schemes')).map((it) => ({
      value: optText(it, 'value'),
      label: optText(it, 'label'),
      host: optText(it, 'host'),
      port: optText(it, 'port'),
      hostLabel: optText(it, 'hostLabel'),
      placeholder: optText(it, 'placeholder'),
      hint: optText(it, 'hint'),
    })),
    gimbal: parseGimbal(json.gimbal),
  };
};

export type EndpointRow = { readonly scheme: string; readonly host: string; readonly port: string };

export const UNKNOWN_SCHEME_HINT = 'Set outside this page — saved exactly as it stands unless you change the type.';

const MIN_PORT = 1;
const MAX_PORT = 65535;

const schemeOf = (schemes: readonly MavlinkScheme[], name: string): MavlinkScheme | null => schemes.find((it) => it.value === name) ?? null;

export const newRow = (schemes: readonly MavlinkScheme[], name = schemes[0]?.value ?? 'udps'): EndpointRow => ({
  scheme: name,
  host: schemeOf(schemes, name)?.host ?? '',
  port: schemeOf(schemes, name)?.port ?? '14550',
});

export const parseEndpointSpec = (spec: string): EndpointRow | null => {
  const cut = spec.indexOf(':');
  if (cut < 0) return null;
  const address = spec.slice(cut + 1);
  const last = address.lastIndexOf(':');
  return { scheme: spec.slice(0, cut), host: last < 0 ? address : address.slice(0, last), port: last < 0 ? '' : address.slice(last + 1) };
};

export const parseEndpointSpecs = (text: string): readonly EndpointRow[] =>
  text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line !== '')
    .map(parseEndpointSpec)
    .filter((row): row is EndpointRow => row !== null && row.scheme !== '');

export const schemeOptions = (schemes: readonly MavlinkScheme[], row: EndpointRow): readonly DeviceOption[] => [
  ...schemes.map((it) => ({ value: it.value, label: it.label })),
  ...(row.scheme === '' || schemeOf(schemes, row.scheme) !== null ? [] : [{ value: row.scheme, label: row.scheme }]),
];

export const retargetRow = (schemes: readonly MavlinkScheme[], row: EndpointRow): EndpointRow => {
  const defaults = new Set(schemes.map((it) => it.host).filter((host) => host !== ''));
  const host = row.host.trim();
  return {
    ...row,
    host: host === '' || defaults.has(host) ? (schemeOf(schemes, row.scheme)?.host ?? '') : row.host,
    port: row.port.trim() === '' ? (schemeOf(schemes, row.scheme)?.port ?? '14550') : row.port,
  };
};

const portInRange = (port: string): boolean => /^\d+$/.test(port) && Number(port) >= MIN_PORT && Number(port) <= MAX_PORT;

export const endpointError = (schemes: readonly MavlinkScheme[], row: EndpointRow): string => {
  const host = row.host.trim();
  const port = row.port.trim();
  if (host === '' && port === '') return '';
  if (!portInRange(port)) return `Needs a port between ${MIN_PORT} and ${MAX_PORT}.`;
  if (host === '' && (schemeOf(schemes, row.scheme)?.host ?? '') === '') return 'Needs an address.';
  return '';
};

export const serializeEndpoints = (schemes: readonly MavlinkScheme[], rows: readonly EndpointRow[]): string =>
  rows
    .filter((row) => endpointError(schemes, row) === '' && (row.host.trim() !== '' || row.port.trim() !== ''))
    .map((row) => `${row.scheme}:${row.host.trim() || (schemeOf(schemes, row.scheme)?.host ?? '')}:${row.port.trim()}`)
    .join('\n');

export const describeEndpoint = (schemes: readonly MavlinkScheme[], row: EndpointRow): string => schemeOf(schemes, row.scheme)?.hint ?? UNKNOWN_SCHEME_HINT;

export const hostLabel = (schemes: readonly MavlinkScheme[], row: EndpointRow): string => schemeOf(schemes, row.scheme)?.hostLabel ?? 'Address';

export const hostPlaceholder = (schemes: readonly MavlinkScheme[], row: EndpointRow): string => schemeOf(schemes, row.scheme)?.placeholder ?? '';
