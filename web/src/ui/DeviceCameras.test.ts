import { describe, expect, it } from 'vitest';
import { type CameraView, cameBack, camerasShown, cloudflareTarget, liveness, openedSection, parseCameras } from '@/ui/DeviceCameras';
import { DeviceLevel } from '@/ui/DeviceLevel';

const wizard = {
  name: '',
  new: true,
  suggestedName: 'cam',
  interfaces: [],
  transports: [],
  others: [],
  templates: [],
  subnets: [],
};

const camera = (name: string, live: boolean) => ({
  name,
  status: { level: live ? 0 : 2, text: live ? 'live' : 'unreachable' },
  live,
  preview: live ? `/whep/${name}` : '',
  placeholder: live ? '' : 'Camera offline',
  watch: `/watch/${name}`,
  copies: [{ label: 'Copy watch link', text: `http://dev/watch/${name}`, qr: false }],
  facts: [{ label: 'Source', value: 'rtsp://10.0.0.1/s' }],
  bitrate: { label: 'Bitrate', values: [1, 2], max: 2, value: '16 bps', level: 3 },
  diagnosis: live ? undefined : { level: 2, text: 'Camera not responding' },
  fix: live ? undefined : { label: 'Scan for it', scan: true },
  links: [
    {
      proto: 'RTSP',
      label: 'QGroundControl · RTSP',
      url: `rtsp://dev.local:8554/${name}`,
      qr: true,
      alternatives: [{ label: 'Ethernet port (eth0)', url: `rtsp://192.168.1.5:8554/${name}` }],
    },
  ],
  chips: [
    {
      key: 'cloud',
      label: 'cloud connected',
      level: 0,
      href: `/settings/camera?camera=${name}#cam-watch`,
    },
  ],
  wizard: { ...wizard, name, new: false },
  setup: {
    watch: {
      part: {
        title: 'Watch from anywhere',
        summary: 'Ground station, browser, QR codes.',
        badge: { level: 1, text: 'cloud sign-in' },
        open: true,
      },
      cloud: true,
      signIn: true,
      cloudLinks: [],
      links: [],
    },
    radio: {
      part: { title: 'Video without cellular', summary: 's' },
      notice: {
        level: 3,
        text: 'needs the engine',
        link: { label: 'What carries video', href: '/settings/video' },
      },
      form: false,
    },
    servers: {
      part: { title: 'Your own server', summary: 's' },
      dests: [
        {
          kind: 'cloudflare',
          part: { title: 'Cloudflare Realtime', summary: 's' },
          on: true,
          app: 'a',
          track: 't',
          watch: `/watch/${name}?via=cloudflare`,
        },
      ],
    },
    shares: {
      part: { title: 'Share links', summary: 's' },
      links: [{ id: 'x', label: '—', url: 'http://dev/s/x', dead: 'Turned off 1 Jan 09:00' }],
      hours: [{ value: '24', label: '1 day' }],
    },
    detect: { part: { title: 'Detection overlay', summary: 's' }, on: false },
    record: { part: { title: 'Recording', summary: 's' }, on: true, full: true },
  },
  topic: {
    title: 'Sharing & recording',
    href: `/settings/camera?camera=${name}`,
    icon: 'sliders-horizontal',
    summary: 'where it goes',
  },
});

const served = {
  cameras: [camera('cam2', true), camera('cam3', false)],
  add: wizard,
  cloudLink: 'https://app/x',
  video: { title: 'Video engine', href: '/settings/video', icon: 'sliders-horizontal' },
};

const cameras = (): readonly CameraView[] => parseCameras(served)?.cameras ?? [];

describe('DeviceCamerasTest', () => {
  it('reads the cameras view the device serves', () => {
    const view = parseCameras(served);
    expect(view?.cameras.map((it) => it.name)).toEqual(['cam2', 'cam3']);
    expect(view?.cloudLink).toBe('https://app/x');
    expect(view?.add.isNew).toBe(true);
    const [live, dark] = view?.cameras ?? [];
    expect(live?.status).toEqual({ level: DeviceLevel.Ready, text: 'live' });
    expect(live?.preview).toBe('/whep/cam2');
    expect(live?.bitrate?.max).toBe(2);
    expect(live?.diagnosis).toBeNull();
    expect(live?.links[0]?.alternatives).toEqual([{ label: 'Ethernet port (eth0)', url: 'rtsp://192.168.1.5:8554/cam2' }]);
    expect(dark?.fix).toEqual({ label: 'Scan for it', scan: true });
    expect(dark?.diagnosis?.level).toBe(DeviceLevel.Blocked);
    expect(live?.setup.watch.part.badge).toEqual({ level: DeviceLevel.Warn, text: 'cloud sign-in' });
    expect(live?.setup.radio.notice.link).toEqual({
      label: 'What carries video',
      href: '/settings/video',
    });
    expect(live?.setup.servers.dests[0]).toMatchObject({
      kind: 'cloudflare',
      on: true,
      app: 'a',
      track: 't',
      watch: '/watch/cam2?via=cloudflare',
    });
    expect(live?.setup.shares.links[0]?.dead).toBe('Turned off 1 Jan 09:00');
    expect(live?.setup.detect.notice).toBeNull();
    expect(live?.setup.record).toMatchObject({ on: true, full: true, usage: '' });
    expect(live?.setup.gimbal).toBeNull();
    expect(live?.topic.href).toBe('/settings/camera?camera=cam2');
    expect(parseCameras({ cameras: [] })).toBeNull();
  });

  it('opens the only camera, walls several, and adds when asked or when there are none', () => {
    const all = cameras();
    const one = all.slice(0, 1);
    expect(camerasShown(one, '', false)).toMatchObject({ kind: 'camera', camera: { name: 'cam2' } });
    expect(camerasShown(all, '', false)).toEqual({ kind: 'wall' });
    expect(camerasShown(all, 'cam3', false)).toMatchObject({
      kind: 'camera',
      camera: { name: 'cam3' },
    });
    expect(camerasShown(all, 'gone', false)).toEqual({ kind: 'wall' });
    expect(camerasShown(all, 'cam3', true)).toEqual({ kind: 'add' });
    expect(camerasShown([], 'cam2', false)).toEqual({ kind: 'add' });
  });

  it('announces a camera only when it comes back from dark', () => {
    const before = liveness(cameras());
    expect(before.get('cam3')).toBe(false);
    const back = cameras().map((it) => ({ ...it, live: true }));
    expect(cameBack(before, back)).toEqual(['cam3']);
    expect(cameBack(new Map(), back)).toEqual([]);
    expect(cameBack(liveness(back), back)).toEqual([]);
  });

  it('composes the Cloudflare target from the app and track', () => {
    expect(cloudflareTarget(' app-123 ', 'cam2-track')).toBe('cloudflare://app-123/cam2-track');
    expect(cloudflareTarget('app-123', '')).toBe('');
  });

  it('a chip opens the setting it names', () => {
    expect(openedSection('/settings/camera?camera=cam2#cam-watch')).toBe('cam-watch');
    expect(openedSection('/settings/camera?camera=cam2')).toBe('');
  });
});
