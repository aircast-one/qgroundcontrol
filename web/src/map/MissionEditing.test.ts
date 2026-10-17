import { describe, expect, it } from 'vitest';
import { TAP_SLOP_PX, handleHit, nearestIndex, withinTap } from './MissionEditing';
import { HANDLE_KIND_CIRCLE, HANDLE_KIND_FENCE, HANDLE_KIND_SURVEY } from './MissionLayers';

describe('HandleHitTest', () => {
  it('each handle kind gives its own hit', () => {
    expect(handleHit(HANDLE_KIND_FENCE, 1, 2)).toEqual({ kind: 'FenceVertex', polygon: 1, vertex: 2 });
    expect(handleHit(HANDLE_KIND_SURVEY, 1, 2)).toEqual({ kind: 'SurveyVertex', item: 1, vertex: 2 });
    expect(handleHit(HANDLE_KIND_CIRCLE, 1, 2)).toEqual({ kind: 'CircleCentre', index: 1 });
  });

  it('a handle kind nobody knows is not a fence vertex', () => {
    expect(handleHit('corridor', 1, 2)).toBeNull();
    expect(handleHit(null, 1, 2)).toBeNull();
    expect(handleHit('', 1, 2)).toBeNull();
  });
});

describe('WithinTapTest', () => {
  it('a finger that barely moves is a tap', () => {
    expect(withinTap(0, 0)).toBe(true);
    expect(withinTap(TAP_SLOP_PX, -TAP_SLOP_PX)).toBe(true);
  });

  it('a finger that travels is a pan, and must not clear the selection', () => {
    expect(withinTap(TAP_SLOP_PX + 1, 0)).toBe(false);
    expect(withinTap(0, TAP_SLOP_PX + 1)).toBe(false);
  });
});

describe('NearestHitTest', () => {
  it('the closest point wins rather than the first drawn', () => {
    expect(nearestIndex(10, 10, [[100, 100], [12, 12], [60, 60]])).toBe(1);
  });

  it('a tie goes to the one drawn first', () => {
    expect(nearestIndex(10, 10, [[20, 10], [0, 10]])).toBe(0);
  });

  it('something that did not project sorts last', () => {
    expect(nearestIndex(10, 10, [null, [40, 40]])).toBe(1);
  });

  it('but still wins when it is all there is', () => {
    expect(nearestIndex(10, 10, [null, null])).toBe(0);
  });

  it('nothing under the tap is no hit', () => {
    expect(nearestIndex(10, 10, [])).toBeNull();
  });
});
