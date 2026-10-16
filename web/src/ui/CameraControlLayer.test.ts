import { describe, expect, it } from 'vitest';
import { BesideAnchor } from './CameraControlLayer';

describe('CameraControlLayerTest', () => {
  const window = { width: 1000, height: 500 };
  const popup = { width: 100, height: 200 };
  const placed = (anchor: [number, number, number, number], direction: 'Ltr' | 'Rtl' = 'Ltr') =>
    BesideAnchor.calculatePosition({ left: anchor[0], top: anchor[1], right: anchor[2], bottom: anchor[3] }, window, direction, popup);

  it('the zoom slider opens beside the chip, centred on it', () => {
    expect(placed([600, 230, 680, 270])).toEqual({ x: 500, y: 150 });
  });

  it('the zoom slider stays on screen next to an edge', () => {
    expect(placed([40, 230, 120, 270])).toEqual({ x: 0, y: 150 });
    expect(placed([600, 10, 680, 50])).toEqual({ x: 500, y: 0 });
    expect(placed([600, 460, 680, 500])).toEqual({ x: 500, y: 300 });
  });

  it("right to left the zoom slider opens on the chip's other side", () => {
    expect(placed([600, 230, 680, 270], 'Rtl')).toEqual({ x: 680, y: 150 });
  });
});
