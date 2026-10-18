import { isJson, jsonObjects, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, parseBadge } from '@/ui/DeviceKit';
import { type DeviceRefusal, NO_ANSWER, deviceUrl, refusalOf } from '@/ui/DeviceView';

export type RadioSettings = { readonly status: DeviceBadge; readonly channel: string; readonly region: string; readonly txPower: string; readonly details: string };

export const SharingStage = { Intro: 'intro', Deploy: 'deploy', Live: 'live' } as const;

export type PublicSharing = { readonly status: DeviceBadge; readonly stage: string; readonly deployUrl: string; readonly signKey: string; readonly deviceToken: string; readonly baseUrl: string };

export type DetectModel = { readonly name: string; readonly path: string; readonly about: string; readonly size: string; readonly classes: DeviceBadge; readonly active: boolean };

export type DetectionSettings = { readonly status: DeviceBadge; readonly unavailable: boolean; readonly models: readonly DetectModel[] };

export type DevicePageLink = { readonly title: string; readonly href: string; readonly summary: string };

export type VideoView = { readonly radio: RadioSettings | null; readonly sharing: PublicSharing; readonly detection: DetectionSettings; readonly engineering: DevicePageLink };

const parseRadio = (json: unknown): RadioSettings | null =>
  isJson(json)
    ? { status: parseBadge(json.status), channel: optText(json, 'channel'), region: optText(json, 'region'), txPower: optText(json, 'txPower'), details: optText(json, 'details') }
    : null;

export const parseVideo = (json: unknown): VideoView | null => {
  if (!isJson(json) || !isJson(json.sharing)) return null;
  const sharing = json.sharing;
  const detection = optJSONObject(json, 'detection');
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
    detection: {
      status: parseBadge(detection?.status),
      unavailable: optBoolean(detection, 'unavailable'),
      models: jsonObjects(optJSONArray(detection, 'models')).map((model) => ({
        name: optText(model, 'name'),
        path: optText(model, 'path'),
        about: optText(model, 'about'),
        size: optText(model, 'size'),
        classes: parseBadge(model.classes),
        active: optBoolean(model, 'active'),
      })),
    },
    engineering: { title: optText(engineering, 'title'), href: optText(engineering, 'href'), summary: optText(engineering, 'summary') },
  };
};

export const SHARING_OFF = { baseUrl: '', signKey: '', deviceToken: '' } as const;

export const modelPath = (name: string): string => `/api/detect/models/${encodeURIComponent(name)}`;

export type ModelUpload = { readonly hef: File; readonly labels: File | null; readonly about: string; readonly use: boolean };

export const modelFields = (upload: ModelUpload): readonly (readonly [string, string | File])[] => [
  ['hef', upload.hef],
  ...(upload.labels !== null ? [['labels', upload.labels] as const] : []),
  ...(upload.about.trim() !== '' ? [['about', upload.about.trim()] as const] : []),
  ...(upload.use ? [['use', '1'] as const] : []),
];

const modelForm = (upload: ModelUpload): FormData =>
  modelFields(upload).reduce((form, [name, value]) => {
    form.append(name, value);
    return form;
  }, new FormData());

export type UploadOutcome = { readonly refusal: DeviceRefusal | null; readonly message: string };

export const uploadMessage = (switched: boolean, use: boolean): UploadOutcome =>
  switched
    ? { refusal: null, message: 'Model uploaded and switched — detection restarts on it.' }
    : use
      ? { refusal: { message: 'Model uploaded, but switching to it failed — pick it from the list.', field: '' }, message: '' }
      : { refusal: null, message: 'Model uploaded — use it from the list when you want it.' };

export const uploadModel = async (upload: ModelUpload): Promise<UploadOutcome> => {
  const response = await fetch(deviceUrl('/api/detect/models'), { method: 'POST', headers: { Accept: 'application/json' }, body: modelForm(upload) }).catch(() => null);
  if (response === null) return { refusal: NO_ANSWER, message: '' };
  const body: unknown = await response.json().catch(() => null);
  return response.ok ? uploadMessage(isJson(body) && optBoolean(body, 'switched'), upload.use) : { refusal: refusalOf(response.status, body), message: '' };
};
