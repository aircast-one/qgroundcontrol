import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { LinkCounts, espBridge, hostPortTyped, stationFieldsEnabled } from './EspBridgeScreen';

describe('EspBridgeScreenTest', () => {
  it('the_bridge_reads_settings_and_grouped_counters', () => {
    expect(espBridge(parseJson('{"available":false}'))).toBeNull();
    const bridge = espBridge(
      parseJson(`{"available":true,"modeIndex":null,"channel":6,"ssid":"PixRacer","password":"pixracer","ssidSta":null,"passwordSta":null,
        "baudRates":[57600,921600],"baudIndex":1,"hostPort":{"valueString":"14550"},
        "status":{"vehicle":{"received":1234567,"lost":0,"sent":null},"bridge":{},"qgc":{}}}`),
    );
    if (bridge === null) throw new Error('parsed');
    expect(bridge.modeIndex).toBeNull();
    expect(bridge.vehicle).toEqual(LinkCounts('1,234,567', '0', ''));
    expect(bridge.hostPort).toBe('14550');
    expect(stationFieldsEnabled(bridge)).toBe(false);
    expect(stationFieldsEnabled({ ...bridge, modeIndex: 1 })).toBe(true);
  });

  it('restart_waits_for_the_bridge_like_controller_busy', () => {
    expect(espBridge(parseJson('{"available":true}'))?.busy).toBe(false);
    expect(espBridge(parseJson('{"available":true,"busy":true}'))?.busy).toBe(true);
  });

  it('the_qgc_udp_port_is_typed_within_the_validator_range', () => {
    const bridge = espBridge(parseJson('{"available":true,"hostPort":{"valueString":"14550","path":"vehicle.parameterManager.getParameter(240,WIFI_UDP_HPORT)"}}'));
    expect(bridge?.hostPortPath).toBe('vehicle.parameterManager.getParameter(240,WIFI_UDP_HPORT)');
    expect(['1023', '1024', '65535', '65536'].map(hostPortTyped)).toEqual([null, 1024, 65535, null]);
  });
});
