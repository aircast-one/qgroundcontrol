import { AppCommands, CameraCommands, DeviceCommands, LinkCommands } from '@/bridge/BridgeCommands';
import { isJson } from '@/bridge/Json';
import { Qgc, refusal } from '@/bridge/Qgc';
import { mutableStateOf } from '@/kotlin/State';
import { DEVICE_CONNECT, DEVICE_DISCONNECT, deviceInvoke, deviceRead, deviceReading, deviceViewPath, resolveDeviceUrl } from '@/ui/DeviceView';
import { appAnswers, httpsOrigin, trustedOrigin } from './DeviceOrigin';

type Device = { name?: string; cameras?: { name: string; url: string }[]; app?: string };

const DEVICE_JSON = '/fly/device.json';

const DEVICE_SUBJECT = 'The device';

const DEVICE_PAGE = '/fly/';

const CONNECT = 'connect';

const STAY_ON_DEVICE = 'local';

const HANDOFF_LOG = 'Web.Handoff';

export type StayReason = 'not-a-device-page' | 'not-offered' | 'asked-to-stay' | 'device-unreachable' | 'app-unreachable' | 'failed';

export type Handoff = { readonly to: string } | { readonly stay: StayReason };

const REPORTED_STAYS: readonly StayReason[] = ['asked-to-stay', 'device-unreachable', 'app-unreachable', 'failed'];

const hashParams = (hash: string): URLSearchParams => new URLSearchParams(hash.replace(/^#/, ''));

export const handoffUrl = (app: string, origin: string, hash: string): string => `${new URL('/', app).href}#${new URLSearchParams([...hashParams(hash), [CONNECT, origin]])}`;

export const handoffOf = (app: string, origin: string | null, appAnswered: boolean, hash: string): Handoff =>
  origin === null ? { stay: 'device-unreachable' } : appAnswered ? { to: handoffUrl(app, origin, hash) } : { stay: 'app-unreachable' };

export const offeredApp = (device: Device | null): string | null => (device !== null && typeof device.app === 'string' && device.app !== '' ? device.app : null);

export const connectAsked = (hash: string): string | null => {
  const asked = hashParams(hash).get(CONNECT);
  return asked === null ? null : httpsOrigin(asked);
};

export const withoutConnect = (hash: string): string => {
  const rest = new URLSearchParams([...hashParams(hash)].filter(([key]) => key !== CONNECT)).toString();
  return rest === '' ? '' : `#${rest}`;
};

export const connectOffered = mutableStateOf<string | null>(null);

const recordHandoff = (event: string): void => void AppCommands.recordLog(HANDOFF_LOG, `handoff ${event}`);

const dropConnect = (): void => {
  const rest = withoutConnect(location.hash);
  if (rest !== location.hash) history.replaceState(null, '', `${location.pathname}${location.search}${rest}`);
};

export const deviceSocket = (page: string): string => {
  const url = new URL('mavlink', page);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return url.href;
};

export const deviceBase = (page: string): string => new URL('.', page).href;

export const deviceLinkName = (device: Device, hostname: string): string => `Aircast ${device.name?.trim() || hostname}`;

export const deviceCameras = (device: Device): [string, string][] => (device.cameras ?? []).map(({ name, url }) => [name, url]);

export const deviceServed = mutableStateOf(false);

const readServed = async (): Promise<Device | null> => {
  const response = await fetch('device.json', { cache: 'no-store' }).catch(() => null);
  return response?.ok && response.headers.get('content-type')?.startsWith('application/json') ? ((await response.json()) as Device) : null;
};

const served: { device: Promise<Device | null> | null } = { device: null };

const servedDevice = (): Promise<Device | null> => (served.device ??= readServed());

const decide = async (app: string | null, hash: string): Promise<Handoff> => {
  if (app === null) return { stay: 'not-offered' };
  if (hashParams(hash).has(STAY_ON_DEVICE)) return { stay: 'asked-to-stay' };
  const [origin, appAnswered] = await Promise.all([trustedOrigin(), appAnswers(app)]);
  return handoffOf(app, origin, appAnswered, hash);
};

const parseDevice = (json: unknown): Device | null => (isJson(json) ? (json as Device) : null);

const upgradeRemembered = async (): Promise<void> => {
  const { origin } = deviceReading(await Qgc.get(deviceViewPath('aircraft')));
  if (origin.startsWith('http://')) await deviceInvoke(DEVICE_CONNECT, origin);
};

const linkDevice = async (): Promise<void> => {
  const { origin } = deviceReading(await Qgc.get(deviceViewPath('aircraft')));
  const device = origin === '' ? null : await deviceRead(DEVICE_JSON, parseDevice);
  if (device === null) return;
  const host = new URL(origin).hostname;
  await Promise.all([
    CameraCommands.adoptDevice(host, deviceCameras(device)),
    LinkCommands.createWebSocket(deviceLinkName(device, host), deviceSocket(resolveDeviceUrl(origin, '/fly/'))),
  ]);
};

const offerHandedOff = async (): Promise<void> => {
  const asked = connectAsked(location.hash);
  if (asked === null) return dropConnect();
  const { origin } = deviceReading(await Qgc.get(deviceViewPath('aircraft')));
  if (origin !== asked) return void (connectOffered.value = asked);
  dropConnect();
  recordHandoff(`arrived outcome=remembered origin=${new URL(asked).host}`);
};

export const DeviceLink = {
  handoff(): Promise<Handoff> {
    if (!location.pathname.startsWith(DEVICE_PAGE)) return Promise.resolve({ stay: 'not-a-device-page' });
    return servedDevice()
      .then((device) => decide(offeredApp(device), location.hash))
      .catch((): Handoff => ({ stay: 'failed' }));
  },

  recordStay(handoff: Handoff): void {
    if ('stay' in handoff && REPORTED_STAYS.includes(handoff.stay)) recordHandoff(`stayed reason=${handoff.stay} page=${location.host}`);
  },

  async acceptHandoff(origin: string): Promise<string | null> {
    const refused = await DeviceLink.connect(origin);
    recordHandoff(`arrived outcome=${refused === null ? 'connected' : 'refused'} origin=${new URL(origin).host}${refused === null ? '' : ` refusal=${JSON.stringify(refused)}`}`);
    if (refused === null) {
      dropConnect();
      connectOffered.value = null;
    }
    return refused;
  },

  declineHandoff(origin: string): void {
    recordHandoff(`arrived outcome=declined origin=${new URL(origin).host}`);
    dropConnect();
    connectOffered.value = null;
  },

  async start(): Promise<void> {
    const device = await servedDevice();
    if (device === null) {
      await upgradeRemembered();
      await offerHandedOff();
      return linkDevice();
    }
    deviceServed.value = true;
    await Promise.all([
      DeviceCommands.pair(deviceBase(location.href)),
      LinkCommands.createWebSocket(deviceLinkName(device, location.hostname), deviceSocket(location.href)),
      CameraCommands.adoptDevice(location.hostname, deviceCameras(device)),
      AppCommands.markFirstRunShown(),
    ]);
  },

  async connect(address: string): Promise<string | null> {
    const refused = refusal(await deviceInvoke(DEVICE_CONNECT, address), DEVICE_SUBJECT);
    if (refused === null) await linkDevice();
    return refused;
  },

  async disconnect(): Promise<void> {
    await deviceInvoke(DEVICE_DISCONNECT);
    location.reload();
  },
};
