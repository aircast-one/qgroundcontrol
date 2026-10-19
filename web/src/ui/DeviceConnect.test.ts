import { describe, expect, it } from 'vitest';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { chosenGcs, parseConnect, probeBlocked, shownTargets } from '@/ui/DeviceConnect';

const served = {
  choices: [
    { value: 'rtsp', label: 'QGroundControl', hides: ['WHEP'] },
    { value: 'whep', label: 'Browser or Aircast QGC', hides: ['RTSP'] },
    { value: 'all', label: 'Show all', hides: [] },
  ],
  scope: 'Every link reaches this device at falcon.tail.ts.net.',
  cameras: [
    {
      name: 'cam2',
      status: { level: 2, text: 'unreachable' },
      notice: 'The link below is still the right one.',
      fix: { label: 'Fix cam2 →', href: '/streaming?camera=cam2' },
      targets: [
        { proto: 'RTSP', label: 'QGroundControl · RTSP', url: 'rtsp://falcon:8554/cam2', hint: 'Paste it.', qr: true, down: true, alternatives: [{ label: 'Ethernet port (eth0)', url: 'rtsp://192.168.1.5:8554/cam2' }] },
        { proto: 'WHEP', label: 'Browser · WHEP', url: 'http://falcon:8889/cam2/whep', reach: 'http://falcon:8889/cam2/whep', qr: true, alternatives: [] },
      ],
    },
  ],
  telemetry: { status: { level: 0, text: 'connected' }, targets: [{ proto: 'UDP', label: 'UDP · MAVLink', url: 'falcon:14550', qr: false, alternatives: [] }] },
  trouble: { fallback: [{ proto: 'HLS', label: 'Web players and Safari', url: 'http://falcon:8888/cam2/index.m3u8', tag: 'High latency', qr: true, alternatives: [] }] },
};

describe('DeviceConnectTest', () => {
  it('reads the links the device serves', () => {
    const view = parseConnect(served);
    const camera = view?.cameras[0];
    expect(camera?.status).toEqual({ level: DeviceLevel.Blocked, text: 'unreachable' });
    expect(camera?.fix).toEqual({ label: 'Fix cam2 →', href: '/streaming?camera=cam2' });
    expect(camera?.targets[0]).toMatchObject({ proto: 'RTSP', down: true, reach: '', qr: true, tag: '' });
    expect(camera?.targets[0]?.alternatives).toEqual([{ label: 'Ethernet port (eth0)', url: 'rtsp://192.168.1.5:8554/cam2' }]);
    expect(camera?.targets[1]?.reach).toBe('http://falcon:8889/cam2/whep');
    expect(view?.telemetry?.status.text).toBe('connected');
    expect(view?.trouble?.fallback[0]?.tag).toBe('High latency');
    expect(view?.trouble?.addresses).toBeNull();
    expect(view?.noCamera).toBeNull();
    expect(parseConnect({ cameras: [] })).toBeNull();
    expect(parseConnect({ choices: [], cameras: [], noCamera: { label: 'Add a camera', href: '/streaming' } })?.noCamera).toEqual({ label: 'Add a camera', href: '/streaming' });
  });

  it('the ground station the operator named decides which links are on offer', () => {
    const view = parseConnect(served);
    const choices = view?.choices ?? [];
    const targets = view?.cameras[0]?.targets ?? [];
    expect(chosenGcs(choices, null)?.value).toBe('rtsp');
    expect(chosenGcs(choices, 'nonsense')?.value).toBe('rtsp');
    expect(shownTargets(targets, chosenGcs(choices, null)).map((it) => it.proto)).toEqual(['RTSP']);
    expect(shownTargets(targets, chosenGcs(choices, 'whep')).map((it) => it.proto)).toEqual(['WHEP']);
    expect(shownTargets(targets, chosenGcs(choices, 'all')).map((it) => it.proto)).toEqual(['RTSP', 'WHEP']);
    expect(shownTargets(view?.telemetry?.targets ?? [], chosenGcs(choices, 'whep')).map((it) => it.proto)).toEqual(['UDP']);
    expect(chosenGcs([], 'rtsp')).toBeNull();
  });

  it('a link this page cannot probe gets no verdict', () => {
    expect(probeBlocked('', 'http:')).toBe(true);
    expect(probeBlocked('http://falcon:8889/cam2/whep', 'https:')).toBe(true);
    expect(probeBlocked('http://falcon:8889/cam2/whep', 'http:')).toBe(false);
    expect(probeBlocked('https://share.example/w/cam', 'https:')).toBe(false);
  });
});
