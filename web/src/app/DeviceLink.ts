import { AppCommands, CameraCommands, DeviceCommands, LinkCommands } from '@/bridge/BridgeCommands';
import { isJson, optText } from '@/bridge/Json';
import { Qgc, refusal } from '@/bridge/Qgc';
import { mutableStateOf } from '@/kotlin/State';
import { REACH_TIMEOUT_MS } from '@/ui/DeviceConnect';
import { DEVICE_CONNECT, DEVICE_DISCONNECT, deviceInvoke, deviceRead, deviceReading, deviceViewPath, resolveDeviceUrl } from '@/ui/DeviceView';

type Device = { name?: string; cameras?: { name: string; url: string }[] };

const DEVICE_JSON = '/fly/device.json';

const DEVICE_SUBJECT = 'The device';

export const deviceSocket = (page: string): string => {
  const url = new URL('mavlink', page);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return url.href;
};

export const deviceBase = (page: string): string => new URL('.', page).href;

export const securePage = (page: string, secureOrigin: string): string | null => {
  const at = new URL(page);
  return at.protocol === 'http:' && secureOrigin.startsWith('https://') ? `${new URL(secureOrigin).origin}${at.pathname}${at.search}${at.hash}` : null;
};

export const deviceLinkName = (device: Device, hostname: string): string => `Aircast ${device.name?.trim() || hostname}`;

export const deviceCameras = (device: Device): [string, string][] => (device.cameras ?? []).map(({ name, url }) => [name, url]);

export const deviceServed = mutableStateOf(false);

const readServed = async (): Promise<Device | null> => {
  const response = await fetch('device.json', { cache: 'no-store' }).catch(() => null);
  return response?.ok && response.headers.get('content-type')?.startsWith('application/json') ? ((await response.json()) as Device) : null;
};

const parseDevice = (json: unknown): Device | null => (isJson(json) ? (json as Device) : null);

const servedSecureOrigin = async (): Promise<string> => {
  const answer: unknown = await fetch('/api/tls', { cache: 'no-store' })
    .then((response) => (response.ok ? response.json() : null))
    .catch(() => null);
  return optText(isJson(answer) ? answer : null, 'origin');
};

const reachable = (page: string): Promise<boolean> =>
  fetch(new URL('/api/tls', page), { mode: 'no-cors', cache: 'no-store', signal: AbortSignal.timeout(REACH_TIMEOUT_MS) }).then(
    () => true,
    () => false,
  );

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

export const DeviceLink = {
  async start(): Promise<void> {
    const device = await readServed();
    if (device === null) {
      await upgradeRemembered();
      return linkDevice();
    }
    const secure = securePage(location.href, await servedSecureOrigin());
    if (secure !== null && (await reachable(secure))) return location.replace(secure);
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
