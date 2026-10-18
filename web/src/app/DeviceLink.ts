import { AppCommands, CameraCommands, DeviceCommands, LinkCommands } from '@/bridge/BridgeCommands';
import { followDeviceStatus } from '@/ui/DeviceStatus';

type Device = { name?: string; cameras?: { name: string; url: string }[] };

export const deviceSocket = (page: string): string => {
  const url = new URL('mavlink', page);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return url.href;
};

export const deviceBase = (page: string): string => new URL('.', page).href;

export const deviceLinkName = (device: Device, hostname: string): string => `Aircast ${device.name?.trim() || hostname}`;

export const deviceCameras = (device: Device): [string, string][] => (device.cameras ?? []).map(({ name, url }) => [name, url]);

const servedBy = async (): Promise<Device | null> => {
  const response = await fetch('device.json', { cache: 'no-store' }).catch(() => null);
  return response?.ok && response.headers.get('content-type')?.startsWith('application/json') ? ((await response.json()) as Device) : null;
};

export const DeviceLink = {
  async start(): Promise<void> {
    const device = await servedBy();
    if (device === null) return;
    followDeviceStatus();
    await Promise.all([
      DeviceCommands.pair(deviceBase(location.href)),
      LinkCommands.createWebSocket(deviceLinkName(device, location.hostname), deviceSocket(location.href)),
      CameraCommands.adoptDevice(location.hostname, deviceCameras(device)),
      AppCommands.markFirstRunShown(),
    ]);
  },
};
