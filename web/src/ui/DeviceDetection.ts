import { isJson, jsonObjects, optBoolean, optJSONArray, optText } from '@/bridge/Json';
import { type DeviceBadge, parseBadge } from '@/ui/DeviceKit';
import { type DevicePart, type DeviceRefusal, deviceUpload } from '@/ui/DeviceView';

export type DetectModel = { readonly name: string; readonly path: string; readonly about: string; readonly size: string; readonly classes: DeviceBadge; readonly engine: string; readonly active: boolean };

export type DetectCamera = { readonly name: string; readonly on: boolean; readonly status: DeviceBadge };

export type DetectionView = { readonly status: DeviceBadge; readonly unavailable: boolean; readonly cameras: readonly DetectCamera[]; readonly models: readonly DetectModel[] };

export const NO_LABELS = 'no labels';

export const parseDetection = (json: unknown): DetectionView | null =>
  isJson(json)
    ? {
        status: parseBadge(json.status),
        unavailable: optBoolean(json, 'unavailable'),
        cameras: jsonObjects(optJSONArray(json, 'cameras')).map((camera) => ({ name: optText(camera, 'name'), on: optBoolean(camera, 'on'), status: parseBadge(camera.status) })),
        models: jsonObjects(optJSONArray(json, 'models')).map((model) => ({
          name: optText(model, 'name'),
          path: optText(model, 'path'),
          about: optText(model, 'about'),
          size: optText(model, 'size'),
          classes: parseBadge(model.classes),
          engine: optText(model, 'engine'),
          active: optBoolean(model, 'active'),
        })),
      }
    : null;

export const detecting = (view: DetectionView): boolean => view.cameras.some((camera) => camera.on);

export const modelPath = (name: string): string => `/api/detect/models/${encodeURIComponent(name)}`;

export type ModelUpload = { readonly hef: File; readonly labels: File | null; readonly about: string; readonly use: boolean };

export const modelFields = (upload: ModelUpload): readonly DevicePart[] => [
  { name: 'hef', file: upload.hef },
  ...(upload.labels !== null ? [{ name: 'labels', file: upload.labels }] : []),
  ...(upload.about.trim() !== '' ? [{ name: 'about', value: upload.about.trim() }] : []),
  ...(upload.use ? [{ name: 'use', value: '1' }] : []),
];

export type UploadOutcome = { readonly refusal: DeviceRefusal | null; readonly message: string };

export const uploadMessage = (switched: boolean, use: boolean, cameraDetecting: boolean): UploadOutcome =>
  switched
    ? {
        refusal: null,
        message: cameraDetecting ? 'Model uploaded and switched — detection restarts on it.' : 'Model uploaded and switched — no camera is detecting yet. Turn it on for a camera on this page.',
      }
    : use
      ? { refusal: { message: 'Model uploaded, but switching to it failed — pick it from the list.', field: '' }, message: '' }
      : { refusal: null, message: 'Model uploaded — use it from the list when you want it.' };

export const uploadModel = async (upload: ModelUpload, cameraDetecting: boolean): Promise<UploadOutcome> => {
  const { refusal, result } = await deviceUpload('POST', '/api/detect/models', modelFields(upload));
  return refusal === null ? uploadMessage(isJson(result) && optBoolean(result, 'switched'), upload.use, cameraDetecting) : { refusal, message: '' };
};

export const addLabels = async (name: string, labels: File): Promise<DeviceRefusal | null> => (await deviceUpload('PUT', `${modelPath(name)}/labels`, [{ name: 'labels', file: labels }])).refusal;
