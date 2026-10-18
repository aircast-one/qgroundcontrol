import { isJson, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, parseBadge } from '@/ui/DeviceKit';

export type DeviceStat = { readonly label: string; readonly value: string; readonly hint: string; readonly level: number };

export type ServiceRow = { readonly label: string; readonly state: string; readonly level: number };

export const CloudStage = { Linked: 'linked', SigningIn: 'signing-in', Connect: 'connect' } as const;

export type CloudAccount = {
  readonly status: DeviceBadge;
  readonly stage: string;
  readonly deviceId: string;
  readonly watchUrl: string;
  readonly userCode: string;
  readonly signInUrl: string;
  readonly error: string;
  readonly issuer: string;
};

export type SoftwareNotice = { readonly level: number; readonly title: string; readonly text: string; readonly detailLabel: string; readonly detail: string };

export type SoftwareCandidate = { readonly title: string; readonly notesUrl: string; readonly install: string; readonly confirm: string; readonly success: string };

export type SoftwareStatus = {
  readonly status: DeviceBadge;
  readonly summary: string;
  readonly notices: readonly SoftwareNotice[];
  readonly candidate: SoftwareCandidate | null;
  readonly notes: readonly string[];
  readonly checked: string;
  readonly canCheck: boolean;
};

export type DeviceSettingsView = {
  readonly health: DeviceBadge & { readonly href: string };
  readonly notice: DeviceBadge | null;
  readonly stats: readonly DeviceStat[];
  readonly services: readonly ServiceRow[];
  readonly cloud: CloudAccount;
  readonly software: SoftwareStatus;
  readonly hostname: string;
};

const parseCandidate = (json: unknown): SoftwareCandidate | null =>
  isJson(json)
    ? { title: optText(json, 'title'), notesUrl: optText(json, 'notesUrl'), install: optText(json, 'install'), confirm: optText(json, 'confirm'), success: optText(json, 'success') }
    : null;

const parseSoftware = (json: unknown): SoftwareStatus => {
  const it = isJson(json) ? json : null;
  return {
    status: parseBadge(it?.status),
    summary: optText(it, 'summary'),
    notices: jsonObjects(optJSONArray(it, 'notices')).map((notice) => ({
      level: optInt(notice, 'level'),
      title: optText(notice, 'title'),
      text: optText(notice, 'text'),
      detailLabel: optText(notice, 'detailLabel'),
      detail: optText(notice, 'detail'),
    })),
    candidate: parseCandidate(it?.candidate),
    notes: jsonStrings(optJSONArray(it, 'notes')),
    checked: optText(it, 'checked'),
    canCheck: optBoolean(it, 'canCheck'),
  };
};

const parseCloud = (json: unknown): CloudAccount => {
  const it = isJson(json) ? json : null;
  return {
    status: parseBadge(it?.status),
    stage: optText(it, 'stage') || CloudStage.Connect,
    deviceId: optText(it, 'deviceId'),
    watchUrl: optText(it, 'watchUrl'),
    userCode: optText(it, 'userCode'),
    signInUrl: optText(it, 'signInUrl'),
    error: optText(it, 'error'),
    issuer: optText(it, 'issuer'),
  };
};

export const parseDeviceSettings = (json: unknown): DeviceSettingsView | null => {
  if (!isJson(json) || !isJson(json.health)) return null;
  return {
    health: { ...parseBadge(json.health), href: optText(json.health, 'href') },
    notice: isJson(json.notice) ? parseBadge(json.notice) : null,
    stats: jsonObjects(optJSONArray(json, 'stats')).map((stat) => ({ label: optText(stat, 'label'), value: optText(stat, 'value'), hint: optText(stat, 'hint'), level: optInt(stat, 'level') })),
    services: jsonObjects(optJSONArray(json, 'services')).map((row) => ({ label: optText(row, 'label'), state: optText(row, 'state'), level: optInt(row, 'level') })),
    cloud: parseCloud(optJSONObject(json, 'cloud')),
    software: parseSoftware(optJSONObject(json, 'software')),
    hostname: optText(json, 'hostname'),
  };
};

export const SERVICES = 'Services';
