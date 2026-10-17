import { describe, expect, it } from 'vitest';
import { argbPixels } from './OpticalFlowScreen';

describe('OpticalFlowScreenTest', () => {
  it('core rgba bytes become argb pixels', () => {
    const rgba = Uint8Array.from([0x11, 0x22, 0x33, 0xff, 0x80, 0, 0x40, 0x7f]);
    expect(Array.from(argbPixels(rgba, 2))).toEqual([0xff112233 | 0, 0x7f800040]);
    expect(Array.from(argbPixels(new Uint8Array(), 1)), 'missing bytes read as transparent black').toEqual([0]);
  });
});
