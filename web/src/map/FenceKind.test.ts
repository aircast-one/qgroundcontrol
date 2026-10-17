import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { circlesAsPolygons, fenceCircles, fencePolygons } from './FenceBridge';
import { fenceDetail } from './SelectionValidity';

const view = parseJson(
  `{"kind":"object","polygons":[
     {"index":0,"inclusion":false,"kindText":"Polygon 1",
      "detailText":"3 vertices \\u00b7 2.5 ha","vertices":[
       {"latitude":41.0,"longitude":44.0},{"latitude":41.1,"longitude":44.0},
       {"latitude":41.1,"longitude":44.1}]}],
   "circles":[
     {"index":0,"inclusion":true,"kindText":"Circle 1",
      "centre":{"latitude":41.0,"longitude":44.0},"radius":150.0}]}`,
);

describe('FenceKindTest', () => {
  const circles = fenceCircles(view);

  it('a selected fence says whether it keeps the vehicle in or out', async () => {
    const polygons = await fencePolygons(view);
    expect(fenceDetail({ kind: 'Circle', index: 0 }, polygons, circles)).toBe('Circle 1 · Inclusion');
    expect(fenceDetail({ kind: 'CircleCentre', index: 0 }, polygons, circles)).toBe('Circle 1 · Inclusion');
  });

  it('a fence says what it is and how big, not one or the other', async () => {
    const polygons = await fencePolygons(view);
    expect(fenceDetail({ kind: 'FenceVertex', polygon: 0, vertex: 0 }, polygons, circles)).toBe('Polygon 1 · Exclusion · 3 vertices · 2.5 ha');
  });

  it('a circle drawn as a polygon keeps the wording it was given', () => {
    const drawn = circlesAsPolygons(circles);
    expect(drawn.length).toBe(1);
    expect(drawn[0]?.kindText).toBe('Circle 1');
  });

  it('a selection that is not a fence says nothing', async () => {
    const polygons = await fencePolygons(view);
    expect(fenceDetail({ kind: 'Waypoint', index: 0 }, polygons, circles)).toBeNull();
    expect(fenceDetail(null, polygons, circles)).toBeNull();
    expect(fenceDetail({ kind: 'FenceVertex', polygon: 9, vertex: 0 }, polygons, circles)).toBeNull();
  });
});
