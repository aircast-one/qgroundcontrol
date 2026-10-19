import { describe, expect, it } from 'vitest';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { cameraHoldsTheWizard, cameraStuck, clampStep, parseSetup, setupNav } from '@/ui/DeviceSetup';

const served = {
  start: 3,
  steps: [
    { key: 'network', label: 'Internet', title: 'Connect to the internet', lead: 'This device needs internet.', status: { level: 0, text: 'ready' } },
    { key: 'tailscale', label: 'Remote access', title: 'Remote access', lead: 'Reach it from anywhere.' },
    { key: 'camera', label: 'Camera', title: 'Camera', lead: 'Point it at the camera.' },
    { key: 'telemetry', label: 'Telemetry', title: 'Telemetry', lead: 'Plug the flight controller in.' },
    { key: 'done', label: 'Done', title: 'Almost there', lead: 'One item still needs attention.' },
  ],
  internet: {
    online: { level: 0, text: 'Online — this device can reach the internet.' },
    joinOpen: false,
    reopen: { host: 'falcon.local', hostname: 'falcon' },
    connection: { title: 'Connection', href: '/network', icon: 'network', summary: 'home-wifi' },
    hint: 'You can continue either way.',
  },
  remote: { status: { level: 2, text: 'offline' }, summary: 'no remote access', facts: [], addresses: [], cancel: false, connect: 'Connect to Tailscale' },
  camera: {
    problem: 'Camera offline — the source is saved but no video is arriving.',
    wizard: { name: 'cam', new: false, source: 'rtsp://cam', interfaces: [], transports: [], others: [], templates: [], subnets: [] },
    cameras: { title: 'Cameras', href: '/streaming', icon: 'video', summary: '1 camera, 1 dark', needs: true },
    hint: 'The rest lives on Cameras.',
  },
  telemetry: {
    waiting: { level: 3, text: 'Waiting for a flight controller on USB.' },
    endpoints: [{ proto: 'udps', label: 'MAVLink', url: 'falcon.tail.ts.net:14550' }],
    hint: 'Point your ground station at this UDP address.',
  },
  checks: [
    { label: 'Internet', status: { level: 0, text: 'ready' }, step: 1 },
    { label: 'Camera stream', status: { level: 3, text: 'not set up' }, step: 3 },
  ],
};

describe('DeviceSetupTest', () => {
  it('reads the walkthrough the device serves', () => {
    const view = parseSetup(served);
    expect(view?.start).toBe(3);
    expect(view?.steps.map((step) => step.key)).toEqual(['network', 'tailscale', 'camera', 'telemetry', 'done']);
    expect(view?.steps[0]?.status).toEqual({ level: DeviceLevel.Ready, text: 'ready' });
    expect(view?.steps[1]?.status).toBeNull();
    expect(view?.internet.reopen).toEqual({ host: 'falcon.local', hostname: 'falcon' });
    expect(view?.internet.connection.href).toBe('/network');
    expect(view?.remote.connect).toBe('Connect to Tailscale');
    expect(view?.camera.wizard.isNew).toBe(false);
    expect(view?.camera.watch).toBe('');
    expect(view?.camera.cameras.needs).toBe(true);
    expect(view?.telemetry.waiting?.text).toContain('Waiting for a flight controller');
    expect(view?.telemetry.endpoints[0]?.url).toBe('falcon.tail.ts.net:14550');
    expect(view?.checks[1]).toEqual({ label: 'Camera stream', status: { level: DeviceLevel.Neutral, text: 'not set up' }, step: 3 });
  });

  it('refuses a body that is not a walkthrough, and keeps a stray start on the trail', () => {
    expect(parseSetup(null)).toBeNull();
    expect(parseSetup({ ...served, steps: [] })).toBeNull();
    expect(parseSetup({ ...served, camera: {} })).toBeNull();
    expect(parseSetup({ ...served, start: 9 })?.start).toBe(5);
    expect(parseSetup({ ...served, start: 0 })?.start).toBe(1);
    expect(clampStep(6, 5)).toBe(5);
  });

  it('the footer offers what each step allows', () => {
    expect(setupNav(1, 5, false, false)).toEqual({ back: false, later: true, next: 'next', finish: false });
    expect(setupNav(3, 5, false, false)).toEqual({ back: true, later: true, next: 'next', finish: false });
    expect(setupNav(5, 5, false, false)).toEqual({ back: true, later: false, next: null, finish: true });
  });

  it('a camera with a problem leaves Reconfigure as the one thing to press, and moving on is a quiet skip', () => {
    expect(cameraStuck('camera', 'Camera offline — the source is saved but no video is arriving.')).toBe(true);
    expect(cameraStuck('camera', '')).toBe(false);
    expect(cameraStuck('network', 'No uplink')).toBe(false);
    expect(setupNav(3, 5, false, true)).toEqual({ back: true, later: true, next: 'skip', finish: false });
  });

  it('a camera being edited holds the wizard until it is applied or skipped', () => {
    expect(cameraHoldsTheWizard('camera', true, 1)).toBe(true);
    expect(cameraHoldsTheWizard('camera', true, 2)).toBe(true);
    expect(cameraHoldsTheWizard('camera', true, 3)).toBe(false);
    expect(cameraHoldsTheWizard('camera', false, 1)).toBe(false);
    expect(cameraHoldsTheWizard('telemetry', true, 1)).toBe(false);
    expect(setupNav(3, 5, true, false)).toEqual({ back: false, later: true, next: 'skip', finish: false });
  });
});
