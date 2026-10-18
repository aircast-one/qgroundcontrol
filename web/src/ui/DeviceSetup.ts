import { type Json, isJson, jsonObjects, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type CameraWizardView, parseCameraWizard } from '@/ui/DeviceCameraWizard';
import { parsePage } from '@/ui/DeviceCameras';
import { type DeviceBadge, parseBadge } from '@/ui/DeviceKit';
import { type TailscaleCard, optBadge, parseTailscale } from '@/ui/DeviceNetwork';
import type { DevicePage } from '@/ui/DeviceStatus';
import { type DeviceEndpoint, parseEndpointList } from '@/ui/DeviceTelemetry';
import type { WifiReopen } from '@/ui/DeviceWifiWizard';

export type SetupStep = { readonly key: string; readonly label: string; readonly title: string; readonly lead: string; readonly status: DeviceBadge | null };

export type SetupCheckRow = { readonly label: string; readonly status: DeviceBadge; readonly step: number };

export type SetupInternet = {
  readonly online: DeviceBadge;
  readonly joinOpen: boolean;
  readonly reopen: WifiReopen;
  readonly connection: DevicePage;
  readonly hint: string;
};

export type SetupCamera = { readonly problem: string; readonly wizard: CameraWizardView; readonly watch: string; readonly cameras: DevicePage; readonly hint: string };

export type SetupTelemetry = { readonly waiting: DeviceBadge | null; readonly endpoints: readonly DeviceEndpoint[]; readonly hint: string };

export type SetupView = {
  readonly start: number;
  readonly steps: readonly SetupStep[];
  readonly internet: SetupInternet;
  readonly remote: TailscaleCard;
  readonly camera: SetupCamera;
  readonly telemetry: SetupTelemetry;
  readonly checks: readonly SetupCheckRow[];
};

export type SetupNav = { readonly back: boolean; readonly later: boolean; readonly next: 'next' | 'skip' | null; readonly finish: boolean };

const parseInternet = (json: Json | null): SetupInternet => {
  const reopen = optJSONObject(json, 'reopen');
  return {
    online: parseBadge(json?.online),
    joinOpen: optBoolean(json, 'joinOpen'),
    reopen: { host: optText(reopen, 'host'), hostname: optText(reopen, 'hostname') },
    connection: parsePage(optJSONObject(json, 'connection')),
    hint: optText(json, 'hint'),
  };
};

export const parseSetup = (json: unknown): SetupView | null => {
  if (!isJson(json)) return null;
  const camera = optJSONObject(json, 'camera');
  const wizard = parseCameraWizard(camera?.wizard);
  const telemetry = optJSONObject(json, 'telemetry');
  const steps = jsonObjects(optJSONArray(json, 'steps')).map((it) => ({
    key: optText(it, 'key'),
    label: optText(it, 'label'),
    title: optText(it, 'title'),
    lead: optText(it, 'lead'),
    status: optBadge(it, 'status'),
  }));
  return steps.length === 0 || wizard === null
    ? null
    : {
        start: clampStep(optInt(json, 'start', 1), steps.length),
        steps,
        internet: parseInternet(optJSONObject(json, 'internet')),
        remote: parseTailscale(optJSONObject(json, 'remote')),
        camera: { problem: optText(camera, 'problem'), wizard, watch: optText(camera, 'watch'), cameras: parsePage(optJSONObject(camera, 'cameras')), hint: optText(camera, 'hint') },
        telemetry: { waiting: optBadge(telemetry, 'waiting'), endpoints: parseEndpointList(telemetry, 'endpoints'), hint: optText(telemetry, 'hint') },
        checks: jsonObjects(optJSONArray(json, 'checks')).map((it) => ({ label: optText(it, 'label'), status: parseBadge(it.status), step: optInt(it, 'step', 1) })),
      };
};

export const clampStep = (step: number, total: number): number => Math.min(total, Math.max(1, step));

export const CAMERA_APPLYING = 3;

export const cameraHoldsTheWizard = (key: string, wizardShown: boolean, wizardStep: number): boolean => key === 'camera' && wizardShown && wizardStep !== CAMERA_APPLYING;

export const setupNav = (step: number, total: number, held: boolean): SetupNav => ({
  back: step > 1 && !held,
  later: step < total,
  next: step >= total ? null : held ? 'skip' : 'next',
  finish: step === total,
});
