import { describe, expect, it } from 'vitest';
import { chipText } from '@/ui/DeviceScreen';
import { DEVICE_SCREENS, devicePath } from '@/ui/DeviceScreens';
import { deviceNeedsAttention, devicePageIcon, parseDeviceStatus } from '@/ui/DeviceStatus';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { DEVICE_PAGE } from '@/ui/DevicePage';
import { SettingsGroup, SettingsGroupEntries, pageLook } from '@/ui/SettingsScreen';
import { openingGroup } from '@/ui/SettingsSheet';

const served = {
  name: 'falcon',
  level: 2,
  headline: 'Camera is not sending video',
  detail: 'Check the cable.',
  action: { label: 'Check the camera', href: '/streaming?camera=cam' },
  checks: [{ label: 'Camera', value: 'no video', level: 2, href: '/streaming?camera=cam' }],
  pages: [
    { title: 'Ground station links', href: '/connect', icon: 'qr-code' },
    { title: 'Cameras', href: '/streaming', icon: 'video', summary: '1 camera, 1 dark', needs: true },
  ],
};

const aircraft = {
  ...served,
  setup: { label: 'Finish setting up', href: '/setup' },
  checks: [
    { label: 'Camera', value: 'no video', level: 2, href: '/streaming?camera=cam' },
    { label: 'Telemetry', value: 'waiting for autopilot', level: 1, href: '/telemetry' },
    { label: 'Remote access', value: 'not set up', level: 1, href: '/network#tailscale' },
  ],
  pages: [
    { title: 'Ground station links', href: '/connect', icon: 'qr-code' },
    { title: 'Cameras', href: '/streaming', icon: 'video' },
    { title: 'Video engine', href: '/settings/video', icon: 'sliders-horizontal' },
    { title: 'Connection', href: '/network', icon: 'network' },
    { title: 'Telemetry', href: '/telemetry', icon: 'activity' },
    { title: 'Recordings', href: '/settings/recordings', icon: 'gauge' },
    { title: 'Flight log', href: '/flights', icon: 'clock' },
    { title: 'Device', href: '/settings/device', icon: 'settings' },
    { title: 'Engineering', href: '/settings/engineering', icon: 'server' },
    { title: 'Sharing & recording', href: '/settings/camera?camera=cam', icon: 'sliders-horizontal' },
    { title: 'Set up this device step by step', href: '/setup', icon: 'scan-search' },
  ],
};

const SCREEN_SOURCES: Readonly<Record<string, string>> = import.meta.glob(['./Device*.tsx', './CameraWizard.tsx'], { query: '?raw', import: 'default', eager: true });

describe('DeviceScreenTest', () => {
  it('reads what the device says about itself', () => {
    const status = parseDeviceStatus(served);
    expect(status?.level).toBe(DeviceLevel.Blocked);
    expect(status?.action).toEqual({ label: 'Check the camera', href: '/streaming?camera=cam' });
    expect(status?.checks[0]).toEqual({ label: 'Camera', value: 'no video', level: 2, href: '/streaming?camera=cam', action: '' });
    expect(status?.pages.map((page) => page.needs)).toEqual([false, true]);
    expect(deviceNeedsAttention(status)).toBe(true);
    expect(status !== null && chipText(status)).toBe('falcon · Camera is not sending video');
    expect(deviceNeedsAttention(parseDeviceStatus({ ...served, level: 0, action: undefined }))).toBe(false);
    expect(parseDeviceStatus({ ...served, action: undefined })?.action).toBeNull();
    expect(status?.setup).toBeNull();
    expect(parseDeviceStatus({ ...served, setup: { label: 'Finish setting up', href: '/setup' } })?.setup).toEqual({ label: 'Finish setting up', href: '/setup' });
    expect(parseDeviceStatus(null)).toBeNull();
    expect(parseDeviceStatus({ name: 'falcon' })).toBeNull();
  });

  it('every device page has its own icon, and an unknown one falls back', () => {
    expect(devicePageIcon('video')).toBe('ic_videocam');
    expect(devicePageIcon('scan-search')).toBe('ic_build');
    expect(devicePageIcon('unheard-of')).toBe('ic_settings');
  });

  it('every page the device links to, and every page a screen opens, is drawn natively', () => {
    const native = (href: string) => DEVICE_SCREENS.has(devicePath(href).path);
    const status = parseDeviceStatus(aircraft);
    const linked = [status?.action?.href, status?.setup?.href, ...(status?.checks.map((check) => check.href) ?? []), ...(status?.pages.map((page) => page.href) ?? [])].filter((href): href is string => href !== undefined && href !== '');
    expect(linked.length).toBe(16);
    expect(linked.filter((href) => !native(href))).toEqual([]);
    const opened = Object.values(SCREEN_SOURCES).flatMap((source) => [...source.matchAll(/onOpen\('([^']+)'\)/g)].map((match) => match[1] ?? ''));
    expect(opened.length).toBeGreaterThan(5);
    expect(opened.filter((href) => !native(href))).toEqual([]);
  });

  it('the Device tab is always there, and a requested device page opens on it', () => {
    expect(SettingsGroupEntries.at(-1)).toBe(SettingsGroup.Device);
    expect(SettingsGroupEntries.map((group) => group.ordinal)).toEqual(SettingsGroupEntries.map((_, index) => index));
    expect(openingGroup(DEVICE_PAGE)).toBe(SettingsGroup.Device);
    expect(pageLook(DEVICE_PAGE).inline).toBe(true);
  });
});

describe('DeviceHomeTest', () => {
  it('the device home carries the link and the health the flight sheet showed', () => {
    const status = parseDeviceStatus({
      ...served,
      link: { carrier: 'Wi-Fi (wlan0)', rate: '↓ 1 kB/s', standby: 'Magti · lte', charts: [{ label: 'Wi-Fi', values: [60, 64], max: 100, value: '64%', level: 3 }] },
      health: { memory: '400 MB of 1 GB', warning: { level: 1, text: 'Undervoltage seen since start' }, charts: [{ label: 'CPU', values: [10, 92], max: 100, value: '92%', level: 2 }] },
      events: '2 events this flight',
    });
    expect(status?.link?.carrier).toBe('Wi-Fi (wlan0)');
    expect(status?.link?.charts[0]).toEqual({ label: 'Wi-Fi', values: [60, 64], max: 100, value: '64%', level: 3 });
    expect(status?.health?.warning).toEqual({ level: 1, text: 'Undervoltage seen since start' });
    expect(status?.health?.charts[0]?.level).toBe(DeviceLevel.Blocked);
    expect(status?.events).toBe('2 events this flight');
    expect(parseDeviceStatus(served)?.link).toBeNull();
  });
});
