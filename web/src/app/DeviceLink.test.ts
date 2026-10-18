import { describe, expect, it } from 'vitest';
import { deviceAddress, deviceBase, deviceCameras, deviceLinkName, deviceSocket, telemetryReachable, unreachableText } from './DeviceLink';
import { flyHereText } from '@/ui/DeviceScreen';

describe('the link to the device serving this page', () => {
  it('reaches telemetry beside the page, on the address the browser already used', () => {
    expect(deviceSocket('http://falcon.local/fly/')).toBe('ws://falcon.local/fly/mavlink');
    expect(deviceSocket('http://192.168.1.100/fly/index.html?x=1#y')).toBe('ws://192.168.1.100/fly/mavlink');
    expect(deviceSocket('https://falcon.tail.ts.net/fly/')).toBe('wss://falcon.tail.ts.net/fly/mavlink');
  });

  it('pairs the core with the folder the device served the page from', () => {
    expect(deviceBase('http://falcon.local/fly/')).toBe('http://falcon.local/fly/');
    expect(deviceBase('http://192.168.1.100/fly/index.html?x=1#y')).toBe('http://192.168.1.100/fly/');
  });

  it('is named after the device, or the address when the device has no name', () => {
    expect(deviceLinkName({ name: 'falcon' }, '192.168.1.100')).toBe('Aircast falcon');
    expect(deviceLinkName({ name: ' ' }, '192.168.1.100')).toBe('Aircast 192.168.1.100');
    expect(deviceLinkName({}, 'falcon.local')).toBe('Aircast falcon.local');
  });

  it('hands the core each camera as a name and the WHEP address the device gave', () => {
    expect(deviceCameras({ cameras: [{ name: 'siyi', url: 'http://192.168.1.100:8889/siyi/whep' }] })).toEqual([['siyi', 'http://192.168.1.100:8889/siyi/whep']]);
    expect(deviceCameras({})).toEqual([]);
  });
});

describe('a device typed into an app served from elsewhere', () => {
  it('takes the address the way an operator writes it', () => {
    expect(deviceAddress('192.168.1.100')).toBe('http://192.168.1.100');
    expect(deviceAddress(' falcon.local ')).toBe('http://falcon.local');
    expect(deviceAddress('falcon.local:8080/fly/')).toBe('http://falcon.local:8080');
    expect(deviceAddress('https://falcon.tail1234.ts.net/settings')).toBe('https://falcon.tail1234.ts.net');
    expect(deviceAddress('')).toBeNull();
    expect(deviceAddress('ftp://falcon.local')).toBeNull();
  });

  it('opens telemetry only where the browser lets the page hold it', () => {
    expect(telemetryReachable('https://fly.aircast.one/', 'http://192.168.1.100')).toBe(false);
    expect(telemetryReachable('https://fly.aircast.one/', 'https://falcon.tail1234.ts.net')).toBe(true);
    expect(telemetryReachable('http://localhost:8800/', 'http://192.168.1.100')).toBe(true);
    expect(flyHereText('http://192.168.1.100')).toContain('http://192.168.1.100/fly/');
    expect(unreachableText('http://192.168.1.100')).toMatch(/^192\.168\.1\.100 did not answer/);
  });
});
