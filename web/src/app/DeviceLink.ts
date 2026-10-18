import { AppCommands, CameraCommands, DeviceCommands, LinkCommands } from '@/bridge/BridgeCommands';
import { getSharedPreferences } from '@/kotlin/Platform';
import { mutableStateOf } from '@/kotlin/State';
import { deviceAnswered, deviceStatus, followDeviceStatus } from '@/ui/DeviceStatus';
import { deviceOrigin, resolveDeviceUrl } from '@/ui/DeviceView';

type Device = { name?: string; cameras?: { name: string; url: string }[] };

export const deviceSocket = (page: string): string => {
  const url = new URL('mavlink', page);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return url.href;
};

export const deviceBase = (page: string): string => new URL('.', page).href;

export const deviceLinkName = (device: Device, hostname: string): string => `Aircast ${device.name?.trim() || hostname}`;

export const deviceCameras = (device: Device): [string, string][] => (device.cameras ?? []).map(({ name, url }) => [name, url]);

export const deviceAddress = (typed: string): string | null => {
  const text = typed.trim();
  if (text === '') return null;
  try {
    const url = new URL(/^[a-z][a-z0-9+.-]*:\/\//i.test(text) ? text : `http://${text}`);
    return url.protocol === 'http:' || url.protocol === 'https:' ? url.origin : null;
  } catch {
    return null;
  }
};

export const telemetryReachable = (page: string, origin: string): boolean => new URL(page).protocol === 'http:' || new URL(origin).protocol === 'https:';

export const deviceServed = mutableStateOf(false);

export const deviceConnected = mutableStateOf<string | null>(null);

const following = mutableStateOf<(() => void) | null>(null);

const preferences = getSharedPreferences('device');

const REMEMBERED = 'origin';

const readDevice = async (url: string): Promise<Device | null> => {
  const response = await fetch(url, { cache: 'no-store' }).catch(() => null);
  return response?.ok && response.headers.get('content-type')?.startsWith('application/json') ? ((await response.json()) as Device) : null;
};

const followStatus = () => {
  following.value?.();
  following.value = followDeviceStatus();
};

export const unreachableText = (origin: string): string =>
  `${new URL(origin).host} did not answer as an Aircast device. Check it is on this network, and when the browser asks to let this site reach devices on your local network, allow it.`;

export const DeviceLink = {
  async start(): Promise<void> {
    const device = await readDevice('device.json');
    if (device !== null) {
      deviceServed.value = true;
      followStatus();
      await Promise.all([
        DeviceCommands.pair(deviceBase(location.href)),
        LinkCommands.createWebSocket(deviceLinkName(device, location.hostname), deviceSocket(location.href)),
        CameraCommands.adoptDevice(location.hostname, deviceCameras(device)),
        AppCommands.markFirstRunShown(),
      ]);
      return;
    }
    const remembered = preferences.getString(REMEMBERED, null);
    if (remembered !== null) await DeviceLink.connect(remembered);
  },

  async connect(origin: string): Promise<string | null> {
    const device = await readDevice(resolveDeviceUrl(origin, '/fly/device.json'));
    if (device === null) return unreachableText(origin);
    const host = new URL(origin).hostname;
    deviceOrigin.value = origin;
    deviceConnected.value = origin;
    preferences.putString(REMEMBERED, origin);
    followStatus();
    await Promise.all([
      CameraCommands.adoptDevice(host, deviceCameras(device)),
      telemetryReachable(location.href, origin) ? LinkCommands.createWebSocket(deviceLinkName(device, host), deviceSocket(resolveDeviceUrl(origin, '/fly/'))) : Promise.resolve(null),
    ]);
    return null;
  },

  disconnect(): void {
    following.value?.();
    following.value = null;
    preferences.remove(REMEMBERED);
    deviceOrigin.value = '';
    deviceConnected.value = null;
    deviceStatus.value = null;
    deviceAnswered.value = false;
    location.reload();
  },
};
