import { describe, expect, it } from 'vitest';
import { DeviceLevel } from '@/ui/DeviceStatus';
import { endpointLabel } from '@/ui/DeviceTelemetryScreen';
import {
  type MavlinkScheme,
  UNKNOWN_SCHEME_HINT,
  describeEndpoint,
  endpointError,
  newRow,
  parseEndpointSpecs,
  parseTelemetry,
  retargetRow,
  schemeOptions,
  serializeEndpoints,
} from '@/ui/DeviceTelemetry';

const scheme = (value: string, host: string, port: string): MavlinkScheme => ({ value, label: value, host, port, hostLabel: 'Address', placeholder: '', hint: `${value} hint` });

const SCHEMES: readonly MavlinkScheme[] = [scheme('udps', '0.0.0.0', '14550'), scheme('udpc', '', '14550'), scheme('udpb', '255.255.255.255', '14550'), scheme('tcps', '0.0.0.0', '5760'), scheme('tcpc', '', '5760')];

describe('DeviceTelemetryTest', () => {
  it('reads the telemetry view the device serves', () => {
    const view = parseTelemetry({
      status: { level: 0, text: 'ready' },
      notices: [{ level: 3, text: 'No flight controller is attached.' }],
      endpoints: [{ proto: 'UDP', label: 'MAVLink', url: 'falcon:14550', down: true }],
      cloud: { status: { level: 0, text: 'Carrying' }, text: 'Also carrying telemetry.' },
      settings: { endpoints: 'udps:0.0.0.0:14550', baud: '57600' },
      schemes: SCHEMES,
      gimbal: { title: 'SIYI gimbal camera', status: { level: 0, text: 'connected' }, vendor: 'siyi', vendors: [{ value: 'siyi', label: 'SIYI' }], models: [] },
    });
    expect(view?.status).toEqual({ level: DeviceLevel.Ready, text: 'ready' });
    expect(view?.endpoints[0]).toMatchObject({ url: 'falcon:14550', down: true });
    expect(view?.endpointNotice).toBeNull();
    expect(view?.cloud?.status.text).toBe('Carrying');
    expect(view?.settings.baud).toBe('57600');
    expect(view?.gimbal?.vendors).toEqual([{ value: 'siyi', label: 'SIYI' }]);
    expect(parseTelemetry({ endpoints: [] })).toBeNull();
    const endpoint = { proto: 'UDP', label: 'MAVLink', url: 'falcon:14550', hint: '', note: '', appLink: '', down: false, primary: false };
    expect(endpointLabel(endpoint)).toBe('UDP · MAVLink');
    expect(endpointLabel({ ...endpoint, proto: 'TCP', label: 'TCP · Betaflight' })).toBe('TCP · Betaflight');
  });

  it('stored endpoints come back as rows a form can edit', () => {
    expect(parseEndpointSpecs('udps:0.0.0.0:14550\n\n tcpc:gcs.local:5760 ')).toEqual([
      { scheme: 'udps', host: '0.0.0.0', port: '14550' },
      { scheme: 'tcpc', host: 'gcs.local', port: '5760' },
    ]);
  });

  it('an IPv6 address keeps its brackets and loses only the port', () => {
    expect(parseEndpointSpecs('udps:[::]:14550')).toEqual([{ scheme: 'udps', host: '[::]', port: '14550' }]);
  });

  it('rows serialize back to the specs the router parses', () => {
    expect(
      serializeEndpoints(SCHEMES, [
        { scheme: 'udps', host: '0.0.0.0', port: '14550' },
        { scheme: 'udpc', host: ' 10.0.0.5 ', port: ' 14551 ' },
      ]),
    ).toBe('udps:0.0.0.0:14550\nudpc:10.0.0.5:14551');
  });

  it('a row with no port is dropped, and an empty host falls back to the default', () => {
    expect(
      serializeEndpoints(SCHEMES, [
        { scheme: 'udpc', host: '10.0.0.5', port: '' },
        { scheme: 'udps', host: '', port: '14550' },
      ]),
    ).toBe('udps:0.0.0.0:14550');
  });

  it('a new row is ready to save as it stands', () => {
    expect(serializeEndpoints(SCHEMES, [newRow(SCHEMES)])).toBe('udps:0.0.0.0:14550');
    expect(serializeEndpoints(SCHEMES, [newRow(SCHEMES, 'tcps')])).toBe('tcps:0.0.0.0:5760');
  });

  it('retargeting keeps a typed address and replaces an untouched default', () => {
    expect(retargetRow(SCHEMES, { scheme: 'tcpc', host: '10.0.0.5', port: '14550' })).toEqual({ scheme: 'tcpc', host: '10.0.0.5', port: '14550' });
    expect(retargetRow(SCHEMES, { scheme: 'udpb', host: '0.0.0.0', port: '14550' })).toEqual({ scheme: 'udpb', host: '255.255.255.255', port: '14550' });
    expect(retargetRow(SCHEMES, { scheme: 'tcps', host: '', port: '' })).toEqual({ scheme: 'tcps', host: '0.0.0.0', port: '5760' });
  });

  it('a half-written address is named in the row, not sent as a broken spec', () => {
    expect(endpointError(SCHEMES, { scheme: 'udpc', host: '', port: '14550' })).toBe('Needs an address.');
    expect(endpointError(SCHEMES, { scheme: 'udpc', host: '10.0.0.5', port: '' })).toBe('Needs a port between 1 and 65535.');
    expect(endpointError(SCHEMES, { scheme: 'udps', host: '', port: '14550' })).toBe('');
    expect(endpointError(SCHEMES, { scheme: 'udpc', host: '', port: '' })).toBe('');
    expect(
      serializeEndpoints(SCHEMES, [
        { scheme: 'udpc', host: '', port: '14550' },
        { scheme: 'udps', host: '', port: '14550' },
      ]),
    ).toBe('udps:0.0.0.0:14550');
  });

  it('a port that is not a port is named in the row', () => {
    expect(endpointError(SCHEMES, { scheme: 'udps', host: '0.0.0.0', port: 'ssh' })).toBe('Needs a port between 1 and 65535.');
    expect(endpointError(SCHEMES, { scheme: 'udps', host: '0.0.0.0', port: '70000' })).toBe('Needs a port between 1 and 65535.');
    expect(endpointError(SCHEMES, { scheme: 'udps', host: '0.0.0.0', port: '14550' })).toBe('');
    expect(serializeEndpoints(SCHEMES, [{ scheme: 'udps', host: '', port: 'ssh' }])).toBe('');
  });

  it('a scheme the device does not offer is shown as itself and saved untouched', () => {
    const row = { scheme: 'serial', host: '/dev/ttyUSB0', port: '57600' };
    expect(schemeOptions(SCHEMES, row).at(-1)).toEqual({ value: 'serial', label: 'serial' });
    expect(schemeOptions(SCHEMES, { scheme: 'udps', host: '', port: '' })).toHaveLength(5);
    expect(serializeEndpoints(SCHEMES, [row])).toBe('serial:/dev/ttyUSB0:57600');
    expect(describeEndpoint(SCHEMES, row)).toBe(UNKNOWN_SCHEME_HINT);
    expect(endpointError(SCHEMES, { scheme: 'serial', host: '', port: '57600' })).toBe('Needs an address.');
  });
});
