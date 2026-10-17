import { describe, expect, it } from 'vitest';
import { deviceCameras, deviceLinkName, deviceSocket } from './DeviceLink';

describe('the link to the device serving this page', () => {
  it('reaches telemetry beside the page, on the address the browser already used', () => {
    expect(deviceSocket('http://falcon.local/fly/')).toBe('ws://falcon.local/fly/mavlink');
    expect(deviceSocket('http://192.168.1.100/fly/index.html?x=1#y')).toBe('ws://192.168.1.100/fly/mavlink');
    expect(deviceSocket('https://falcon.tail.ts.net/fly/')).toBe('wss://falcon.tail.ts.net/fly/mavlink');
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
