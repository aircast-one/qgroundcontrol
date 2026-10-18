import { isJson, jsonObjects, jsonStrings, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, parseBadge } from '@/ui/DeviceKit';

export type BackendChoice = { readonly status: DeviceBadge; readonly text: string; readonly switchTo: string; readonly label: string; readonly confirm: string; readonly success: string };

export type SignInMode = {
  readonly value: string;
  readonly title: string;
  readonly desc: string;
  readonly fields: readonly string[];
  readonly clears: readonly string[];
  readonly submit: string;
  readonly unsaved: string;
  readonly confirm: string;
};

export type SignInField = { readonly name: string; readonly label: string; readonly value: string; readonly placeholder: string; readonly secret: boolean };

export type PublishSignIn = {
  readonly status: DeviceBadge;
  readonly awaitingCloud: boolean;
  readonly mode: string;
  readonly modes: readonly SignInMode[];
  readonly fields: readonly SignInField[];
  readonly environment: string;
  readonly verdict: DeviceBadge;
};

export type DiagnosticsSettings = {
  readonly status: DeviceBadge;
  readonly hint: string;
  readonly endpoint: string;
  readonly token: string;
  readonly tokenLabel: string;
  readonly streaming: boolean;
  readonly canStream: boolean;
  readonly state: string;
  readonly confirm: string;
};

export type EngineeringView = { readonly backend: BackendChoice; readonly publish: PublishSignIn; readonly diagnostics: DiagnosticsSettings };

const parsePublish = (json: unknown): PublishSignIn => {
  const it = isJson(json) ? json : null;
  return {
    status: parseBadge(it?.status),
    awaitingCloud: optBoolean(it, 'awaitingCloud'),
    mode: optText(it, 'mode'),
    modes: jsonObjects(optJSONArray(it, 'modes')).map((mode) => ({
      value: optText(mode, 'value'),
      title: optText(mode, 'title'),
      desc: optText(mode, 'desc'),
      fields: jsonStrings(optJSONArray(mode, 'fields')),
      clears: jsonStrings(optJSONArray(mode, 'clears')),
      submit: optText(mode, 'submit'),
      unsaved: optText(mode, 'unsaved'),
      confirm: optText(mode, 'confirm'),
    })),
    fields: jsonObjects(optJSONArray(it, 'fields')).map((field) => ({
      name: optText(field, 'name'),
      label: optText(field, 'label'),
      value: optText(field, 'value'),
      placeholder: optText(field, 'placeholder'),
      secret: optBoolean(field, 'secret'),
    })),
    environment: optText(it, 'environment'),
    verdict: parseBadge(it?.verdict),
  };
};

export const parseEngineering = (json: unknown): EngineeringView | null => {
  if (!isJson(json) || !isJson(json.backend)) return null;
  const backend = json.backend;
  const diagnostics = optJSONObject(json, 'diagnostics');
  return {
    backend: {
      status: parseBadge(backend.status),
      text: optText(backend, 'text'),
      switchTo: optText(backend, 'switch'),
      label: optText(backend, 'label'),
      confirm: optText(backend, 'confirm'),
      success: optText(backend, 'success'),
    },
    publish: parsePublish(optJSONObject(json, 'publish')),
    diagnostics: {
      status: parseBadge(diagnostics?.status),
      hint: optText(diagnostics, 'hint'),
      endpoint: optText(diagnostics, 'endpoint'),
      token: optText(diagnostics, 'token'),
      tokenLabel: optText(diagnostics, 'tokenLabel'),
      streaming: optBoolean(diagnostics, 'streaming'),
      canStream: optBoolean(diagnostics, 'canStream'),
      state: optText(diagnostics, 'state'),
      confirm: optText(diagnostics, 'confirm'),
    },
  };
};

export const modeNamed = (publish: PublishSignIn, value: string): SignInMode | null => publish.modes.find((mode) => mode.value === value) ?? null;

export const shownFields = (publish: PublishSignIn, value: string): readonly SignInField[] => {
  const names = modeNamed(publish, value)?.fields ?? [];
  return publish.fields.filter((field) => names.includes(field.name));
};

export const signInBody = (publish: PublishSignIn, value: string, edits: Readonly<Record<string, string>>): Record<string, string> => {
  const clears = modeNamed(publish, value)?.clears ?? [];
  return Object.fromEntries(publish.fields.map((field) => [field.name, clears.includes(field.name) ? '' : (edits[field.name] ?? field.value)]));
};

export const signInKey = (publish: PublishSignIn): string => [publish.mode, ...publish.fields.map((field) => field.value)].join('|');
