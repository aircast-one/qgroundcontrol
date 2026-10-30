import { isJson, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, parseBadge } from '@/ui/DeviceKit';

export type RadioSettings = { readonly status: DeviceBadge; readonly channel: string; readonly region: string; readonly txPower: string; readonly details: string };

export const SharingStage = { Intro: 'intro', Deploy: 'deploy', Live: 'live' } as const;

export type PublicSharing = { readonly status: DeviceBadge; readonly stage: string; readonly deployUrl: string; readonly signKey: string; readonly deviceToken: string; readonly baseUrl: string };

export type DevicePageLink = { readonly title: string; readonly href: string; readonly summary: string };

export type VideoView = { readonly radio: RadioSettings | null; readonly sharing: PublicSharing; readonly engineering: DevicePageLink };

const parseRadio = (json: unknown): RadioSettings | null =>
  isJson(json)
    ? { status: parseBadge(json.status), channel: optText(json, 'channel'), region: optText(json, 'region'), txPower: optText(json, 'txPower'), details: optText(json, 'details') }
    : null;

export const parseVideo = (json: unknown): VideoView | null => {
  if (!isJson(json) || !isJson(json.sharing)) return null;
  const sharing = json.sharing;
  const engineering = optJSONObject(json, 'engineering');
  return {
    radio: parseRadio(json.radio),
    sharing: {
      status: parseBadge(sharing.status),
      stage: optText(sharing, 'stage') || SharingStage.Intro,
      deployUrl: optText(sharing, 'deployUrl'),
      signKey: optText(sharing, 'signKey'),
      deviceToken: optText(sharing, 'deviceToken'),
      baseUrl: optText(sharing, 'baseUrl'),
    },
    engineering: { title: optText(engineering, 'title'), href: optText(engineering, 'href'), summary: optText(engineering, 'summary') },
  };
};

export const SHARING_OFF = { baseUrl: '', signKey: '', deviceToken: '' } as const;
