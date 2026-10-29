import { describe, expect, it } from 'vitest';
import { NO_FORM, continueLeads, parseCameraWizard, parseDeviceWizard, parseWizardForm, parseWizardNext, sameCamera, scanLeads } from '@/ui/DeviceCameraWizard';

describe('DeviceCameraWizardTest', () => {
  it('reads the wizard the device serves', () => {
    const view = parseCameraWizard({
      name: 'cam2',
      title: 'cam2 is not answering',
      new: false,
      source: 'rtsp://192.168.8.10:554/stream',
      rtsp: '',
      rtspHint: 'rtsp://192.168.8.10:554/stream',
      lookingFor: 'The camera at 192.168.8.10:554 refuses the connection.',
      checks: ["The RTSP port in the camera's app is the one in its address — 192.168.8.10:554", 7],
      interface: 'eth0',
      interfaces: [
        {
          value: 'eth0',
          label: 'Ethernet port (eth0) — 192.168.8.2/24',
          address: '192.168.8.2/24',
        },
      ],
      piCamera: { note: 'A Raspberry Pi camera is connected to this device.', action: 'Use it' },
      others: [{ name: 'cam', source: 'rtsp://x' }],
    });
    expect(view?.iface).toBe('eth0');
    expect(view?.title).toBe('cam2 is not answering');
    expect(view?.checks).toEqual(["The RTSP port in the camera's app is the one in its address — 192.168.8.10:554"]);
    expect(parseCameraWizard({ name: 'cam' })?.checks).toEqual([]);
    expect(view?.interfaces[0]?.address).toBe('192.168.8.2/24');
    expect(view?.piCamera?.action).toBe('Use it');
    expect(view?.json.others).toEqual([{ name: 'cam', source: 'rtsp://x' }]);
    expect(parseCameraWizard(null)).toBeNull();
  });

  it('reads what the core made of the scan, the ports and the running setup', () => {
    const wizard = parseDeviceWizard({
      again: 7,
      session: 3,
      scanning: false,
      scans: 2,
      note: '',
      checklist: 'No camera answered — check it, then scan again',
      adopt: { host: '192.168.8.10', url: 'rtsp://192.168.8.10:554/stream1', needsAuth: false, pathFound: true, used: '', note: 'ready to use', action: 'Use' },
      cams: [],
      camHeading: '0 cameras',
      camsHint: '',
      details: 'Details for a technician',
      todoHeading: 'Try this',
      todo: [
        {
          iface: 'eth0',
          label: 'Ethernet port (eth0)',
          detail: 'no address on this port',
          open: true,
          suggest: '192.168.144.2/24',
          hint: 'Prefilled.',
          go: 'Set up eth0 and find the camera',
          options: [{ cidr: '192.168.144.2/24', text: '192.168.144.2/24 · found 192.168.144.25' }],
          busy: true,
          status: 'Setting up eth0…',
        },
      ],
      rest: [{ iface: 'wlan0', label: 'Wi-Fi (wlan0)', detail: "this is the device's own uplink" }],
      restTitle: 'Wi-Fi (wlan0) — nothing answered',
      templates: [{ id: 'siyi', label: 'SIYI camera', sub: 'ZT30', action: 'Set up' }],
      templateBusy: false,
      templateNote: '',
      template: { n: 1, refusal: '', camera: null, name: 'siyi' },
      apply: { n: 0, state: 'failed', text: 'That name is taken', field: 'name', url: '', camera: '' },
      notice: { n: 1, text: 'Gimbal control was not switched to this camera — no answer' },
    });
    expect(wizard.session).toBe(3);
    expect(wizard.scans).toBe(2);
    expect(wizard.again).toBe(7);
    expect(wizard.adopt?.url).toBe('rtsp://192.168.8.10:554/stream1');
    expect(wizard.todo[0]?.options[0]).toEqual({ cidr: '192.168.144.2/24', text: '192.168.144.2/24 · found 192.168.144.25' });
    expect(wizard.todo[0]?.busy).toBe(true);
    expect(wizard.rest[0]?.detail).toBe("this is the device's own uplink");
    expect(wizard.templates[0]).toEqual({ id: 'siyi', label: 'SIYI camera', sub: 'ZT30', action: 'Set up' });
    expect(wizard.template).toEqual({ n: 1, refusal: '', camera: null, name: 'siyi' });
    expect(wizard.apply).toEqual({ n: 0, state: 'failed', text: 'That name is taken', field: 'name', url: '', camera: '' });
    expect(wizard.notice.n).toBe(1);
    expect(parseDeviceWizard(null).session).toBe(-1);
  });

  it('reads what Continue does next', () => {
    const refused = parseWizardNext({
      kind: 'refuse',
      rtsp: 'rtsp://192.168.8.10:554/stream',
      name: null,
      camera: 'cam',
      refusal: { text: 'Already used', warn: true, field: null },
      dupAck: 'rtsp://192.168.8.10:554/stream',
      iface: '',
      ip: '',
    });
    expect(refused?.refusal).toEqual({ text: 'Already used', warn: true, field: null });
    expect(refused?.name).toBeNull();
    expect(parseWizardNext({ kind: 'port', rtsp: 'rtsp://192.168.50.99/s', name: 'cam', camera: 'cam', refusal: null, dupAck: '', iface: 'eth0', ip: '192.168.50.2/24' })).toEqual({
      kind: 'port',
      rtsp: 'rtsp://192.168.50.99/s',
      name: 'cam',
      camera: 'cam',
      refusal: null,
      dupAck: '',
      iface: 'eth0',
      ip: '192.168.50.2/24',
    });
    expect(parseWizardForm({ next: 'Add camera', skipsPort: true, applyNote: '' })).toEqual({ next: 'Add camera', skipsPort: true, applyNote: '' });
    expect(parseWizardForm(null)).toEqual(NO_FORM);
  });

  it('a camera stays picked across views of the same scan', () => {
    const cam = { host: '192.168.8.10', url: 'rtsp://192.168.8.10:554/stream1', needsAuth: false, pathFound: true, used: '', note: 'ready to use', action: 'Use' };
    expect(sameCamera({ ...cam }, cam)).toBe(true);
    expect(sameCamera({ ...cam, host: '192.168.8.11' }, cam)).toBe(false);
    expect(sameCamera(null, cam)).toBe(false);
  });

  it('Scan leads until an address is filled, then Continue takes over', () => {
    expect(scanLeads('', false)).toBe(true);
    expect(continueLeads('')).toBe(false);
    expect(scanLeads('rtsp://192.168.8.99:554/stream', false)).toBe(false);
    expect(continueLeads('rtsp://192.168.8.99:554/stream')).toBe(true);
    expect(scanLeads('', true)).toBe(false);
  });
});
