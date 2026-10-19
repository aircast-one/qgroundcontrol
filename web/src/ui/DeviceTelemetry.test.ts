import { describe, expect, it } from 'vitest';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { endpointLabel } from '@/ui/DeviceTelemetryScreen';
import { parseEndpointEditor, parseTelemetry } from '@/ui/DeviceTelemetry';

const UDPS = { value: 'udps', label: 'UDP · listen', host: '0.0.0.0', port: '14550', hostLabel: 'Listen address', placeholder: '0.0.0.0', hint: 'The device listens here.' };

describe('DeviceTelemetryTest', () => {
  it('reads the telemetry view the device serves', () => {
    const view = parseTelemetry({
      status: { level: 0, text: 'ready' },
      notices: [{ level: 3, text: 'No flight controller is attached.' }],
      endpoints: [{ proto: 'UDP', label: 'MAVLink', url: 'falcon:14550', down: true }],
      cloud: { status: { level: 0, text: 'Carrying' }, text: 'Also carrying telemetry.' },
      settings: { endpoints: 'udps:0.0.0.0:14550', baud: '57600' },
      schemes: [UDPS],
      gimbal: { title: 'SIYI gimbal camera', status: { level: 0, text: 'connected' }, vendor: 'siyi', vendors: [{ value: 'siyi', label: 'SIYI' }], models: [] },
    });
    expect(view?.status).toEqual({ level: DeviceLevel.Ready, text: 'ready' });
    expect(view?.endpoints[0]).toMatchObject({ url: 'falcon:14550', down: true });
    expect(view?.endpointNotice).toBeNull();
    expect(view?.cloud?.status.text).toBe('Carrying');
    expect(view?.settings).toEqual({ endpoints: 'udps:0.0.0.0:14550', baud: '57600' });
    expect(view?.schemes).toEqual([UDPS]);
    expect(view?.gimbal?.vendors).toEqual([{ value: 'siyi', label: 'SIYI' }]);
    expect(parseTelemetry({ endpoints: [] })).toBeNull();
    const endpoint = { proto: 'UDP', label: 'MAVLink', url: 'falcon:14550', hint: '', note: '', appLink: '', down: false, primary: false };
    expect(endpointLabel(endpoint)).toBe('UDP · MAVLink');
    expect(endpointLabel({ ...endpoint, proto: 'TCP', label: 'TCP · Betaflight' })).toBe('TCP · Betaflight');
  });

  it('reads the endpoint rows the core makes of the stored spec or the typed rows', () => {
    const row = { scheme: 'udpc', host: '', port: '14550' };
    const shown = { ...row, hostLabel: 'Ground station address', placeholder: '192.168.1.20', note: 'Pushes telemetry.', problem: 'Needs an address. It is not saved until it has one.' };
    const options = [{ value: 'udps', label: 'UDP · listen', row: { scheme: 'udps', host: '0.0.0.0', port: '14550' } }];
    const blank = { scheme: 'udps', host: '0.0.0.0', port: '14550', hostLabel: 'Listen address', placeholder: '0.0.0.0', note: 'Listens.', problem: '', options: [] };
    const editor = parseEndpointEditor({ ok: true, result: { rows: [{ ...shown, options }], blank, endpoints: 'udps:0.0.0.0:14550' } });
    expect(editor?.rows[0]).toEqual({ row, hostLabel: shown.hostLabel, placeholder: shown.placeholder, note: shown.note, problem: shown.problem, options });
    expect(editor?.blank.row).toEqual({ scheme: 'udps', host: '0.0.0.0', port: '14550' });
    expect(editor?.endpoints).toBe('udps:0.0.0.0:14550');
    expect(parseEndpointEditor({ ok: false, reason: 'no' })).toBeNull();
    expect(parseEndpointEditor(null)).toBeNull();
  });
});
