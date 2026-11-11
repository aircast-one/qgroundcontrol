import { describe, expect, it } from 'vitest';
import { connectAsked, deviceBase, deviceCameras, deviceLinkName, deviceSocket, handoffUrl, handsOff, withoutConnect } from './DeviceLink';

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
  const origin = 'https://100-64-0-15.falcon-gb3v.d.aircast.one';

  it('hands off only when the device names an app and the page was not asked to stay', () => {
    expect(handsOff({ app: 'https://fly.aircast.one' }, '')).toBe(true);
    expect(handsOff({ app: 'https://fly.aircast.one' }, '#local')).toBe(false);
    expect(handsOff({ app: 'https://fly.aircast.one' }, '#device=%2Fflights&local')).toBe(false);
    expect(handsOff({ name: 'falcon' }, '')).toBe(false);
    expect(handsOff(null, '')).toBe(false);
  });

  it('opens the hosted app at its root, asked to connect to the device and keeping the device page asked for', () => {
    const url = handoffUrl('https://fly.aircast.one', origin, '#device=%2Fflights%3Fflight%3D7');
    expect(url.startsWith('https://fly.aircast.one/#')).toBe(true);
    expect(connectAsked(new URL(url).hash)).toBe(origin);
    expect(new URLSearchParams(new URL(url).hash.slice(1)).get('device')).toBe('/flights?flight=7');
  });

  it('forgets the connect request once taken, leaving the rest of the hash', () => {
    const hash = new URL(handoffUrl('https://fly.aircast.one', origin, '#device=%2Fflights')).hash;
    expect(withoutConnect(hash)).toBe('#device=%2Fflights');
    expect(withoutConnect(new URL(handoffUrl('https://fly.aircast.one', origin, '')).hash)).toBe('');
    expect(connectAsked('#device=%2Fflights')).toBeNull();
  });
});
