import { type Json, isJson, jsonObjects, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, type DeviceChartView, type DeviceOption, parseBadge, parseChart, parseOptions } from '@/ui/DeviceKit';
import { type CameraWizardView, parseCameraWizard } from '@/ui/DeviceCameraWizard';
import { type DevicePage } from '@/ui/DeviceStatus';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { type ConnectTarget, parseTargets } from '@/ui/DeviceConnect';
import { type DeviceEndpoint, parseEndpointList } from '@/ui/DeviceTelemetry';

export type CameraCopy = { readonly label: string; readonly text: string; readonly qr: boolean };
export type CameraFact = { readonly label: string; readonly value: string };
export type CameraFix = { readonly label: string; readonly scan: boolean };
export type CameraChip = {
  readonly key: string;
  readonly label: string;
  readonly level: number;
  readonly href: string;
};
export type SetupPart = {
  readonly title: string;
  readonly summary: string;
  readonly badge: DeviceBadge | null;
  readonly open: boolean;
};
export type DeviceLink = { readonly label: string; readonly href: string };
export type LinkedNotice = DeviceBadge & { readonly link: DeviceLink | null };
export type ServerDest = {
  readonly kind: string;
  readonly part: SetupPart;
  readonly on: boolean;
  readonly url: string;
  readonly key: string;
  readonly secret: string;
  readonly app: string;
  readonly track: string;
  readonly watch: string;
};
export type ShareLink = {
  readonly id: string;
  readonly label: string;
  readonly url: string;
  readonly expires: string;
  readonly dead: string;
};

export type CameraSetup = {
  readonly watch: {
    readonly part: SetupPart;
    readonly cloud: boolean;
    readonly canConnect: boolean;
    readonly signIn: boolean;
    readonly cloudLinks: readonly DeviceEndpoint[];
    readonly cloudCopy: string;
  };
  readonly radio: {
    readonly part: SetupPart;
    readonly notice: LinkedNotice;
    readonly form: boolean;
    readonly address: string;
    readonly stop: boolean;
  };
  readonly servers: { readonly part: SetupPart; readonly dests: readonly ServerDest[] };
  readonly shares: {
    readonly part: SetupPart;
    readonly links: readonly ShareLink[];
    readonly hours: readonly DeviceOption[];
  };
  readonly detect: {
    readonly part: SetupPart;
    readonly on: boolean;
    readonly model: string;
    readonly notice: LinkedNotice | null;
  };
  readonly record: {
    readonly part: SetupPart;
    readonly on: boolean;
    readonly full: boolean;
    readonly usage: string;
  };
  readonly gimbal: { readonly part: SetupPart; readonly owner: boolean } | null;
};

export type CameraView = {
  readonly name: string;
  readonly status: DeviceBadge;
  readonly live: boolean;
  readonly shared: boolean;
  readonly preview: string;
  readonly placeholder: string;
  readonly watch: string;
  readonly copies: readonly CameraCopy[];
  readonly facts: readonly CameraFact[];
  readonly bitrate: DeviceChartView | null;
  readonly diagnosis: DeviceBadge | null;
  readonly fix: CameraFix | null;
  readonly links: readonly ConnectTarget[];
  readonly chips: readonly CameraChip[];
  readonly recording: boolean;
  readonly wizard: CameraWizardView;
  readonly setup: CameraSetup;
  readonly topic: DevicePage;
};

export type CamerasView = {
  readonly cameras: readonly CameraView[];
  readonly add: CameraWizardView;
  readonly cloudLink: string;
  readonly video: DevicePage;
};

const parsePart = (json: Json | null): SetupPart => ({
  title: optText(json, 'title'),
  summary: optText(json, 'summary'),
  badge: isJson(json?.badge) ? parseBadge(json.badge) : null,
  open: optBoolean(json, 'open'),
});

const parseLinkedNotice = (json: Json | null): LinkedNotice | null => {
  if (json === null) return null;
  const link = optJSONObject(json, 'link');
  return {
    ...parseBadge(json),
    link: link === null ? null : { label: optText(link, 'label'), href: optText(link, 'href') },
  };
};

export const parsePage = (json: Json | null): DevicePage => ({
  title: optText(json, 'title'),
  href: optText(json, 'href'),
  icon: optText(json, 'icon'),
  summary: optText(json, 'summary'),
  needs: optBoolean(json, 'needs'),
});

const EMPTY_NOTICE: LinkedNotice = { level: DeviceLevel.Neutral, text: '', link: null };

const parseSetup = (json: Json | null): CameraSetup => {
  const watch = optJSONObject(json, 'watch');
  const radio = optJSONObject(json, 'radio');
  const servers = optJSONObject(json, 'servers');
  const shares = optJSONObject(json, 'shares');
  const detect = optJSONObject(json, 'detect');
  const record = optJSONObject(json, 'record');
  const gimbal = optJSONObject(json, 'gimbal');
  return {
    watch: {
      part: parsePart(optJSONObject(watch, 'part')),
      cloud: optBoolean(watch, 'cloud'),
      canConnect: optBoolean(watch, 'canConnect'),
      signIn: optBoolean(watch, 'signIn'),
      cloudLinks: parseEndpointList(watch, 'cloudLinks'),
      cloudCopy: optText(watch, 'cloudCopy'),
    },
    radio: {
      part: parsePart(optJSONObject(radio, 'part')),
      notice: parseLinkedNotice(optJSONObject(radio, 'notice')) ?? EMPTY_NOTICE,
      form: optBoolean(radio, 'form'),
      address: optText(radio, 'address'),
      stop: optBoolean(radio, 'stop'),
    },
    servers: {
      part: parsePart(optJSONObject(servers, 'part')),
      dests: jsonObjects(optJSONArray(servers, 'dests')).map((it) => ({
        kind: optText(it, 'kind'),
        part: parsePart(optJSONObject(it, 'part')),
        on: optBoolean(it, 'on'),
        url: optText(it, 'url'),
        key: optText(it, 'key'),
        secret: optText(it, 'secret'),
        app: optText(it, 'app'),
        track: optText(it, 'track'),
        watch: optText(it, 'watch'),
      })),
    },
    shares: {
      part: parsePart(optJSONObject(shares, 'part')),
      links: jsonObjects(optJSONArray(shares, 'links')).map((it) => ({
        id: optText(it, 'id'),
        label: optText(it, 'label'),
        url: optText(it, 'url'),
        expires: optText(it, 'expires'),
        dead: optText(it, 'dead'),
      })),
      hours: parseOptions(shares, 'hours'),
    },
    detect: {
      part: parsePart(optJSONObject(detect, 'part')),
      on: optBoolean(detect, 'on'),
      model: optText(detect, 'model'),
      notice: parseLinkedNotice(optJSONObject(detect, 'notice')),
    },
    record: {
      part: parsePart(optJSONObject(record, 'part')),
      on: optBoolean(record, 'on'),
      full: optBoolean(record, 'full'),
      usage: optText(record, 'usage'),
    },
    gimbal: gimbal === null ? null : { part: parsePart(optJSONObject(gimbal, 'part')), owner: optBoolean(gimbal, 'owner') },
  };
};

const parseCamera = (json: Json): CameraView | null => {
  const wizard = parseCameraWizard(json.wizard);
  return wizard === null
    ? null
    : {
        name: optText(json, 'name'),
        status: parseBadge(json.status),
        live: optBoolean(json, 'live'),
        shared: optBoolean(json, 'shared'),
        preview: optText(json, 'preview'),
        placeholder: optText(json, 'placeholder'),
        watch: optText(json, 'watch'),
        copies: jsonObjects(optJSONArray(json, 'copies')).map((it) => ({
          label: optText(it, 'label'),
          text: optText(it, 'text'),
          qr: optBoolean(it, 'qr'),
        })),
        facts: jsonObjects(optJSONArray(json, 'facts')).map((it) => ({
          label: optText(it, 'label'),
          value: optText(it, 'value'),
        })),
        bitrate: parseChart(json.bitrate),
        diagnosis: isJson(json.diagnosis) ? parseBadge(json.diagnosis) : null,
        fix: isJson(json.fix) ? { label: optText(json.fix, 'label'), scan: optBoolean(json.fix, 'scan') } : null,
        links: parseTargets(json, 'links'),
        chips: jsonObjects(optJSONArray(json, 'chips')).map((it) => ({
          key: optText(it, 'key'),
          label: optText(it, 'label'),
          level: optInt(it, 'level'),
          href: optText(it, 'href'),
        })),
        recording: optBoolean(json, 'recording'),
        wizard,
        setup: parseSetup(optJSONObject(json, 'setup')),
        topic: parsePage(optJSONObject(json, 'topic')),
      };
};

export const parseCameras = (json: unknown): CamerasView | null => {
  const add = parseCameraWizard(isJson(json) ? json.add : null);
  return !isJson(json) || add === null
    ? null
    : {
        cameras: jsonObjects(optJSONArray(json, 'cameras'))
          .map(parseCamera)
          .filter((camera): camera is CameraView => camera !== null),
        add,
        cloudLink: optText(json, 'cloudLink'),
        video: parsePage(optJSONObject(json, 'video')),
      };
};

export type CamerasShown = { readonly kind: 'add' } | { readonly kind: 'wall' } | { readonly kind: 'camera'; readonly camera: CameraView };

export const camerasShown = (cameras: readonly CameraView[], picked: string, adding: boolean): CamerasShown => {
  const named = cameras.find((camera) => camera.name === picked);
  const only = cameras.length === 1 ? cameras[0] : undefined;
  return adding || cameras.length === 0 ? { kind: 'add' } : named !== undefined ? { kind: 'camera', camera: named } : only !== undefined ? { kind: 'camera', camera: only } : { kind: 'wall' };
};

export const cameBack = (before: ReadonlyMap<string, boolean>, cameras: readonly CameraView[]): readonly string[] =>
  cameras.filter((camera) => camera.live && before.get(camera.name) === false).map((camera) => camera.name);

export const liveness = (cameras: readonly CameraView[]): ReadonlyMap<string, boolean> => new Map(cameras.map((camera) => [camera.name, camera.live]));

export const cloudflareTarget = (app: string, track: string): string => (app.trim() !== '' && track.trim() !== '' ? `cloudflare://${app.trim()}/${track.trim()}` : '');

export const SETUP_SECTIONS = {
  watch: 'cam-watch',
  radio: 'cam-radio',
  servers: 'cam-servers',
  shares: 'cam-share',
  detect: 'cam-detect',
  record: 'cam-record',
  gimbal: 'cam-gimbal',
} as const;

export const openedSection = (href: string): string => new URL(href, 'http://device').hash.slice(1);
