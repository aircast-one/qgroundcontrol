import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { usbRadioBadge, usbRadioTone } from './UsbRadioIndicator';

describe('UsbRadioIndicator', () => {
  it("reads the core's radio state and hides while Radio on USB is off", () => {
    expect(usbRadioBadge(parseJson('{"indicator":{"text":"Radio · live","state":"live"}}'))).toEqual({ text: 'Radio · live', state: 'live' });
    expect(usbRadioBadge(parseJson('{"indicator":null}'))).toBeNull();
    expect(usbRadioBadge(parseJson('{"indicator":{"text":"?","state":"exploded"}}'))).toBeNull();
  });

  it('a stopped radio reads as an error and a paused one as a warning', () => {
    expect(usbRadioTone('stopped')).toBe('error');
    expect(usbRadioTone('paused')).toBe('warning');
    expect(usbRadioTone('live')).toBeNull();
    expect(usbRadioTone('standby')).toBeNull();
  });
});
