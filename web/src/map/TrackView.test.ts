import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { mergedTrack, plottedTrack, trackDraws, trackReading } from './TrackView';

const served = (points: number, { available = true, from = 0, count = points, generation = 1, vehicleId = 1 } = {}): Json => ({
  class: 'Track',
  available,
  count,
  vehicleId,
  generation,
  from,
  order: 'oldestFirst',
  points: Array.from({ length: points }, (_, it) => ({ latitude: 47.0 + (from + it) * 0.001, longitude: 8.5 })),
});

describe('TrackViewTest', () => {
  it('the trail is whatever the core recorded and nothing the head decided', () => {
    const reading = trackReading(served(3));
    expect(reading.points.length).toBe(3);
    expect(reading.points[0]?.latitude).toBeCloseTo(47.0, 9);
  });

  it('one point is a position, not a line', () => {
    expect(trackDraws(trackReading(served(1)))).toBe(false);
    expect(trackDraws(trackReading(served(2)))).toBe(true);
  });

  it('an unavailable track draws nothing however many points came with it', () => {
    expect(trackDraws(trackReading(served(5, { available: false })))).toBe(false);
    expect(trackDraws(trackReading(null))).toBe(false);
  });

  it('a point the map cannot plot is not drawn but keeps its place in the trail', () => {
    const broken = { class: 'Track', available: true, count: 2, from: 0, points: [{ latitude: 47.0, longitude: 8.5 }, { latitude: null, longitude: null }] };
    expect(trackReading(broken).points.length).toBe(2);
    expect(plottedTrack(trackReading(broken)).length).toBe(1);
  });

  it('a tail joins the trail already held and replaces its moved end point', () => {
    const held = trackReading(served(300));
    const tail = trackReading(served(129, { from: 299, count: 428 }));
    const merged = mergedTrack(held, tail);
    expect(merged?.points.length).toBe(428);
    expect(merged?.from).toBe(0);
    expect(merged?.points).toEqual(trackReading(served(428)).points);
  });

  it('a tail the head cannot join asks for the whole trail', () => {
    const held = trackReading(served(300));
    expect(mergedTrack(held, trackReading(served(128, { from: 400, count: 528 })))).toBeNull();
    expect(mergedTrack(held, trackReading(served(128, { from: 10, count: 138, generation: 2 })))).toBeNull();
    expect(mergedTrack(held, trackReading(served(128, { from: 10, count: 138, vehicleId: 2 })))).toBeNull();
    expect(mergedTrack(null, trackReading(served(128, { from: 10, count: 138 })))).toBeNull();
  });

  it('a short trail arrives whole in the tail', () => {
    const tail = trackReading(served(5));
    expect(mergedTrack(trackReading(served(300, { generation: 7 })), tail)).toEqual(tail);
  });
});
