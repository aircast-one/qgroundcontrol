import { describe, expect, it } from 'vitest';
import {
  type CameraSubnet,
  type CameraTemplate,
  type FoundCamera,
  type ScanPort,
  adoptedCamera,
  applyNote,
  camURL,
  checklistHint,
  cidrIn,
  commitsNow,
  continueLeads,
  fixerGoLabel,
  fixersFor,
  ipToInt,
  nextLabel,
  nextStep,
  normalizeRtsp,
  parseCameraWizard,
  parseFoundCameras,
  parseScanPorts,
  portDetail,
  portLabel,
  rtspHost,
  sameSubnet,
  scanLeads,
  scanOutcome,
  subnetOptions,
  suggestIP,
  templateAction,
  templateHolderError,
  templatePort,
  templateReadyPort,
  templateSub,
  todoHeading,
} from '@/ui/DeviceCameraWizard';

const port = (fields: Partial<ScanPort>): ScanPort => ({
  iface: 'eth0',
  label: 'Ethernet port (eth0)',
  type: 'ethernet',
  state: 'searched',
  subnet: '',
  checked: 0,
  reason: '',
  triedAddress: '',
  isUplink: false,
  neighbors: [],
  ...fields,
});
const SWEPT = port({ subnet: '192.168.8.0/24', checked: 253 });
const UPLINK = port({
  iface: 'wlan0',
  label: 'Wi-Fi (wlan0)',
  type: 'wi-fi',
  subnet: '192.168.1.0/24',
  checked: 253,
  isUplink: true,
});
const cam = (fields: Partial<FoundCamera>): FoundCamera => ({
  host: '192.168.8.10',
  url: 'rtsp://192.168.8.10:554/stream1',
  needsAuth: false,
  pathFound: true,
  used: '',
  ...fields,
});
const SUBNETS: readonly CameraSubnet[] = [
  { cidr: '192.168.144.2/24', label: 'SIYI, Topotek gimbals, Herelink' },
  { cidr: '192.168.8.2/24', label: 'LTE routers' },
];
const SIYI: CameraTemplate = {
  id: 'siyi',
  label: 'SIYI camera',
  hint: "ZT30 — plugged into this device's Ethernet port",
  cidr: '192.168.144.2/24',
  host: '192.168.144.25',
  name: 'siyi',
  gimbal: 'siyi',
  gimbalAddr: '192.168.144.25:37260',
};

describe('DeviceCameraWizardTest', () => {
  it('reads the wizard the device serves', () => {
    const view = parseCameraWizard({
      name: 'cam2',
      new: false,
      source: 'rtsp://192.168.8.10:554/stream',
      rtsp: '',
      rtspHint: 'rtsp://192.168.8.10:554/stream',
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
      subnets: SUBNETS,
    });
    expect(view?.iface).toBe('eth0');
    expect(view?.interfaces[0]?.address).toBe('192.168.8.2/24');
    expect(view?.piCamera?.action).toBe('Use it');
    expect(view?.others).toEqual([{ name: 'cam', source: 'rtsp://x' }]);
    expect(view?.subnets).toEqual(SUBNETS);
    expect(view?.templates).toEqual([]);
    expect(parseCameraWizard(null)).toBeNull();
  });

  it('reads the scan, and a camera this wizard already is reads as fresh', () => {
    const scan = {
      cameras: [
        {
          host: '192.168.8.4',
          port: 554,
          url: 'rtsp://192.168.8.4:554/s',
          pathFound: true,
          usedBy: 'cam',
        },
        {
          host: '192.168.8.5',
          port: 554,
          url: 'rtsp://192.168.8.5:554/s',
          pathFound: true,
          usedBy: 'cam2',
        },
      ],
      ports: [
        {
          interface: 'eth1',
          label: 'Ethernet port (eth1)',
          type: 'ethernet',
          state: 'searched',
          neighbors: [{ addr: '192.168.144.25', mac: 'x', suggest: '192.168.144.2/24' }],
        },
      ],
    };
    expect(parseFoundCameras(scan, 'cam2').map((it) => it.used)).toEqual(['cam', '']);
    expect(parseScanPorts(scan)[0]).toMatchObject({
      iface: 'eth1',
      neighbors: [{ addr: '192.168.144.25', suggest: '192.168.144.2/24' }],
    });
  });

  it('takes the host out of a camera URL', () => {
    expect(rtspHost('rtsp://192.168.8.10:554/stream')).toBe('192.168.8.10');
    expect(rtspHost('rtsp://cam.local/live')).toBe('cam.local');
    expect(rtspHost('not a url')).toBe('');
    expect(rtspHost('')).toBe('');
  });

  it('supplies the scheme, and never rewrites one that is there', () => {
    expect(normalizeRtsp('192.168.8.10')).toBe('rtsp://192.168.8.10');
    expect(normalizeRtsp('cam.local:554/live')).toBe('rtsp://cam.local:554/live');
    expect(normalizeRtsp('  192.168.8.10/stream  ')).toBe('rtsp://192.168.8.10/stream');
    expect(normalizeRtsp('rtsp://192.168.8.10/x')).toBe('rtsp://192.168.8.10/x');
    expect(normalizeRtsp('http://192.168.8.10')).toBe('http://192.168.8.10');
    expect(normalizeRtsp('')).toBe('');
  });

  it('never offers the address the camera already answers on', () => {
    expect(suggestIP('rtsp://192.168.8.2/live')).toBe('192.168.8.3/24');
    expect(suggestIP('rtsp://192.168.8.10/live')).toBe('192.168.8.2/24');
    expect(suggestIP('rtsp://cam.local/live')).toBe('');
    expect(suggestIP('garbage')).toBe('');
  });

  it('spots a subnet typed where a camera URL belongs', () => {
    expect(cidrIn('192.168.8.0/24')).toBe('192.168.8.0/24');
    expect(cidrIn(' rtsp://192.168.8.0/24 ')).toBe('192.168.8.0/24');
    expect(cidrIn('192.168.8.10')).toBe('');
    expect(cidrIn('rtsp://192.168.8.10/stream')).toBe('');
    expect(cidrIn('192.168.8.0/33')).toBe('');
  });

  it('refuses anything that is not four plain octets', () => {
    expect(ipToInt('0.0.0.0')).toBe(0);
    expect(ipToInt('255.255.255.255')).toBe(4294967295);
    expect(ipToInt('192.168.1.1')).toBe(3232235777);
    expect(ipToInt('1.2.3')).toBeNull();
    expect(ipToInt('1.2.3.256')).toBeNull();
    expect(ipToInt('1.2.3.')).toBeNull();
    expect(ipToInt('1.2.3. 4')).toBeNull();
    expect(ipToInt('1.2.3.0x4')).toBeNull();
  });

  it('masks both sides before comparing, and is false on junk', () => {
    expect(sameSubnet('192.168.8.10', '192.168.8.0/24')).toBe(true);
    expect(sameSubnet('192.168.9.10', '192.168.8.0/24')).toBe(false);
    expect(sameSubnet('192.168.8.10', '192.168.8.3/24')).toBe(true);
    expect(sameSubnet('10.1.2.3', '10.0.0.0/8')).toBe(true);
    expect(sameSubnet('1.2.3.4', '255.255.255.255/0')).toBe(true);
    expect(sameSubnet('192.168.8.11', '192.168.8.10/32')).toBe(false);
    expect(sameSubnet('cam.local', '192.168.8.0/24')).toBe(false);
    expect(sameSubnet('192.168.8.10', 'not-a-subnet')).toBe(false);
    expect(sameSubnet('192.168.8.10', '192.168.8.0/33')).toBe(false);
    expect(sameSubnet('', '')).toBe(false);
  });

  it('names a port by what is wrong with it, not by a missing camera', () => {
    expect(portLabel(SWEPT)).toBe('checked 253 addresses on 192.168.8.0/24 — nothing answered');
    expect(portLabel(port({ state: 'no-carrier' }))).toBe("nothing plugged in — check the cable and the camera's power");
    expect(portLabel(port({ state: 'no-carrier', type: 'wi-fi' }))).toBe('not connected to any network');
    expect(portLabel(port({ state: 'failed', reason: 'address-unusable', triedAddress: '192.168.8.4/24' }))).toMatch(/^192\.168\.8\.4\/24 could not be used/);
    expect(
      portLabel(
        port({
          state: 'searched',
          subnet: '192.168.1.0/24',
          neighbors: [{ addr: '192.168.144.25', suggest: '192.168.144.2/24' }],
        }),
      ),
    ).toBe('a device answered at 192.168.144.25 (not confirmed as a camera), not on 192.168.1.0/24 — set this port to 192.168.144.2/24 to reach it');
    expect(portDetail({ port: UPLINK, found: false }, SUBNETS, '')).toBe("checked 253 addresses on 192.168.1.0/24 — nothing answered — this is the device's own uplink");
    expect(portDetail({ port: SWEPT, found: false }, SUBNETS, '192.168.8.10')).toContain('nothing answered at 192.168.8.10 on eth0');
    expect(portDetail({ port: SWEPT, found: true }, SUBNETS, '')).toBe('camera found here');
  });

  it('a router on a port already set for the camera asks for the cable, not an address', () => {
    const idle = port({
      subnet: '192.168.144.0/24',
      neighbors: [{ addr: '192.168.1.3', suggest: '' }],
    });
    const outcome = scanOutcome([], [idle, UPLINK], '');
    expect(todoHeading(outcome.todo, SUBNETS)).toBe('Check the cable');
    expect(portDetail(outcome.todo[0] ?? { port: idle, found: false }, SUBNETS, '')).toContain('Plug the camera in here');
    expect(fixersFor(outcome, SUBNETS).eth0?.open).toBe(false);
    expect(checklistHint([idle], SUBNETS)).toContain('looks like a router or switch');
    expect(subnetOptions(idle, [idle, UPLINK], SUBNETS).map((it) => it.cidr)).toEqual(['192.168.144.2/24', '192.168.8.2/24']);
  });

  it('scan results lead with the port that has a next step', () => {
    const blank = port({ state: 'no-address' });
    const outcome = scanOutcome([], [UPLINK, blank], '');
    expect(outcome.todo.map((it) => it.port.iface)).toEqual(['eth0']);
    expect(outcome.rest.map((it) => it.port.iface)).toEqual(['wlan0']);
    expect(todoHeading(outcome.todo, SUBNETS)).toBe('Try this');
    expect(outcome.note).toBe('No camera answered.');
    expect(fixersFor(outcome, SUBNETS).eth0).toEqual({
      open: true,
      input: '',
      suggest: '',
      status: '',
      busy: false,
    });
    expect(scanOutcome([], [], '').note).toBe("No camera port on this device — type the camera's address below.");
  });

  it('a port with nothing left to do stops competing once a new camera answers', () => {
    const outcome = scanOutcome([cam({})], [SWEPT, port({ iface: 'eth1', state: 'no-address' })], '');
    expect(outcome.todo).toEqual([]);
    expect(adoptedCamera(outcome, true, '')?.url).toBe('rtsp://192.168.8.10:554/stream1');
  });

  it('only one fresh camera with a path is adopted, and never over a typed address', () => {
    expect(adoptedCamera(scanOutcome([cam({})], [SWEPT], ''), true, 'rtsp://192.168.144.25:8554/main.264')).toBeNull();
    expect(adoptedCamera(scanOutcome([cam({ pathFound: false })], [SWEPT], ''), false, '')).toBeNull();
    expect(adoptedCamera(scanOutcome([cam({ used: 'cam' })], [SWEPT], ''), false, '')).toBeNull();
    expect(adoptedCamera(scanOutcome([cam({}), cam({ host: '192.168.8.11' })], [SWEPT], ''), false, '')).toBeNull();
    expect(adoptedCamera(scanOutcome([cam({ used: 'cam' }), cam({ host: '192.168.144.25' })], [SWEPT], ''), false, '')?.host).toBe('192.168.144.25');
  });

  it('the subnet chips put what was measured first and never a subnet another port holds', () => {
    const failed = port({
      state: 'failed',
      reason: 'address-unusable',
      triedAddress: '192.168.8.4/24',
      neighbors: [{ addr: '192.168.144.25', suggest: '192.168.144.2/24' }],
    });
    expect(subnetOptions(failed, [failed], SUBNETS)).toEqual([
      { cidr: '192.168.144.2/24', label: 'found 192.168.144.25' },
      { cidr: '192.168.8.4/24', label: 'tried before' },
      { cidr: '192.168.8.2/24', label: 'LTE routers' },
    ]);
    const fixers = fixersFor(scanOutcome([], [failed], ''), SUBNETS);
    expect(fixers.eth0).toMatchObject({ open: true, input: '192.168.144.2/24' });
    expect(fixerGoLabel(failed, fixers.eth0 ?? { open: false, input: '', suggest: '', status: '', busy: false })).toBe('Set up eth0 and find the camera');
  });

  it('a camera make picks the port that can carry its subnet and says when another holds it', () => {
    expect(templatePort([UPLINK, port({ state: 'no-carrier' }), port({ iface: 'eth1', state: 'no-address' })], SIYI)?.iface).toBe('eth1');
    expect(templatePort([UPLINK], SIYI)).toBeNull();
    const ready = port({ subnet: '192.168.144.0/24' });
    expect(templateReadyPort(SIYI, [], [ready])?.iface).toBe('eth0');
    expect(templateReadyPort(SIYI, [cam({ host: '192.168.144.25' })], [ready])).toBeNull();
    expect(templateSub(SIYI, ready)).toBe('The Ethernet port (eth0) is ready for a SIYI camera — nothing answered at 192.168.144.25. Check the cable.');
    expect(templateSub(SIYI, null)).toBe(SIYI.hint);
    expect(templateAction(false, ready)).toBe('Scan again');
    expect(templateAction(true, ready)).toBe('Setting up…');
    expect(templateHolderError(UPLINK, { ...SIYI, label: 'OpenIPC camera', cidr: '192.168.1.2/24' })).toContain('Wi-Fi (wlan0) already uses 192.168.1.x');
  });

  it('a sign-in goes into the URL, encoded, and leaves with the camera', () => {
    expect(camURL('rtsp://192.168.8.14:554/live', 'admin', 'p@ss word')).toBe('rtsp://admin:p%40ss%20word@192.168.8.14:554/live');
    expect(camURL('rtsp://192.168.8.14:554/live', ' admin ', '')).toBe('rtsp://admin@192.168.8.14:554/live');
    expect(camURL('rtsp://192.168.8.14:554/live', '', '')).toBe('rtsp://192.168.8.14:554/live');
  });

  it('Scan leads until an address is filled, then Continue takes over', () => {
    expect(scanLeads('', false)).toBe(true);
    expect(continueLeads('')).toBe(false);
    expect(scanLeads('rtsp://192.168.8.99:554/stream', false)).toBe(false);
    expect(continueLeads('rtsp://192.168.8.99:554/stream')).toBe(true);
    expect(scanLeads('', true)).toBe(false);
  });

  it('a camera on a working port skips the port question', () => {
    expect(commitsNow('rtsp://192.168.8.10:554/stream1', [SWEPT])).toBe(true);
    expect(nextLabel('rtsp://192.168.8.10:554/stream1', [SWEPT], false)).toBe('Set up & verify');
    expect(nextLabel('rtsp://192.168.8.10:554/stream1', [SWEPT], true)).toBe('Add camera');
    expect(nextLabel('rtsp://192.168.50.99:554/s', [SWEPT], true)).toBe('Continue');
    expect(nextLabel('rpiCamera', [], true)).toBe('Add camera');
    expect(
      nextStep({
        rtsp: 'rtsp://192.168.8.10:554/stream1',
        name: null,
        others: [],
        ports: [SWEPT],
        dupAck: '',
      }).kind,
    ).toBe('apply');
    expect(nextStep({ rtsp: '192.168.50.99:554/s', name: null, others: [], ports: [SWEPT], dupAck: '' })).toEqual({ kind: 'port', rtsp: 'rtsp://192.168.50.99:554/s', name: null });
  });

  it('Continue refuses what cannot work and says where it belongs', () => {
    const refusal = (rtsp: string, name: string | null = null, dupAck = '') => {
      const out = nextStep({
        rtsp,
        name,
        others: [{ name: 'cam', source: 'rtsp://192.168.8.10:554/stream' }],
        ports: [],
        dupAck,
      });
      return out.kind === 'refuse' ? out : null;
    };
    expect(refusal('192.168.144.2/24')?.refusal.text).toContain("address for this device's port, not the camera");
    expect(refusal('rtsp://192.168.144.2/24')?.refusal.field).toBe('rtsp');
    expect(refusal('rtsp://192.168.8.11/s', ' ')?.refusal).toEqual({
      text: 'Give the camera a name.',
      warn: false,
      field: 'name',
    });
    expect(refusal('rtsp://192.168.8.11/s', 'CAM')?.refusal.text).toBe('A camera named “cam” already exists — pick another name.');
    expect(refusal('ftp://x')?.refusal.text).toContain("Enter the camera's address");
    const dup = refusal('rtsp://192.168.8.10:554/stream');
    expect(dup?.refusal.warn).toBe(true);
    expect(dup?.dupAck).toBe('rtsp://192.168.8.10:554/stream');
    expect(refusal('rtsp://192.168.8.10:554/stream', null, 'rtsp://192.168.8.10:554/stream')).toBeNull();
    expect(nextStep({ rtsp: 'rpiCamera', name: 'picam', others: [], ports: [], dupAck: '' })).toEqual({ kind: 'apply', rtsp: 'rpiCamera', name: 'picam' });
  });

  it('the apply note says whether the port changes', () => {
    expect(applyNote('eth0', '192.168.8.2/24', '192.168.8.2/24')).toBe('eth0 keeps 192.168.8.2/24 — nothing on that port changes.');
    expect(applyNote('eth0', '192.168.50.2/24', '')).toBe('Sets eth0 to 192.168.50.2/24 — anything else on that port may need reconnecting.');
    expect(applyNote('', '192.168.50.2/24', '')).toBe('');
  });
});
