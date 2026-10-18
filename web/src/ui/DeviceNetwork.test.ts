import { describe, expect, it } from 'vitest';
import { shownFields } from '@/ui/DeviceKitForms';
import { GOOGLE_STUN, connectionFieldShown, iceRows, manualAddressing, parseNetwork, serializeIceServers, uplinkIcon } from '@/ui/DeviceNetwork';
import { DeviceLevel } from '@/ui/DeviceStatus';
import { NO_ANSWER } from '@/ui/DeviceView';
import { joinOutcome, parseWifiScan, reopenLabel, reopenLink, wifiJoinProblem } from '@/ui/DeviceWifiWizard';

const VIEW = {
  uplink: {
    icon: 'wifi',
    title: 'Wi-Fi (wlan0)',
    status: { level: 0, text: 'carrying traffic' },
    verdict: { level: 3, text: 'No video going to the cloud' },
    addr: '192.168.1.20/24',
    failover: [{ label: 'LTE (wwan0)', status: { level: 3, text: 'standing by · takes over in ~15 s' } }],
    switch: 'Moved just now: from Wi-Fi (wlan0) to LTE (wwan0)',
  },
  modem: {
    status: { level: 0, text: 'connected' },
    summary: 'HARNESS',
    notices: [],
    operator: 'HARNESS',
    detail: 'lte · home · signal 72%',
    signal: { label: 'Signal', values: [80, -1, 72], max: 100, value: '72%', level: 3 },
    facts: [{ label: 'APN (in use)', value: 'internet' }],
    messages: { summary: '1 on the SIM · latest from MAGTICOM', items: [{ from: 'MAGTICOM', text: 'code 2157', when: 'Sep 25 17:41' }] },
    sim: { summary: 'APN internet', apn: 'internet', placeholder: 'auto', metered: 'auto', options: [{ value: 'auto', label: 'Automatic' }] },
  },
  starlink: { status: { level: 3, text: 'no dish' }, summary: '', notices: [], facts: [], address: { value: '10.0.0.9:9200', default: '192.168.100.1:9200', defaultFirst: true, open: true } },
  wifi: { status: { level: 3, text: 'not joined' }, summary: 'join a network', reopen: { host: 'harness.local', hostname: 'harness' } },
  interfaces: {
    status: { level: 3, text: '' },
    summary: '1 in use · 1 known',
    links: [
      {
        name: 'camera (eth0)',
        kind: 'ethernet',
        where: 'eth0 · 192.168.8.2/24',
        local: true,
        editor: { interface: 'eth0', ipv4: { method: 'manual', metric: '', metricPlaceholder: 'automatic', address: '192.168.8.2/24', gateway: '', dns: '' }, uuid: 'uuid-eth0', deleteConfirm: 'Delete?' },
      },
    ],
    saved: [{ uuid: 'uuid-spare', name: 'spare-net', kind: 'ethernet', ipv4: { method: 'auto' }, activateConfirm: 'Activate spare-net?', deleteConfirm: 'Delete spare-net?' }],
    other: [],
    methods: [{ value: 'auto', label: 'DHCP (auto)' }],
    add: { types: [{ value: 'ethernet', label: 'Ethernet' }], ports: [], autoconnect: [] },
  },
  data: { status: { level: 3, text: 'no caps' }, summary: '0 B this month', budget: { summary: 'no caps set', monthlyCapGB: '', flightCapMB: '', warnPercent: '0' } },
  tailscale: {
    status: { level: 1, text: 'signing in' },
    summary: 'finish the sign-in',
    banner: { title: 'Waiting for sign-in', subtitle: 'Step 1 of 2', status: { level: 1, text: 'pending' } },
    facts: [],
    addresses: [],
    signIn: { label: 'Open Tailscale login', href: 'https://login.tailscale.com/a/x' },
    cancel: true,
  },
  relay: { status: { level: 3, text: 'off' }, summary: '', on: false, configured: false, reason: 'Add a relay server first', notice: { level: 3, text: 'Video keeps working.' }, servers: [] },
};

describe('DeviceNetworkTest', () => {
  it('reads the network view the device serves', () => {
    const view = parseNetwork(VIEW);
    expect(view?.uplink.failover[0]).toEqual({ label: 'LTE (wwan0)', status: { level: DeviceLevel.Neutral, text: 'standing by · takes over in ~15 s' } });
    expect(view?.uplink.switchText).toContain('from Wi-Fi (wlan0) to LTE (wwan0)');
    expect(view?.modem?.signal?.values).toEqual([80, -1, 72]);
    expect(view?.modem?.messages.items[0]?.from).toBe('MAGTICOM');
    expect(view?.modem?.sim?.apn).toBe('internet');
    expect(view?.starlink.address).toEqual({ value: '10.0.0.9:9200', fallback: '192.168.100.1:9200', defaultFirst: true, open: true });
    expect(view?.starlink.demo).toBeNull();
    expect(view?.wifi.joined).toBeNull();
    expect(view?.wifi.reopen).toEqual({ host: 'harness.local', hostname: 'harness' });
    expect(view?.interfaces.links[0]?.editor?.iface).toBe('eth0');
    expect(view?.interfaces.links[0]?.role).toBeNull();
    expect(view?.interfaces.saved[0]?.ipv4.method).toBe('auto');
    expect(view?.data.gauge).toBeNull();
    expect(view?.tailscale.signIn?.href).toBe('https://login.tailscale.com/a/x');
    expect(view?.tailscale.control).toBeNull();
    expect(view?.relay.reason).toBe('Add a relay server first');
    expect(parseNetwork({ ...VIEW, modem: undefined })?.modem).toBeNull();
    expect(parseNetwork({ modem: VIEW.modem })).toBeNull();
  });

  it('each kind of uplink gets an icon of its own, anything else the network one', () => {
    expect(uplinkIcon('satellite-dish')).toBe('satellite_alt');
    expect(uplinkIcon('signal')).toBe('signal_cellular_alt');
    expect(uplinkIcon('ethernet-port')).toBe('settings_ethernet');
    expect(uplinkIcon('network')).toBe('lan');
    expect(uplinkIcon('')).toBe('lan');
  });

  it('relay servers serialize to the lines the device parses, credentials only where given', () => {
    expect(
      serializeIceServers([
        { url: ' turn:turn.example.com:3478 ', username: 'drone', password: '' },
        { url: GOOGLE_STUN, username: '', password: '' },
        { url: '  ', username: 'orphan', password: 'x' },
      ]),
    ).toBe(`turn:turn.example.com:3478 | drone | \n${GOOGLE_STUN}`);
    expect(serializeIceServers([])).toBe('');
  });

  it('an empty server list still offers one row to fill in', () => {
    expect(iceRows([])).toEqual([{ url: '', username: '', password: '' }]);
    const saved = [{ url: GOOGLE_STUN, username: '', password: '' }];
    expect(iceRows(saved)).toBe(saved);
  });

  it('a new connection shows only the fields its type needs, and manual addressing only when chosen', () => {
    expect(connectionFieldShown('wifi', 'ssid')).toBe(true);
    expect(connectionFieldShown('wifi', 'apn')).toBe(false);
    expect(connectionFieldShown('gsm', 'pin')).toBe(true);
    expect(connectionFieldShown('ethernet', 'psk')).toBe(false);
    expect(connectionFieldShown('ethernet', 'name')).toBe(true);
    expect(manualAddressing('manual')).toBe(true);
    expect(manualAddressing('auto')).toBe(false);
    const fields = [
      { name: 'method', label: 'IPv4 method', value: 'auto' },
      { name: 'address', label: 'Address', value: '', shownWhen: (values: Readonly<Record<string, string>>) => manualAddressing(values.method ?? '') },
    ];
    expect(shownFields(fields, { method: 'auto' }).map((field) => field.name)).toEqual(['method']);
    expect(shownFields(fields, { method: 'manual' }).map((field) => field.name)).toEqual(['method', 'address']);
  });

  it('a Wi-Fi join is checked before the hotspot is given up', () => {
    expect(wifiJoinProblem('  ', '', '')).toEqual({ message: 'Enter your network name first.', field: 'ssid' });
    expect(wifiJoinProblem('workshop', '', 'workshop')?.message).toContain('secured network');
    expect(wifiJoinProblem('typed-by-hand', '', '')).toBeNull();
    expect(wifiJoinProblem('workshop', 'short', '')?.message).toContain('8–63 characters');
    expect(wifiJoinProblem('workshop', 'x'.repeat(64), '')?.field).toBe('psk');
    expect(wifiJoinProblem('workshop', 'supersecret', 'workshop')).toBeNull();
  });

  it('a join that lost the link says how to find the device, a refused one hides those steps', () => {
    const join = { attempt: 1, target: 'workshop', status: 'Switching…', instructions: true };
    expect(joinOutcome(join, null).status).toContain('Connected to “workshop”');
    expect(joinOutcome(join, NO_ANSWER)).toMatchObject({ instructions: true, status: expect.stringContaining('Lost contact') });
    expect(joinOutcome(join, { message: 'SSID is too long', field: 'ssid' })).toMatchObject({ instructions: false, status: '✗ SSID is too long' });
    expect(reopenLabel('http://harness.local/fly/#device=%2Fnetwork')).toBe('harness.local/fly/#device=%2Fnetwork');
    const here = { pathname: '/fly/', hash: '#device=%2Fnetwork' };
    expect(reopenLink('harness.local', '', here)).toBe('http://harness.local/fly/#device=%2Fnetwork');
    expect(reopenLink('harness.local', 'https://racer.ts.net', here)).toBe('http://harness.local');
    expect(reopenLink('', '', here)).toBe('');
    expect(reopenLabel('')).toBe("the device's address on your network");
  });

  it('reads the networks a scan found', () => {
    expect(parseWifiScan({ networks: [{ ssid: 'workshop', signal: 82, secured: true }] })).toEqual([{ ssid: 'workshop', signal: 82, secured: true, active: false }]);
    expect(parseWifiScan(null)).toBeNull();
  });
});
