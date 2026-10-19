import { type Json, isJson, jsonObjects, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
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
  readonly schemes: readonly Json[];
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
    schemes: jsonObjects(optJSONArray(json, 'schemes')),
    gimbal: parseGimbal(json.gimbal),
  };
};

export type EndpointRow = { readonly scheme: string; readonly host: string; readonly port: string };

export type EndpointOption = DeviceOption & { readonly row: EndpointRow };

export type EndpointRowView = {
  readonly row: EndpointRow;
  readonly hostLabel: string;
  readonly placeholder: string;
  readonly note: string;
  readonly problem: string;
  readonly options: readonly EndpointOption[];
};

export type EndpointEditor = { readonly rows: readonly EndpointRowView[]; readonly blank: EndpointRowView; readonly endpoints: string };

export const TELEMETRY_ENDPOINTS = 'deviceTelemetry.endpoints';

const parseRow = (json: Json | null): EndpointRow => ({ scheme: optText(json, 'scheme'), host: optText(json, 'host'), port: optText(json, 'port') });

const parseRowView = (json: Json | null): EndpointRowView => ({
  row: parseRow(json),
  hostLabel: optText(json, 'hostLabel'),
  placeholder: optText(json, 'placeholder'),
  note: optText(json, 'note'),
  problem: optText(json, 'problem'),
  options: jsonObjects(optJSONArray(json, 'options')).map((option) => ({ value: optText(option, 'value'), label: optText(option, 'label'), row: parseRow(optJSONObject(option, 'row')) })),
});

export const parseEndpointEditor = (answer: Json | null): EndpointEditor | null => {
  const result = optJSONObject(answer, 'result');
  return !optBoolean(answer, 'ok') || result === null
    ? null
    : { rows: jsonObjects(optJSONArray(result, 'rows')).map(parseRowView), blank: parseRowView(optJSONObject(result, 'blank')), endpoints: optText(result, 'endpoints') };
};

export const endpointEditor = async (schemes: readonly Json[], rows: readonly EndpointRow[] | null, stored: string): Promise<EndpointEditor | null> =>
  parseEndpointEditor(await Qgc.call(TELEMETRY_ENDPOINTS, schemes, rows ?? stored));
