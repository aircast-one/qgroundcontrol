import { describe, expect, it } from 'vitest';
import { VIDEO_FILL, VIDEO_FIT_HEIGHT, VIDEO_FIT_WIDTH, VIDEO_NO_CROP, pinchStep, videoAspect, videoContentSize } from '@/ui/VideoSurface';

describe('PinchZoomTest', () => {
  it("a pinch maps to FlightDisplayViewVideo's step values", () => {
    expect(pinchStep(2.2)).toBe(2);
    expect(pinchStep(1.1)).toBe(1);
    expect(pinchStep(0.5)).toBe(-5);
  });
});

describe('VideoFitTest', () => {
  const wide = 16.0 / 9.0;
  const expectSize = (actual: readonly [number, number], expected: readonly [number, number], message = '') => {
    expect(actual[0], message).toBeCloseTo(expected[0], 2);
    expect(actual[1], message).toBeCloseTo(expected[1], 2);
  };

  it('each fit mode sizes the picture like FlightDisplayViewVideo', () => {
    expectSize(videoContentSize(1000, 1000, wide, VIDEO_FIT_WIDTH), [1000, 562.5]);
    expectSize(videoContentSize(1000, 1000, wide, VIDEO_FIT_HEIGHT), [1777.7778, 1000]);
    expectSize(videoContentSize(1000, 1000, wide, VIDEO_FILL), [1777.7778, 1000], 'a square box is filled by cropping the sides');
    expectSize(videoContentSize(1000, 1000, wide, VIDEO_NO_CROP), [1000, 562.5], 'no crop letterboxes');
    expectSize(videoContentSize(1000, 400, 0.0, VIDEO_NO_CROP), [1000, 400], 'no aspect known fills the box');
  });

  it("the stream's own size wins over the aspect ratio setting", () => {
    expect(videoAspect({ width: 640, height: 480 }, wide)).toBeCloseTo(4.0 / 3.0, 9);
    expect(videoAspect(null, wide)).toBeCloseTo(wide, 9);
    expect(videoAspect({ width: 0, height: 0 }, null)).toBeCloseTo(0.0, 9);
  });
});
