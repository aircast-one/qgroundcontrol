import { Qgc } from '@/bridge/Qgc';
import { type Json, isJson, isNull, jsonObjects, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceOption, parseOptions } from '@/ui/DeviceKit';

export type PortOption = {
  readonly value: string;
  readonly label: string;
  readonly address: string;
};

export type CameraWizardView = {
  readonly name: string;
  readonly title: string;
  readonly isNew: boolean;
  readonly suggestedName: string;
  readonly rtsp: string;
  readonly rtspHint: string;
  readonly addressOpen: boolean;
  readonly lookingFor: string;
  readonly checks: readonly string[];
  readonly piCamera: { readonly note: string; readonly action: string } | null;
  readonly iface: string;
  readonly interfaces: readonly PortOption[];
  readonly transport: string;
  readonly transports: readonly DeviceOption[];
  readonly coverage: string;
  readonly json: Json;
};

export type WizardCamera = {
  readonly host: string;
  readonly url: string;
  readonly needsAuth: boolean;
  readonly pathFound: boolean;
  readonly used: string;
  readonly note: string;
  readonly action: string;
};
export type WizardChip = { readonly cidr: string; readonly text: string };
export type WizardPort = {
  readonly iface: string;
  readonly label: string;
  readonly detail: string;
  readonly open: boolean;
  readonly suggest: string;
  readonly hint: string;
  readonly go: string;
  readonly options: readonly WizardChip[];
  readonly busy: boolean;
  readonly status: string;
};
export type WizardMake = { readonly id: string; readonly label: string; readonly sub: string; readonly action: string };
export type WizardFound = { readonly n: number; readonly refusal: string; readonly camera: WizardCamera | null; readonly name: string };
export type WizardApply = {
  readonly n: number;
  readonly state: string;
  readonly text: string;
  readonly field: string;
  readonly url: string;
  readonly camera: string;
};
export type WizardNotice = { readonly n: number; readonly text: string };
export type DeviceWizard = {
  readonly session: number;
  readonly scanning: boolean;
  readonly scans: number;
  readonly again: number;
  readonly adopt: WizardCamera | null;
  readonly note: string;
  readonly checklist: string;
  readonly cams: readonly WizardCamera[];
  readonly camHeading: string;
  readonly camsHint: string;
  readonly details: string;
  readonly todoHeading: string;
  readonly todo: readonly WizardPort[];
  readonly rest: readonly WizardPort[];
  readonly restTitle: string;
  readonly templates: readonly WizardMake[];
  readonly templateBusy: boolean;
  readonly templateNote: string;
  readonly template: WizardFound;
  readonly apply: WizardApply;
  readonly notice: WizardNotice;
};

export type WizardRefusal = {
  readonly text: string;
  readonly warn: boolean;
  readonly field: string | null;
};

export type WizardNext = {
  readonly kind: string;
  readonly rtsp: string;
  readonly name: string | null;
  readonly camera: string;
  readonly refusal: WizardRefusal | null;
  readonly dupAck: string;
  readonly iface: string;
  readonly ip: string;
};

export type WizardForm = { readonly next: string; readonly skipsPort: boolean; readonly applyNote: string };

export const PI_CAMERA = 'rpiCamera';
export const DEVICE_WIZARD = 'view.deviceWizard';
const WIZARD_OPEN = 'deviceWizard.open';
const WIZARD_SCAN = 'deviceWizard.scan';
const WIZARD_FIX_PORT = 'deviceWizard.fixPort';
const WIZARD_TEMPLATE = 'deviceWizard.template';
const WIZARD_NEXT = 'deviceWizard.next';
const WIZARD_FORM = 'deviceWizard.form';
const WIZARD_SIGN_IN = 'deviceWizard.signIn';
const WIZARD_APPLY = 'deviceWizard.apply';
const WIZARD_RESTART = 'deviceWizard.restart';

export const STEP_NAMES = ['Camera', 'Port', 'Picture'] as const;

export const NO_FORM: WizardForm = { next: 'Continue', skipsPort: false, applyNote: '' };

export const parseCameraWizard = (json: unknown): CameraWizardView | null => {
  if (!isJson(json)) return null;
  const pi = optJSONObject(json, 'piCamera');
  return {
    name: optText(json, 'name'),
    title: optText(json, 'title'),
    isNew: optBoolean(json, 'new'),
    suggestedName: optText(json, 'suggestedName'),
    rtsp: optText(json, 'rtsp'),
    rtspHint: optText(json, 'rtspHint'),
    addressOpen: optBoolean(json, 'addressOpen'),
    lookingFor: optText(json, 'lookingFor'),
    checks: (optJSONArray(json, 'checks') ?? []).filter((it): it is string => typeof it === 'string'),
    piCamera: pi === null ? null : { note: optText(pi, 'note'), action: optText(pi, 'action') },
    iface: optText(json, 'interface'),
    interfaces: jsonObjects(optJSONArray(json, 'interfaces')).map((it) => ({
      value: optText(it, 'value'),
      label: optText(it, 'label'),
      address: optText(it, 'address'),
    })),
    transport: optText(json, 'transport'),
    transports: parseOptions(json, 'transports'),
    coverage: optText(json, 'coverage'),
    json,
  };
};

const parseWizardCamera = (json: Json | null): WizardCamera | null =>
  json === null
    ? null
    : {
        host: optText(json, 'host'),
        url: optText(json, 'url'),
        needsAuth: optBoolean(json, 'needsAuth'),
        pathFound: optBoolean(json, 'pathFound'),
        used: optText(json, 'used'),
        note: optText(json, 'note'),
        action: optText(json, 'action'),
      };

const parseWizardPort = (json: Json): WizardPort => ({
  iface: optText(json, 'iface'),
  label: optText(json, 'label'),
  detail: optText(json, 'detail'),
  open: optBoolean(json, 'open'),
  suggest: optText(json, 'suggest'),
  hint: optText(json, 'hint'),
  go: optText(json, 'go'),
  options: jsonObjects(optJSONArray(json, 'options')).map((it) => ({ cidr: optText(it, 'cidr'), text: optText(it, 'text') })),
  busy: optBoolean(json, 'busy'),
  status: optText(json, 'status'),
});

export const parseDeviceWizard = (json: Json | null): DeviceWizard => {
  const template = optJSONObject(json, 'template');
  const apply = optJSONObject(json, 'apply');
  const notice = optJSONObject(json, 'notice');
  return {
    session: optInt(json, 'session', -1),
    scanning: optBoolean(json, 'scanning'),
    scans: optInt(json, 'scans'),
    again: optInt(json, 'again'),
    adopt: parseWizardCamera(optJSONObject(json, 'adopt')),
    note: optText(json, 'note'),
    checklist: optText(json, 'checklist'),
    cams: jsonObjects(optJSONArray(json, 'cams')).flatMap((it) => parseWizardCamera(it) ?? []),
    camHeading: optText(json, 'camHeading'),
    camsHint: optText(json, 'camsHint'),
    details: optText(json, 'details'),
    todoHeading: optText(json, 'todoHeading'),
    todo: jsonObjects(optJSONArray(json, 'todo')).map(parseWizardPort),
    rest: jsonObjects(optJSONArray(json, 'rest')).map(parseWizardPort),
    restTitle: optText(json, 'restTitle'),
    templates: jsonObjects(optJSONArray(json, 'templates')).map((it) => ({
      id: optText(it, 'id'),
      label: optText(it, 'label'),
      sub: optText(it, 'sub'),
      action: optText(it, 'action'),
    })),
    templateBusy: optBoolean(json, 'templateBusy'),
    templateNote: optText(json, 'templateNote'),
    template: {
      n: optInt(template, 'n'),
      refusal: optText(template, 'refusal'),
      camera: parseWizardCamera(optJSONObject(template, 'camera')),
      name: optText(template, 'name'),
    },
    apply: {
      n: optInt(apply, 'n'),
      state: optText(apply, 'state'),
      text: optText(apply, 'text'),
      field: optText(apply, 'field'),
      url: optText(apply, 'url'),
      camera: optText(apply, 'camera'),
    },
    notice: { n: optInt(notice, 'n'), text: optText(notice, 'text') },
  };
};

export const NO_WIZARD = parseDeviceWizard(null);

export const parseWizardNext = (json: unknown): WizardNext | null => {
  if (!isJson(json)) return null;
  const refusal = optJSONObject(json, 'refusal');
  return {
    kind: optText(json, 'kind'),
    rtsp: optText(json, 'rtsp'),
    name: isNull(json, 'name') ? null : optText(json, 'name'),
    camera: optText(json, 'camera'),
    refusal: refusal === null ? null : { text: optText(refusal, 'text'), warn: optBoolean(refusal, 'warn'), field: isNull(refusal, 'field') ? null : optText(refusal, 'field') },
    dupAck: optText(json, 'dupAck'),
    iface: optText(json, 'iface'),
    ip: optText(json, 'ip'),
  };
};

export const parseWizardForm = (json: unknown): WizardForm =>
  isJson(json) ? { next: optText(json, 'next'), skipsPort: optBoolean(json, 'skipsPort'), applyNote: optText(json, 'applyNote') } : NO_FORM;

export const sameCamera = (a: WizardCamera | null, b: WizardCamera): boolean => a !== null && a.host === b.host && a.url === b.url;

export const continueLeads = (rtsp: string): boolean => rtsp.trim() !== '';

export const repairing = (view: CameraWizardView): boolean => !view.isNew && view.lookingFor !== '';

export const nextOffered = (view: CameraWizardView, rtsp: string, picked: boolean): boolean => !repairing(view) || picked || rtsp.trim() !== '';

export const scanLeads = (rtsp: string, picked: boolean): boolean => rtsp.trim() === '' && !picked;

export const openWizard = async (view: CameraWizardView, scan: boolean): Promise<number> => optInt(optJSONObject(await Qgc.call(WIZARD_OPEN, view.json, scan), 'result'), 'session', -1);

export const scanForCameras = async (): Promise<void> => void (await Qgc.call(WIZARD_SCAN));

export type PortAddress = { readonly port: string; readonly address: string };

export const fixWizardPort = async (iface: string, address: string): Promise<void> => void (await Qgc.call(WIZARD_FIX_PORT, iface, address));

export const setUpCameraMake = async (id: string): Promise<void> => void (await Qgc.call(WIZARD_TEMPLATE, id));

export const wizardNext = async (rtsp: string, name: string, dupAck: string): Promise<WizardNext | null> => parseWizardNext(optJSONObject(await Qgc.call(WIZARD_NEXT, rtsp, name, dupAck), 'result'));

export const wizardForm = async (rtsp: string, iface: string, ip: string, current: string): Promise<WizardForm> =>
  parseWizardForm(optJSONObject(await Qgc.call(WIZARD_FORM, rtsp, iface, ip, current), 'result'));

export const signedInUrl = async (base: string, user: string, pass: string): Promise<string> => optText(await Qgc.call(WIZARD_SIGN_IN, base, user, pass), 'result') || base;

export const applyCamera = async (rtsp: string, camera: string, iface: string, ip: string, transport: string): Promise<void> =>
  void (await Qgc.call(WIZARD_APPLY, rtsp, camera, iface, ip, transport));

export const restartWizard = async (): Promise<void> => void (await Qgc.call(WIZARD_RESTART));
