import { describe, expect, it } from 'vitest';
import { connectAsked, deviceBase, deviceCameras, deviceLinkName, deviceSocket, handoffOf, handoffUrl, offeredApp, withoutConnect } from './DeviceLink';

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

describe('handing a tailnet browser to the hosted app', () => {
  const app = 'https://fly.aircast.one';
  const origin = 'https://100-64-0-15.falcon-gb3v.d.aircast.one';

  it('takes the hosted app only when the device names one', () => {
    expect(offeredApp({ app })).toBe(app);
    expect(offeredApp({ app: '' })).toBeNull();
    expect(offeredApp({ name: 'falcon' })).toBeNull();
    expect(offeredApp(null)).toBeNull();
  });

  it('hands over only when both the device and the hosted app answered, and says which did not', () => {
    expect(handoffOf(app, origin, true, '')).toEqual({ to: handoffUrl(app, origin, '') });
    expect(handoffOf(app, null, true, '')).toEqual({ stay: 'device-unreachable' });
    expect(handoffOf(app, origin, false, '')).toEqual({ stay: 'app-unreachable' });
  });

  it('opens the hosted app at its root, asked to connect to the device and keeping the device page asked for', () => {
    const url = handoffUrl(app, origin, '#device=%2Fflights%3Fflight%3D7');
    expect(url.startsWith('https://fly.aircast.one/#')).toBe(true);
    expect(connectAsked(new URL(url).hash)).toBe(origin);
    expect(new URLSearchParams(new URL(url).hash.slice(1)).get('device')).toBe('/flights?flight=7');
  });

  it('takes a connect request only for a bare https origin', () => {
    expect(connectAsked(`#connect=${encodeURIComponent('http://192.168.1.100')}`)).toBeNull();
    expect(connectAsked(`#connect=${encodeURIComponent('https://falcon.local/fly/')}`)).toBeNull();
    expect(connectAsked('#connect=javascript%3Aalert(1)')).toBeNull();
    expect(connectAsked('#device=%2Fflights')).toBeNull();
  });

  it('forgets the connect request once answered, leaving the rest of the hash', () => {
    expect(withoutConnect(new URL(handoffUrl(app, origin, '#device=%2Fflights')).hash)).toBe('#device=%2Fflights');
    expect(withoutConnect(new URL(handoffUrl(app, origin, '')).hash)).toBe('');
  });
});
