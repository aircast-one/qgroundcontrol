import { describe, expect, it } from 'vitest';
import { mapScaleBar, metresAcross, metresPerPixel } from './MapScale';

describe('MapScaleTest', () => {
  it('metres across the bar follow latitude and zoom', () => {
    const equator = metresAcross(0.0, 16.0, 300.0) ?? NaN;
    const south = metresAcross(-60.0, 16.0, 300.0) ?? NaN;
    expect(south < equator).toBe(true);
    expect(equator).toBeCloseTo(metresPerPixel(0.0, 16.0) * 300.0, 9);
  });

  it('a bar that cannot be measured is not drawn', () => {
    expect(metresAcross(0.0, 16.0, 0.0)).toBeNull();
    expect(metresAcross(NaN, 16.0, 200.0)).toBeNull();
  });

  it("the bar takes the core's text and its share of the width", () => {
    const bar = mapScaleBar({ available: true, text: '100 m', fraction: 0.5 }, 300.0);
    expect(bar?.text).toBe('100 m');
    expect(bar?.pixels).toBeCloseTo(150.0, 9);
  });

  it('imperial text is passed through, because the core chose the units', () => {
    expect(mapScaleBar({ available: true, text: '500 ft', fraction: 0.4 }, 200.0)?.text).toBe('500 ft');
  });

  it('a scale the core could not work out draws nothing', () => {
    expect(mapScaleBar({ available: false }, 300.0)).toBeNull();
    expect(mapScaleBar({ available: true, text: '', fraction: 0.5 }, 300.0)).toBeNull();
    expect(mapScaleBar({ available: true, text: '100 m' }, 300.0)).toBeNull();
    expect(mapScaleBar(null, 300.0)).toBeNull();
  });
});
