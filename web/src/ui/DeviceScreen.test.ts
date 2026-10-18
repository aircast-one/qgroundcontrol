import { describe, expect, it } from 'vitest';
import { chipText, framedElsewhere, framedTitle, openedTitle } from '@/ui/DeviceScreen';
import { DEVICE_PAGE, DeviceLevel, deviceNeedsAttention, devicePageIcon, parseDeviceStatus } from '@/ui/DeviceStatus';
import { SettingsGroup, SettingsGroupEntries, pageLook, sheetGroups } from '@/ui/SettingsScreen';
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
    expect(parseDeviceStatus(null)).toBeNull();
    expect(parseDeviceStatus({ name: 'falcon' })).toBeNull();
  });

  it('a device page opens under the title of the row that opened it', () => {
    const status = parseDeviceStatus(served);
    expect(openedTitle(status, '/streaming')).toBe('Cameras');
    expect(openedTitle(status, '/streaming?camera=cam')).toBe('Camera');
    expect(openedTitle(null, '/network')).toBe('Device');
    expect(framedTitle('aircast · Connection', 'Cameras')).toBe('Connection');
    expect(framedTitle('', 'Cameras')).toBe('Cameras');
    expect(devicePageIcon('video')).toBe('ic_videocam');
    expect(devicePageIcon('unheard-of')).toBe('ic_settings');
  });

  it('back walks the framed pages before it leaves them', () => {
    const at = (url: string) => new URL(url) as unknown as Location;
    expect(framedElsewhere('/streaming?camera=cam', at('http://falcon.local/streaming?camera=cam'))).toBe(false);
    expect(framedElsewhere('/streaming?camera=cam', at('http://falcon.local/settings/camera?camera=cam'))).toBe(true);
    expect(framedElsewhere('/network', undefined)).toBe(false);
  });

  it('the Device tab is there only once the device has answered', () => {
    expect(sheetGroups(false)).toEqual(SettingsGroupEntries);
    expect(sheetGroups(true).at(-1)).toBe(SettingsGroup.Device);
    expect(sheetGroups(true).map((group) => group.ordinal)).toEqual(sheetGroups(true).map((_, index) => index));
    expect(openingGroup(DEVICE_PAGE)).toBe(SettingsGroup.Device);
    expect(pageLook(DEVICE_PAGE).inline).toBe(true);
  });
});
