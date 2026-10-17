import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { firmwareFence } from './FenceBridge';
import { FIRMWARE_FENCE_COLOUR, KEEP_IN_COLOUR, KEEP_OUT_COLOUR } from './MissionLayers';

const view = (body: string) => parseJson(`{"firmwareFence":${body}}`);

describe('FirmwareFenceTest', () => {
  it('a served fence carries its radius, its text and its centre', () => {
    const fence = firmwareFence(view('{"radiusMetres":300.0,"radiusText":"300 m","centre":{"latitude":41.7,"longitude":44.8}}'));
    expect(fence).not.toBeNull();
    expect(fence?.radiusMetres).toBeCloseTo(300.0, 3);
    expect(fence?.radiusText).toBe('300 m');
    expect(fence?.centre?.latitude).toBeCloseTo(41.7, 3);
  });

  it('no fence at all is not a fence of no size', () => {
    expect(firmwareFence(parseJson('{}'))).toBeNull();
    expect(firmwareFence(null)).toBeNull();
  });

  it('a radius of zero is absent parameters, not a circle round home', () => {
    expect(firmwareFence(view('{"radiusMetres":0.0,"radiusText":"0 m"}'))).toBeNull();
  });

  it('a fence with no home to sit on still reports its radius', () => {
    const fence = firmwareFence(view('{"radiusMetres":300.0,"radiusText":"300 m","centre":null}'));
    expect(fence).not.toBeNull();
    expect(fence?.centre).toBeNull();
    expect(fence?.radiusText).toBe('300 m');
  });

  it('the fence the vehicle imposes cannot be drawn like one the plan can edit', () => {
    expect(FIRMWARE_FENCE_COLOUR).not.toBe(KEEP_IN_COLOUR);
    expect(FIRMWARE_FENCE_COLOUR).not.toBe(KEEP_OUT_COLOUR);
  });
});
