import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { PipSurfaces } from '@/ui/VideoSurface';
import { paintedRect, videoReading } from '@/ui/VideoView';

const view = (active: number, ...statuses: string[]): Json => ({
  kind: 'object',
  class: 'Video',
  available: true,
  decoding: false,
  summary: 'Not streaming.',
  activeSource: active,
  cameras: statuses.map((status, slot) => ({
    slot,
    title: `Camera ${slot + 1}`,
    status,
    connecting: false,
    recording: false,
    configured: status.trim() !== '' && status !== 'No stream URL',
  })),
});

describe('VideoViewTest', () => {
  it("the reading carries the core's summary and cameras", () => {
    const reading = videoReading(view(1, 'Streaming', 'No stream URL'))!;

    expect(reading.summary).toBe('Not streaming.');
    expect(reading.activeSource).toBe(1);
    expect(reading.cameras.map((it) => it.status)).toEqual(['Streaming', 'No stream URL']);
    expect(reading.cameras.map((it) => it.configured)).toEqual([true, false]);
  });

  it('a stream the core switched off reads as off, and one it does not mention as on', () => {
    expect(videoReading({ ...view(0, 'Streaming'), streamEnabled: false })!.streamEnabled).toBe(false);
    expect(videoReading(view(0, 'Streaming'))!.streamEnabled).toBe(true);
  });

  it('the source size is read only when the core reports a usable one', () => {
    const decoding = { ...view(0, 'Streaming'), sourceSize: { width: 640, height: 480 } };
    const zero = { ...view(0, 'Streaming'), sourceSize: { width: 0, height: 0 } };

    expect(videoReading(decoding)!.sourceSize).toEqual({ width: 640, height: 480 });
    expect(videoReading(zero)!.sourceSize).toBeNull();
    expect(videoReading(view(0, 'Streaming'))!.sourceSize).toBeNull();
  });

  it('the picture is letterboxed inside the surface it is painted on', () => {
    const wide = paintedRect(1080.0, 1770.0, { width: 640, height: 480 });
    expect(wide.width).toBeCloseTo(1080.0, 3);
    expect(wide.height).toBeCloseTo(810.0, 3);
    expect(wide.left).toBeCloseTo(0.0, 3);
    expect(wide.top).toBeCloseTo(480.0, 3);

    const tall = paintedRect(400.0, 200.0, { width: 480, height: 640 });
    expect(tall.width).toBeCloseTo(150.0, 3);
    expect(tall.height).toBeCloseTo(200.0, 3);
    expect(tall.left).toBeCloseTo(125.0, 3);
  });

  it('an unknown source size paints the whole surface', () => {
    const whole = paintedRect(300.0, 200.0, null);

    expect(whole.left).toBeCloseTo(0.0, 3);
    expect(whole.width).toBeCloseTo(300.0, 3);
    expect(whole.height).toBeCloseTo(200.0, 3);
  });

  it('the picture-in-picture plays only while a thumbnail surface is on screen', () => {
    const sent = { value: [] as readonly boolean[] };
    const surfaces = new PipSurfaces((shown) => {
      sent.value = [...sent.value, shown];
    });
    surfaces.created();
    surfaces.destroyed();
    expect(sent.value).toEqual([true, false]);
  });

  it('a thumbnail that reappears before the old one is torn down keeps the picture-in-picture playing', () => {
    const sent = { value: [] as readonly boolean[] };
    const surfaces = new PipSurfaces((shown) => {
      sent.value = [...sent.value, shown];
    });
    surfaces.created();
    surfaces.created();
    surfaces.destroyed();
    expect(sent.value).toEqual([true, true, true]);
    surfaces.destroyed();
    expect(sent.value.at(-1)).toBe(false);
  });

  it('a reply that is not the video view reads as nothing', () => {
    expect(videoReading(null)).toBeNull();
    expect(videoReading({ kind: 'null' })).toBeNull();
  });
});

describe('VideoPanelVisibilityTest', () => {
  it('a source that is not configured should not hold map space explaining that', () => {
    const off = videoReading({ class: 'Video', available: false, decoding: false, summary: 'No stream URL is set.', activeSource: 0 });

    expect(off?.available).toBe(false);
  });

  it('a configured source that is not decoding yet still earns its panel', () => {
    const waiting = videoReading({ class: 'Video', available: true, decoding: false, summary: 'Waiting for a stream.', activeSource: 0 });

    expect(waiting?.available).toBe(true);
  });
});
