import { describe, expect, it } from 'vitest';
import { FenceRow, fenceHeading, fenceRowHit, rallyAfterRemove, rowSelected } from './FenceBridge';

describe('FenceListTest', () => {
  it('a row selects its shape and a deleted rally point hands selection to its neighbour', () => {
    const polygon = FenceRow({ index: 1, circle: false, title: 'Polygon 2', detail: '', inclusion: false });
    const circle = FenceRow({ index: 0, circle: true, title: 'Circle 1', detail: '' });
    expect(fenceRowHit(polygon)).toEqual({ kind: 'FenceVertex', polygon: 1, vertex: 0 });
    expect(fenceRowHit(circle)).toEqual({ kind: 'Circle', index: 0 });
    expect(rowSelected(polygon, { kind: 'FenceVertex', polygon: 1, vertex: 3 })).toBe(true);
    expect(rallyAfterRemove(1, 3), 'RallyPointController selects the next point').toEqual({ kind: 'Rally', index: 1 });
    expect(rallyAfterRemove(2, 3), 'or the new last one').toEqual({ kind: 'Rally', index: 1 });
    expect(rallyAfterRemove(0, 1)).toBeNull();
  });

  it('polygon and circle rows sit under GeoFenceEditor section labels', () => {
    const first = FenceRow({ index: 0, circle: false, title: 'a', detail: '' });
    const second = FenceRow({ index: 1, circle: false, title: 'b', detail: '' });
    const circle = FenceRow({ index: 0, circle: true, title: 'c', detail: '' });
    expect(fenceHeading(first, null)).toBe('Polygon fences');
    expect(fenceHeading(second, first)).toBeNull();
    expect(fenceHeading(circle, second)).toBe('Circular fences');
    expect(fenceHeading(circle, null)).toBe('Circular fences');
  });
});
