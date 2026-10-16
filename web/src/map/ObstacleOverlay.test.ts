import { describe, expect, it } from 'vitest';
import { format } from '@/kotlin/Std';
import { type ObstacleOverlay, type Offset, mapOverlayLabels, mapOverlayShape, obstacleOverlay, rangeIndex, videoOverlaySegments } from './ObstacleOverlay';

const overlay = (ranges: readonly number[], increment = 90.0, offset = 0.0, max = 40.0): ObstacleOverlay => ({
  ranges,
  texts: ranges.map((it) => format('%.2f', it)),
  increment,
  offset,
  maxMetres: max,
});

const expectOffset = (actual: Offset | undefined, x: number, y: number) => {
  expect(actual?.x).toBeCloseTo(x, 3);
  expect(actual?.y).toBeCloseTo(y, 3);
};

describe('ObstacleOverlayTest', () => {
  it('the overlay reads the core drawing and needs an increment and a range', () => {
    const view = { ringIncrement: 5.0, ringOffset: 0.0, overlay: { ranges: [3.2, 655.35], texts: ['3.20', '655.35'], maxMetres: 40.0 } };
    expect(obstacleOverlay(view)).toEqual({ ranges: [3.2, 655.35], texts: ['3.20', '655.35'], increment: 5.0, offset: 0.0, maxMetres: 40.0 });
    expect(obstacleOverlay({ ringIncrement: null, overlay: { ranges: [3.2], texts: ['3.20'], maxMetres: 40.0 } })).toBeNull();
    expect(obstacleOverlay({ ringIncrement: 5.0, overlay: null })).toBeNull();
  });

  it('range index rounds up and turns with heading like rangeIdx', () => {
    const ring = overlay([1.0, 2.0, 3.0, 4.0]);
    expect(rangeIndex(0.0, ring, 0.0)).toBe(0);
    expect(rangeIndex(10.0, ring, 0.0)).toBe(1);
    expect(rangeIndex(0.0, ring, 90.0)).toBe(3);
    expect(rangeIndex(90.0, overlay([1.0, 2.0, 3.0, 4.0], 90.0, 180.0), 0.0)).toBe(3);
  });

  it('map points sit between the inner and outer radius and flip to true scale when zoomed in', () => {
    const ring = overlay([40.0, 20.0, 0.0, 10.0]);
    const shape = mapOverlayShape(ring, { x: 500, y: 500 }, 1000, 100.0, 100, 0.0, 0.0);
    expect(shape.gradientFrom).toBeCloseTo(90, 3);
    expect(shape.gradientTo).toBeCloseTo(450, 3);
    expectOffset(shape.points[0]?.outer, 500, 50);
    expectOffset(shape.points[1]?.outer, 770, 500);
    expectOffset(shape.points[2]?.outer, 500, 590);
    const zoomed = mapOverlayShape(ring, { x: 500, y: 500 }, 1000, 2.0, 100, 0.0, 0.0);
    expect(zoomed.gradientFrom).toBe(0);
    expect(zoomed.gradientTo).toBeCloseTo(2000, 3);
  });

  it('map labels take the nearer of each pair and skip repeats within two metres', () => {
    const ring = overlay([10.0, 9.0, 30.0, 11.0, 12.0, 30.0, 25.0, 50.0, 50.0], 40.0);
    const shape = mapOverlayShape(ring, { x: 0, y: 0 }, 1000, 100.0, 100, 0.0, 0.0);
    expect(mapOverlayLabels(ring, shape.points).map((it) => it.range)).toEqual([9.0, 25.0]);
  });

  it('video segments stop at the level that reaches the nearest range and label only it', () => {
    const ring = overlay(Array.from({ length: 16 }, (_, it) => (it === 0 ? 15.0 : 655.35)), 22.5);
    const placed = videoOverlaySegments(ring, 1000, true);
    expect(placed).not.toBeNull();
    if (placed === null) return;
    const [radii, segments] = placed;
    expect(Math.abs(radii[0] - (450 - ((450 * 0.2) / 8) * 4 * 2))).toBeLessThanOrEqual(0.01);
    const first = segments.filter((it) => it.radFrom === 0.0);
    expect(first.length).toBe(3);
    expect(first.map((it) => it.label)).toEqual([null, null, '15.00']);
    expect(segments.length).toBe(15 + 3);
    expect(videoOverlaySegments(overlay([1.0], 90.0, 0.0, 250.0), 1000, true)).toBeNull();
  });
});
