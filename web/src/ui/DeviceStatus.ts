import { type Json, isJson, jsonObjects, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { mutableStateOf } from '@/kotlin/State';
import type { DrawableName } from '@/res/drawables';

export const DeviceLevel = { Ready: 0, Warn: 1, Blocked: 2 } as const;

export type DeviceAction = { readonly label: string; readonly href: string };
export type DeviceCheck = { readonly label: string; readonly value: string; readonly level: number; readonly href: string; readonly action: string };
export type DevicePage = { readonly title: string; readonly href: string; readonly icon: string; readonly summary: string; readonly needs: boolean };
export type DeviceStatus = {
  readonly name: string;
  readonly level: number;
  readonly headline: string;
  readonly detail: string;
  readonly action: DeviceAction | null;
  readonly checks: readonly DeviceCheck[];
  readonly pages: readonly DevicePage[];
};

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

export const DEVICE_PAGE = 'Aircast device';

export const deviceStatus = mutableStateOf<DeviceStatus | null>(null);

export const deviceAnswered = mutableStateOf(false);

const STATUS_EVERY_MS = 5000;

const readDeviceStatus = async (): Promise<DeviceStatus | null> => {
  const response = await fetch('aircraft.json', { cache: 'no-store' }).catch(() => null);
  return response?.ok ? parseDeviceStatus(await response.json().catch(() => null)) : null;
};

export const followDeviceStatus = (): void =>
  void readDeviceStatus().then((status) => {
    deviceStatus.value = status;
    if (status !== null) deviceAnswered.value = true;
    setTimeout(followDeviceStatus, STATUS_EVERY_MS);
  });
