import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { ArcTone, NEAREST_VISIBLE_FRACTION, arcRadiusFraction, arcSweep, arcTone, sampleIsClose } from './ObstacleArc';
import { obstacleRing } from './ObstacleDistance';

const json = (text: string): Json => parseJson(text) ?? {};

describe('ObstacleRingTest', () => {
  const ring = (nonNull: ReadonlyMap<number, number>, size: number = 72): string =>
    Array.from({ length: size }, (_, index) => {
      const value = nonNull.get(index);
      return value === undefined ? 'null' : String(value);
    }).join(',');

  const served = ({
    available = true,
    stale = false,
    increment = '5.0',
    offset = '0.0',
    max = '40.0',
    body = ring(new Map([[18, 3.2]])),
  }: { available?: boolean; stale?: boolean; increment?: string; offset?: string; max?: string; body?: string } = {}) =>
    json(`{"kind":"object","class":"ObstacleDistance","available":${available},"stale":${stale},
        "ringMetres":[${body}],"ringIncrement":${increment},"ringOffset":${offset},
        "rangeMinMetres":0.2,"rangeMaxMetres":${max}}`);

  it("a sector's index places it, and it agrees with nearest", () => {
    const read = obstacleRing(served())!;
    expect(read.samples.length).toBe(1);
    expect(read.samples[0]!.bearingDegrees, 'sector 18 at 5 degrees a step from an offset of zero is due right').toBeCloseTo(90.0, 9);
    expect(read.samples[0]!.metres).toBeCloseTo(3.2, 9);
  });

  it('an empty sector is absent, not an obstacle at zero', () => {
    const read = obstacleRing(served({ body: ring(new Map([[0, 0.0], [36, 7.5]])) }))!;
    expect(read.samples.length).toBe(2);
    expect(read.samples.map((it) => [it.bearingDegrees, it.metres])).toEqual([
      [0.0, 0.0],
      [180.0, 7.5],
    ]);
  });

  it('a ring with no spacing cannot be placed, so it is not drawn', () => {
    expect(obstacleRing(served({ increment: 'null' })), 'without an increment a sample has no angle').toBeNull();
  });

  it('nothing is drawn when the view says there is nothing to draw', () => {
    expect(obstacleRing(served({ available: false }))).toBeNull();
    expect(obstacleRing(null)).toBeNull();
    expect(obstacleRing(served({ body: ring(new Map()) })), 'every sector empty is no arc').toBeNull();
  });

  it('a stale ring still has its samples, and says it is stale', () => {
    const read = obstacleRing(served({ stale: true }))!;
    expect(read.stale).toBe(true);
    expect(read.samples.length).toBe(1);
  });

  it('a live ring is not marked stale', () => {
    expect(obstacleRing(served())).not.toBeNull();
    expect(obstacleRing(served())!.stale).toBe(false);
  });

  it('the ceiling comes from the vehicle, not from a guess', () => {
    expect(obstacleRing(served())!.maxMetres).toBeCloseTo(40.0, 9);
    expect(obstacleRing(served({ max: 'null' })), 'no ceiling means no scale to draw against').toBeNull();
  });

  it('the closest obstacle is never the least visible', () => {
    const ceiling = 40.0;
    expect(arcRadiusFraction(3.2, ceiling)).toBeGreaterThanOrEqual(NEAREST_VISIBLE_FRACTION);
    expect(arcRadiusFraction(30.0, ceiling)).toBeGreaterThan(arcRadiusFraction(3.2, ceiling));
    expect(arcRadiusFraction(40.0, ceiling), 'the ceiling is the rim').toBeCloseTo(1, 6);
    expect(arcRadiusFraction(90.0, ceiling), 'beyond the ceiling is still the rim').toBeCloseTo(1, 6);
    expect(arcRadiusFraction(3.2, 0.0), 'no ceiling, nothing to scale against').toBeCloseTo(0, 6);
  });

  it('the near band does not collapse, where the difference matters most', () => {
    const ceiling = 40.0;
    expect(arcRadiusFraction(3.2, ceiling)).toBeGreaterThan(arcRadiusFraction(0.2, ceiling) * 2);
    expect(arcRadiusFraction(1.0, ceiling)).toBeGreaterThan(arcRadiusFraction(0.2, ceiling));
  });

  it("too near is the vehicle's own judgement, not a pixel threshold", () => {
    expect(sampleIsClose(0.2, 0.2)).toBe(true);
    expect(sampleIsClose(0.5, 0.2)).toBe(false);
    expect(sampleIsClose(0.1, 0.0)).toBe(false);
  });

  it('the ring carries the spacing and floor it was measured with', () => {
    const read = obstacleRing(served())!;
    expect(read.incrementDegrees).toBeCloseTo(5.0, 9);
    expect(read.floorMetres).toBeCloseTo(0.2, 9);
  });

  it('a vehicle reporting no floor has nothing judged close', () => {
    const noFloor = json(`{"kind":"object","class":"ObstacleDistance","available":true,"stale":false,
        "ringMetres":[${ring(new Map([[18, 3.2]]))}],"ringIncrement":5.0,"ringOffset":0.0,
        "rangeMinMetres":null,"rangeMaxMetres":40.0}`);
    const read = obstacleRing(noFloor)!;
    expect(read.floorMetres).toBeCloseTo(0.0, 9);
    expect(sampleIsClose(read.samples[0]!.metres, read.floorMetres)).toBe(false);
  });

  it('a close reading never looks like a distant one', () => {
    expect(arcTone(true, false)).not.toBe(arcTone(false, false));
  });

  it('a stale reading looks like neither', () => {
    expect(arcTone(true, true)).toBe(ArcTone.Stale);
    expect(arcTone(false, true)).toBe(ArcTone.Stale);
    expect(arcTone(true, false)).not.toBe(ArcTone.Stale);
    expect(arcTone(false, false)).not.toBe(ArcTone.Stale);
  });

  it("a sector's wedge is narrower than its spacing so neighbours stay apart", () => {
    expect(arcSweep(5.0)).toBeLessThan(5.0);
    expect(arcSweep(5.0)).toBeGreaterThan(0);
  });
});
