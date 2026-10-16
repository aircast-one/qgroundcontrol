import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { hatBits, powerStateText, scaled, streams } from './GamepadInput';

describe('GamepadStreamsTest', () => {
  it('theGamepadStreamsOnlyWhileItDrivesAVehicleOrIsBeingCalibrated', () => {
    expect(streams(parseJson('{"vehicle":true,"enabled":true,"calibration":null}'))).toBe(true);
    expect(streams(parseJson('{"vehicle":true,"enabled":false,"calibration":null}'))).toBe(false);
    expect(streams(parseJson('{"vehicle":false,"enabled":true,"calibration":null}'))).toBe(false);
    expect(streams(parseJson('{"vehicle":false,"enabled":false,"calibration":{"step":1}}'))).toBe(true);
    expect(streams(null)).toBe(false);
  });
});

describe('GamepadInput', () => {
  it('gamepad_values_scale_to_the_core_range_and_the_dpad_becomes_hat_bits', () => {
    expect(scaled(1)).toBe(32767);
    expect(scaled(-2)).toBe(-32767);
    expect(hatBits(-1, -1)).toBe(0x01 | 0x08);
    expect(hatBits(1, 1)).toBe(0x04 | 0x02);
    expect(hatBits(0.2, -0.2)).toBe(0);
  });

  it('an unknown power state reads as nothing', () => {
    expect(powerStateText(-1)).toBe('');
    expect(powerStateText(2)).toBe('Charging');
  });
});
