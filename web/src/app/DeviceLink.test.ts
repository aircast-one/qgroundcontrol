import { describe, expect, it } from 'vitest';
import { deviceBase, deviceCameras, deviceLinkName, deviceSocket, securePage, typedOrigin } from './DeviceLink';

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

  it('moves a page the device served over http to the https address it names, keeping where the pilot was', () => {
    const secure = 'https://192-168-1-50.6f1c2a9e-3b7d-4e21-9a55-0c8d7e6f5a41.d.aircast.one';
    expect(securePage('http://192.168.1.50/fly/?x=1#plan', secure)).toBe(`${secure}/fly/?x=1#plan`);
    expect(securePage('http://192.168.1.50/fly/', `${secure}/ignored/path`)).toBe(`${secure}/fly/`);
    expect(securePage(`${secure}/fly/`, secure)).toBeNull();
    expect(securePage('http://192.168.1.50/fly/', '')).toBeNull();
    expect(securePage('http://192.168.1.50/fly/', 'http://192.168.1.50')).toBeNull();
  });

  it('asks a typed plain-http address for its https origin, and nothing else', () => {
    expect(typedOrigin(' 192.168.1.100 ')).toBe('http://192.168.1.100');
    expect(typedOrigin('falcon.local:8080/fly/')).toBe('http://falcon.local:8080');
    expect(typedOrigin('http://Falcon.Local/')).toBe('http://falcon.local');
    expect(typedOrigin('https://192-168-1-100.a.d.aircast.one')).toBeNull();
    expect(typedOrigin('')).toBeNull();
    expect(typedOrigin('not a host at all/')).toBeNull();
  });

  it('hands the core each camera as a name and the WHEP address the device gave', () => {
    expect(deviceCameras({ cameras: [{ name: 'siyi', url: 'http://192.168.1.100:8889/siyi/whep' }] })).toEqual([['siyi', 'http://192.168.1.100:8889/siyi/whep']]);
    expect(deviceCameras({})).toEqual([]);
  });
});
