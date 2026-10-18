import { type Json, isJson, jsonObjects, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceOption, parseOptions } from '@/ui/DeviceKit';

export type PortOption = {
  readonly value: string;
  readonly label: string;
  readonly address: string;
};
export type CameraRef = { readonly name: string; readonly source: string };
export type CameraTemplate = {
  readonly id: string;
  readonly label: string;
  readonly hint: string;
  readonly cidr: string;
  readonly host: string;
  readonly name: string;
  readonly gimbal: string;
  readonly gimbalAddr: string;
};
export type CameraSubnet = { readonly cidr: string; readonly label: string };

export type CameraWizardView = {
  readonly name: string;
  readonly isNew: boolean;
  readonly suggestedName: string;
  readonly source: string;
  readonly live: boolean;
  readonly rtsp: string;
  readonly rtspHint: string;
  readonly addressOpen: boolean;
  readonly lookingFor: string;
  readonly piCamera: { readonly note: string; readonly action: string } | null;
  readonly iface: string;
  readonly interfaces: readonly PortOption[];
  readonly transport: string;
  readonly transports: readonly DeviceOption[];
  readonly others: readonly CameraRef[];
  readonly templates: readonly CameraTemplate[];
  readonly subnets: readonly CameraSubnet[];
  readonly coverage: string;
};

export type ScanNeighbor = { readonly addr: string; readonly suggest: string };
export type ScanPort = {
  readonly iface: string;
  readonly label: string;
  readonly type: string;
  readonly state: string;
  readonly subnet: string;
  readonly checked: number;
  readonly reason: string;
  readonly triedAddress: string;
  readonly isUplink: boolean;
  readonly neighbors: readonly ScanNeighbor[];
};
export type FoundCamera = {
  readonly host: string;
  readonly url: string;
  readonly needsAuth: boolean;
  readonly pathFound: boolean;
  readonly used: string;
};
export type ScannedPort = { readonly port: ScanPort; readonly found: boolean };
export type ScanOutcome = {
  readonly cams: readonly FoundCamera[];
  readonly ports: readonly ScanPort[];
  readonly todo: readonly ScannedPort[];
  readonly rest: readonly ScannedPort[];
  readonly note: string;
};
export type PortFixer = {
  readonly open: boolean;
  readonly input: string;
  readonly suggest: string;
  readonly status: string;
  readonly busy: boolean;
};

export const PI_CAMERA = 'rpiCamera';

export const SCANNING_NOTE = 'Looking for cameras on every port of this device — up to 20 seconds…';
export const SCAN_FAILED_NOTE = "Couldn't reach this device to run the scan — check you're still connected to it, or type the camera's address below.";
export const NO_ANSWER_CHECKLIST = 'No camera answered — check it, then scan again';

const parseSubnets = (json: Json | null): readonly CameraSubnet[] =>
  jsonObjects(optJSONArray(json, 'subnets')).map((it) => ({
    cidr: optText(it, 'cidr'),
    label: optText(it, 'label'),
  }));

export const parseCameraWizard = (json: unknown): CameraWizardView | null => {
  if (!isJson(json)) return null;
  const pi = optJSONObject(json, 'piCamera');
  return {
    name: optText(json, 'name'),
    isNew: optBoolean(json, 'new'),
    suggestedName: optText(json, 'suggestedName'),
    source: optText(json, 'source'),
    live: optBoolean(json, 'live'),
    rtsp: optText(json, 'rtsp'),
    rtspHint: optText(json, 'rtspHint'),
    addressOpen: optBoolean(json, 'addressOpen'),
    lookingFor: optText(json, 'lookingFor'),
    piCamera: pi === null ? null : { note: optText(pi, 'note'), action: optText(pi, 'action') },
    iface: optText(json, 'interface'),
    interfaces: jsonObjects(optJSONArray(json, 'interfaces')).map((it) => ({
      value: optText(it, 'value'),
      label: optText(it, 'label'),
      address: optText(it, 'address'),
    })),
    transport: optText(json, 'transport'),
    transports: parseOptions(json, 'transports'),
    others: jsonObjects(optJSONArray(json, 'others')).map((it) => ({
      name: optText(it, 'name'),
      source: optText(it, 'source'),
    })),
    templates: jsonObjects(optJSONArray(json, 'templates')).map((it) => ({
      id: optText(it, 'id'),
      label: optText(it, 'label'),
      hint: optText(it, 'hint'),
      cidr: optText(it, 'cidr'),
      host: optText(it, 'host'),
      name: optText(it, 'name'),
      gimbal: optText(it, 'gimbal'),
      gimbalAddr: optText(it, 'gimbalAddr'),
    })),
    subnets: parseSubnets(json),
    coverage: optText(json, 'coverage'),
  };
};

export const parseScanPorts = (json: unknown): readonly ScanPort[] =>
  jsonObjects(optJSONArray(isJson(json) ? json : null, 'ports')).map((it) => ({
    iface: optText(it, 'interface'),
    label: optText(it, 'label'),
    type: optText(it, 'type'),
    state: optText(it, 'state'),
    subnet: optText(it, 'subnet'),
    checked: optInt(it, 'checked'),
    reason: optText(it, 'reason'),
    triedAddress: optText(it, 'triedAddress'),
    isUplink: optBoolean(it, 'isUplink'),
    neighbors: jsonObjects(optJSONArray(it, 'neighbors')).map((n) => ({
      addr: optText(n, 'addr'),
      suggest: optText(n, 'suggest'),
    })),
  }));

export const parseFoundCameras = (json: unknown, wizardName: string): readonly FoundCamera[] =>
  jsonObjects(optJSONArray(isJson(json) ? json : null, 'cameras')).map((it) => ({
    host: optText(it, 'host'),
    url: optText(it, 'url'),
    needsAuth: optBoolean(it, 'needsAuth'),
    pathFound: optBoolean(it, 'pathFound'),
    used: optText(it, 'usedBy') === wizardName ? '' : optText(it, 'usedBy'),
  }));

export const parseCameraStatus = (json: unknown): { readonly state: string; readonly reason: string } | null =>
  isJson(json) ? { state: optText(json, 'state'), reason: optText(json, 'reason') } : null;

export const rtspHost = (rtsp: string): string => {
  try {
    return new URL(rtsp).hostname;
  } catch {
    return '';
  }
};

export const suggestIP = (rtsp: string): string => {
  const parts = /^(\d+)\.(\d+)\.(\d+)\.(\d+)$/.exec(rtspHost(rtsp));
  return parts === null ? '' : `${parts[1]}.${parts[2]}.${parts[3]}.${parts[4] === '2' ? '3' : '2'}/24`;
};

export const normalizeRtsp = (raw: string): string => {
  const value = raw.trim();
  return value === '' || /^[a-z][a-z0-9+.-]*:\/\//i.test(value) || !/^[\w.-]+(:\d+)?(\/.*)?$/.test(value) ? value : `rtsp://${value}`;
};

export const cidrIn = (rtsp: string): string => {
  const parts = /^(?:rtsps?:\/\/)?(\d{1,3}(?:\.\d{1,3}){3})\/(\d|[12]\d|3[0-2])$/i.exec(rtsp.trim());
  return parts === null ? '' : `${parts[1]}/${parts[2]}`;
};

export const ipToInt = (ip: string): number | null => {
  const parts = ip.split('.');
  return parts.length !== 4 ? null : parts.reduce<number | null>((acc, part) => (acc === null || !/^\d+$/.test(part) || Number(part) > 255 ? null : acc * 256 + Number(part)), 0);
};

export const sameSubnet = (host: string, subnet: string): boolean => {
  const [base = '', bitsText = ''] = subnet.split('/');
  const bits = Number(bitsText);
  const a = ipToInt(host);
  const b = ipToInt(base);
  if (a === null || b === null || !(bits >= 0 && bits <= 32) || bitsText === '') return false;
  const mask = bits === 0 ? 0 : (0xffffffff << (32 - bits)) >>> 0;
  return (a & mask) >>> 0 === (b & mask) >>> 0;
};

export const portHasSubnet = (port: ScanPort, want: string): boolean => {
  const host = want.split('/')[0] ?? '';
  return port.subnet !== '' && host !== '' && sameSubnet(host, port.subnet);
};

export const firstSuggest = (port: ScanPort): string => port.neighbors.map((n) => n.suggest).find((s) => s !== '') ?? '';

export const subnetName = (subnets: readonly CameraSubnet[], subnet: string): string =>
  subnet === '' ? '' : (subnets.find((option) => sameSubnet(option.cidr.split('/')[0] ?? '', subnet))?.label ?? '');

export const idleOnCameraSubnet = (subnets: readonly CameraSubnet[], port: ScanPort): boolean => subnetName(subnets, port.subnet) !== '' && port.neighbors.length > 0 && firstSuggest(port) === '';

export const camRowLabel = (cam: FoundCamera): string => (!cam.pathFound ? 'answers on RTSP — add the stream path' : cam.needsAuth ? 'needs a username and password' : 'ready to use');

export const rowNote = (cam: FoundCamera): string => (cam.used !== '' ? `already added as ${cam.used}` : camRowLabel(cam));

export const neighborLabel = (port: ScanPort): string => {
  const lead = port.neighbors[0];
  if (lead === undefined) return '';
  const more = port.neighbors.length > 1 ? ` (and ${port.neighbors.length - 1} more)` : '';
  const here = port.subnet !== '' ? `, not on ${port.subnet}` : '';
  const advice = lead.suggest !== '' ? ` — set this port to ${lead.suggest} to reach it` : ' — probably a router or switch, not a camera';
  return `a device answered at ${lead.addr}${more} (not confirmed as a camera)${here}${advice}`;
};

const REASONS: ReadonlyMap<string, (address: string) => string> = new Map([
  ['address-unusable', (address: string) => `${address} could not be used — another device on this network already has it, or nothing on that subnet answered`],
  ['address-expired', (address: string) => `${address} expired and could not be renewed`],
  ['needs-password', () => 'this port needs a password before it can connect'],
  ['config-failed', () => 'the settings on this port were refused — set an address below'],
]);

const stateLabel = (port: ScanPort): string => {
  switch (port.state) {
    case 'searched':
      return `checked ${port.checked} addresses on ${port.subnet} — nothing answered`;
    case 'no-carrier':
      return port.type === 'wi-fi' ? 'not connected to any network' : "nothing plugged in — check the cable and the camera's power";
    case 'not-configured':
      return 'cable is in, but this port has no address yet';
    case 'configuring':
      return 'still getting an address — try again in a moment';
    case 'failed':
      return 'could not get an address — set one below';
    case 'no-address':
      return 'no address on this port — set one below to reach the camera';
    case 'subnet-too-big':
      return `${port.subnet} is too wide to sweep — type the camera's address below`;
    case 'no-hosts':
      return `${port.subnet} has no other address on it`;
    default:
      return port.state;
  }
};

export const portLabel = (port: ScanPort): string => {
  const neighbor = neighborLabel(port);
  const reason = REASONS.get(port.reason);
  return neighbor !== '' ? neighbor : reason !== undefined ? reason(port.triedAddress || 'that address') : stateLabel(port);
};

export const cableLabel = (port: ScanPort, host: string): string =>
  host !== '' && port.subnet !== '' && sameSubnet(host, port.subnet)
    ? `nothing answered at ${host} on ${port.iface} — the port is set right; check the cable goes to the camera, not a router or switch.`
    : '';

export const portActionable = (port: ScanPort, found: boolean): boolean => !found && !port.isUplink && (port.neighbors.length > 0 || port.state !== 'searched');

export const portDetail = (scanned: ScannedPort, subnets: readonly CameraSubnet[], sourceHost: string): string => {
  const { port } = scanned;
  if (scanned.found) return 'camera found here';
  if (port.isUplink) return `${portLabel(port)} — this is the device's own uplink`;
  if (idleOnCameraSubnet(subnets, port))
    return `Set for ${subnetName(subnets, port.subnet)} (${port.subnet}). Something at ${port.neighbors[0]?.addr ?? ''} is on this cable — probably a router or switch, not a camera. Plug the camera in here and scan again.`;
  return cableLabel(port, sourceHost) || portLabel(port);
};

export const scanOutcome = (cams: readonly FoundCamera[], ports: readonly ScanPort[], sourceHost: string): ScanOutcome => {
  const fresh = cams.filter((cam) => cam.used === '');
  const classified = ports.map((port) => {
    const found = cams.some((cam) => port.subnet !== '' && sameSubnet(cam.host, port.subnet));
    const wired = cableLabel(port, sourceHost) !== '' && fresh.length === 0;
    return { port, found, actionable: wired || portActionable(port, found || fresh.length > 0) };
  });
  return {
    cams,
    ports,
    todo: classified.filter((it) => it.actionable).map(({ port, found }) => ({ port, found })),
    rest: classified.filter((it) => !it.actionable).map(({ port, found }) => ({ port, found })),
    note: cams.length > 0 ? '' : ports.length > 0 ? 'No camera answered.' : "No camera port on this device — type the camera's address below.",
  };
};

export const fixersFor = (outcome: ScanOutcome, subnets: readonly CameraSubnet[]): Readonly<Record<string, PortFixer>> =>
  Object.fromEntries(
    outcome.todo.map(({ port }) => {
      const suggest = firstSuggest(port);
      return [
        port.iface,
        {
          open: suggest !== '' || (outcome.todo.length === 1 && !idleOnCameraSubnet(subnets, port)),
          input: suggest,
          suggest,
          status: '',
          busy: false,
        },
      ];
    }),
  );

export const adoptedCamera = (outcome: ScanOutcome, auto: boolean, rtsp: string): FoundCamera | null => {
  const fresh = outcome.cams.filter((cam) => cam.used === '');
  const only = fresh[0];
  return fresh.length === 1 && only !== undefined && only.pathFound && !(auto && rtsp.trim() !== '') ? only : null;
};

export const camHeading = (cams: readonly FoundCamera[]): string => (cams.length === 1 ? '1 camera' : `${cams.length} cameras`);

export const checklistHint = (ports: readonly ScanPort[], subnets: readonly CameraSubnet[]): string => {
  const dead = ports.find((port) => port.type === 'ethernet' && port.state === 'no-carrier');
  const idle = ports.find((port) => idleOnCameraSubnet(subnets, port));
  return dead !== undefined
    ? `Nothing is plugged into the ${dead.label}.`
    : idle !== undefined
      ? `Something else is plugged into the ${idle.label} — it looks like a router or switch, not a camera.`
      : '';
};

export const todoHeading = (todo: readonly ScannedPort[], subnets: readonly CameraSubnet[]): string => {
  const only = todo.length === 1 ? todo[0] : undefined;
  return only === undefined ? 'Ports you can still set up' : idleOnCameraSubnet(subnets, only.port) ? 'Check the cable' : 'Try this';
};

export const subnetOptions = (port: ScanPort, ports: readonly ScanPort[], subnets: readonly CameraSubnet[]): readonly CameraSubnet[] => {
  const found = port.neighbors.filter((n) => n.suggest !== '').map((n) => ({ cidr: n.suggest, label: `found ${n.addr}` }));
  const tried = port.triedAddress !== '' ? [{ cidr: port.triedAddress, label: 'tried before' }] : [];
  const claimed = (cidr: string) => ports.some((other) => other.iface !== port.iface && portHasSubnet(other, cidr));
  return [...found, ...tried, ...subnets].filter((option, index, all) => !claimed(option.cidr) && all.findIndex((it) => it.cidr === option.cidr) === index);
};

export const fixerHint = (fixer: PortFixer): string =>
  fixer.suggest !== ''
    ? "This device's address on the camera's subnet. Prefilled from the device that answered on this port."
    : "This device's address on the camera's subnet — match the camera (a 192.168.144.25 camera needs 192.168.144.x here). Pick your camera's make below, or read its address from its app or manual.";

export const fixerGoLabel = (port: ScanPort, fixer: PortFixer): string => (fixer.suggest !== '' ? `Set up ${port.iface} and find the camera` : 'Set up & scan again');

export const templatePort = (ports: readonly ScanPort[], template: CameraTemplate): ScanPort | null =>
  ports.find((port) => !port.isUplink && portHasSubnet(port, template.cidr)) ??
  ports.find((port) => !port.isUplink && port.type === 'ethernet' && port.state !== 'no-carrier') ??
  ports.find((port) => !port.isUplink && port.type === 'ethernet') ??
  null;

export const templateReadyPort = (template: CameraTemplate, cams: readonly FoundCamera[], ports: readonly ScanPort[]): ScanPort | null =>
  cams.some((cam) => cam.host === template.host) ? null : (ports.find((port) => portHasSubnet(port, template.cidr)) ?? null);

export const templateSub = (template: CameraTemplate, ready: ScanPort | null): string =>
  ready === null ? template.hint : `The ${ready.label} is ready for a ${template.label} — nothing answered at ${template.host}. Check the cable.`;

export const templateAction = (busy: boolean, ready: ScanPort | null): string => (busy ? 'Setting up…' : ready !== null ? 'Scan again' : 'Set up');

export const templateHolderError = (holder: ScanPort, template: CameraTemplate): string =>
  `The ${holder.label} already uses ${(template.cidr.split('/')[0] ?? '').replace(/\.\d+$/, '.x')}, so the camera port cannot take it too — move the ${template.label} to another subnet in its app, or disconnect ${holder.label} first.`;

export const camURL = (base: string, user: string, pass: string): string =>
  base === '' || (user.trim() === '' && pass === '')
    ? base
    : base.replace(/^(rtsps?:\/\/)/i, (scheme) => `${scheme}${encodeURIComponent(user.trim())}${pass !== '' ? `:${encodeURIComponent(pass)}` : ''}@`);

export const reachablePort = (rtsp: string, ports: readonly ScanPort[]): ScanPort | null => {
  const host = rtspHost(rtsp);
  return host === '' ? null : (ports.find((port) => !port.isUplink && port.subnet !== '' && sameSubnet(host, port.subnet)) ?? null);
};

export const commitsNow = (rtsp: string, ports: readonly ScanPort[]): boolean => rtsp.trim() !== '' && (rtsp.trim() === PI_CAMERA || reachablePort(rtsp.trim(), ports) !== null);

export const nextLabel = (rtsp: string, ports: readonly ScanPort[], isNew: boolean): string => (!commitsNow(rtsp, ports) ? 'Continue' : isNew ? 'Add camera' : 'Set up & verify');

export const continueLeads = (rtsp: string): boolean => rtsp.trim() !== '';

export const scanLeads = (rtsp: string, picked: boolean): boolean => rtsp.trim() === '' && !picked;

export type NextInput = {
  readonly rtsp: string;
  readonly name: string | null;
  readonly others: readonly CameraRef[];
  readonly ports: readonly ScanPort[];
  readonly dupAck: string;
};

export type WizardRefusal = {
  readonly text: string;
  readonly warn: boolean;
  readonly field: 'rtsp' | 'name' | null;
};

export type NextOutcome =
  | {
      readonly kind: 'refuse';
      readonly rtsp: string;
      readonly name: string | null;
      readonly refusal: WizardRefusal;
      readonly dupAck: string;
    }
  | { readonly kind: 'apply' | 'port'; readonly rtsp: string; readonly name: string | null };

export const nextStep = ({ rtsp: typed, name: typedName, others, ports, dupAck }: NextInput): NextOutcome => {
  const cidr = cidrIn(typed);
  const refuse = (rtsp: string, name: string | null, text: string, field: WizardRefusal['field'], warn = false, ack = dupAck): NextOutcome => ({
    kind: 'refuse',
    rtsp,
    name,
    refusal: { text, warn, field },
    dupAck: ack,
  });
  if (cidr !== '')
    return refuse(
      typed,
      typedName,
      `${cidr} is an address for this device's port, not the camera. It belongs in the port box above — set the port to it, and the scan fills this field with the camera's own URL.`,
      'rtsp',
    );
  const piCamera = typed.trim() === PI_CAMERA;
  const rtsp = piCamera ? PI_CAMERA : normalizeRtsp(typed);
  const name = typedName === null ? null : typedName.trim().toLowerCase();
  if (name === '') return refuse(rtsp, name, 'Give the camera a name.', 'name');
  if (name !== null && others.some((other) => other.name.toLowerCase() === name)) return refuse(rtsp, name, `A camera named “${name}” already exists — pick another name.`, 'name');
  if (!piCamera && !/^rtsps?:\/\/.+/i.test(rtsp)) return refuse(rtsp, name, "Enter the camera's address, e.g. 192.168.8.10 or rtsp://192.168.8.10:554/stream", 'rtsp');
  const duplicate = others.find((other) => other.source.trim() === rtsp);
  if (duplicate !== undefined && dupAck !== rtsp)
    return refuse(
      rtsp,
      name,
      `Camera “${duplicate.name}” already uses this source. If that camera only allows one connection, this one won't get video. Click Continue again to add it anyway, or change the URL.`,
      null,
      true,
      rtsp,
    );
  return { kind: piCamera || reachablePort(rtsp, ports) !== null ? 'apply' : 'port', rtsp, name };
};

export const portToSuggest = (ports: readonly ScanPort[]): string => ports.find((port) => !port.isUplink && port.state !== 'searched')?.iface ?? '';

export const applyNote = (iface: string, ip: string, current: string): string =>
  iface === '' || ip.trim() === ''
    ? ''
    : current === ip.trim()
      ? `${iface} keeps ${ip.trim()} — nothing on that port changes.`
      : `Sets ${iface} to ${ip.trim()} — anything else on that port may need reconnecting.`;

export const STEP_NAMES = ['Camera', 'Port', 'Picture'] as const;
