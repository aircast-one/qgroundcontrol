import { type Json, isJson, jsonObjects, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, type DeviceChartView, parseBadge, parseChart } from '@/ui/DeviceKit';
import { mutableStateOf } from '@/kotlin/State';
import type { DrawableName } from '@/res/drawables';
import { followDeviceView } from '@/ui/DeviceView';

export const DeviceLevel = { Ready: 0, Warn: 1, Blocked: 2, Neutral: 3 } as const;

export type DeviceAction = { readonly label: string; readonly href: string };
export type DeviceCheck = { readonly label: string; readonly value: string; readonly level: number; readonly href: string; readonly action: string };
export type DevicePage = { readonly title: string; readonly href: string; readonly icon: string; readonly summary: string; readonly needs: boolean };
export type DeviceUplink = {
  readonly carrier: string;
  readonly backup: string;
  readonly rate: string;
  readonly network: string;
  readonly standby: string;
  readonly trouble: string;
  readonly data: string;
  readonly charts: readonly DeviceChartView[];
};

export type DeviceHealth = { readonly memory: string; readonly warning: DeviceBadge | null; readonly charts: readonly DeviceChartView[] };

export type DeviceStatus = {
  readonly name: string;
  readonly level: number;
  readonly headline: string;
  readonly detail: string;
  readonly action: DeviceAction | null;
  readonly checks: readonly DeviceCheck[];
  readonly pages: readonly DevicePage[];
  readonly link: DeviceUplink | null;
  readonly health: DeviceHealth | null;
  readonly events: string;
};

const parseCharts = (json: Json | null): readonly DeviceChartView[] => jsonObjects(optJSONArray(json, 'charts')).flatMap((chart) => {
  const parsed = parseChart(chart);
  return parsed === null ? [] : [parsed];
});

const parseUplink = (json: Json | null): DeviceUplink | null =>
  json === null
    ? null
    : {
        carrier: optText(json, 'carrier'),
        backup: optText(json, 'backup'),
        rate: optText(json, 'rate'),
        network: optText(json, 'network'),
        standby: optText(json, 'standby'),
        trouble: optText(json, 'trouble'),
        data: optText(json, 'data'),
        charts: parseCharts(json),
      };

const parseHealth = (json: Json | null): DeviceHealth | null =>
  json === null ? null : { memory: optText(json, 'memory'), warning: isJson(json.warning) ? parseBadge(json.warning) : null, charts: parseCharts(json) };

const deviceAction = (json: Json | null): DeviceAction | null => {
  const href = optText(json, 'href');
  return href === '' ? null : { label: optText(json, 'label'), href };
};

export const parseDeviceStatus = (json: unknown): DeviceStatus | null =>
  isJson(json) && optText(json, 'headline') !== ''
    ? {
        name: optText(json, 'name'),
        level: optInt(json, 'level'),
        headline: optText(json, 'headline'),
        detail: optText(json, 'detail'),
        action: deviceAction(optJSONObject(json, 'action')),
        checks: jsonObjects(optJSONArray(json, 'checks')).map((check) => ({
          label: optText(check, 'label'),
          value: optText(check, 'value'),
          level: optInt(check, 'level'),
          href: optText(check, 'href'),
          action: optText(check, 'action'),
        })),
        pages: jsonObjects(optJSONArray(json, 'pages')).map((page) => ({
          title: optText(page, 'title'),
          href: optText(page, 'href'),
          icon: optText(page, 'icon'),
          summary: optText(page, 'summary'),
          needs: optBoolean(page, 'needs'),
        })),
        link: parseUplink(optJSONObject(json, 'link')),
        health: parseHealth(optJSONObject(json, 'health')),
        events: optText(json, 'events'),
      }
    : null;

const PAGE_ICONS: ReadonlyMap<string, DrawableName> = new Map<string, DrawableName>([
  ['qr-code', 'ic_link'],
  ['video', 'ic_videocam'],
  ['sliders-horizontal', 'ic_tune'],
  ['network', 'ic_lan'],
  ['activity', 'ic_analytics'],
  ['gauge', 'ic_stop_circle'],
  ['clock', 'ic_history'],
  ['settings', 'ic_developer_board'],
  ['server', 'ic_terminal'],
]);

export const devicePageIcon = (icon: string): DrawableName => PAGE_ICONS.get(icon) ?? 'ic_settings';

export const deviceNeedsAttention = (status: DeviceStatus | null): boolean => status !== null && status.level !== DeviceLevel.Ready;

export const DEVICE_HOME = 'aircast:home';


export const deviceStatus = mutableStateOf<DeviceStatus | null>(null);

export const deviceAnswered = mutableStateOf(false);

export const deviceRequested = mutableStateOf<string | null>(null);

export const followDeviceStatus = (): (() => void) =>
  followDeviceView('aircraft', parseDeviceStatus, (status) => {
    deviceStatus.value = status;
    deviceAnswered.value = true;
  });
