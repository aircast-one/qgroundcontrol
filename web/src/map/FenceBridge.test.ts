import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { FenceCircle, FencePolygon, fenceCircles, fencePolygons, launchBreachesFence, rallyPoints } from './FenceBridge';

const vertex = (latitude: number, longitude: number) => `{"latitude":${latitude},"longitude":${longitude}}`;

const polygon = (vertices: readonly string[], inclusion = true) => `{"inclusion":${inclusion},"vertices":[${vertices.join(',')}]}`;

const model = (...elements: string[]) => parseJson(`{"kind":"object","polygons":[${elements.join(',')}]}`);

const circles = (...elements: string[]) => parseJson(`{"kind":"object","circles":[${elements.join(',')}]}`);

const rally = (...elements: string[]) => parseJson(`{"kind":"object","rallyPoints":[${elements.join(',')}]}`);

const square = [vertex(41.0, 44.0), vertex(41.0, 44.1), vertex(41.1, 44.1), vertex(41.1, 44.0)];

describe('FenceBridgeTest', () => {
  it('a closed polygon keeps its vertices and inclusion', async () => {
    const polygons = await fencePolygons(model(polygon(square, false)));

    expect(polygons.length).toBe(1);
    expect(polygons[0]?.vertices.length).toBe(4);
    expect(polygons[0]?.inclusion).toBe(false);
    expect(polygons[0]?.vertices[0]).toEqual({ latitude: 41.0, longitude: 44.0 });
  });

  it('a polygon that cannot close is not drawn', async () => {
    const polygons = await fencePolygons(model(polygon([vertex(41.0, 44.0), vertex(41.0, 44.1)]), polygon(square)));

    expect(polygons.length).toBe(1);
  });

  it('unusable vertices are dropped and can sink a polygon', async () => {
    const polygons = await fencePolygons(model(polygon([vertex(41.0, 44.0), vertex(0.0, 0.0), vertex(41.1, 44.1)])));

    expect(polygons.length).toBe(0);
  });

  it('a circle reads the radius the core resolved', () => {
    const good = `{"inclusion":true,"centre":${vertex(41.0, 44.0)},"radius":136.0}`;

    const read = fenceCircles(circles(good));

    expect(read.length).toBe(1);
    expect(read[0]?.radius).toBeCloseTo(136.0, 9);
    expect(read[0]?.inclusion).toBe(true);
  });

  it('circles without a centre or a usable radius are dropped', () => {
    const noRadius = `{"inclusion":true,"centre":${vertex(41.0, 44.0)},"radius":0.0}`;
    const noRadiusField = `{"inclusion":true,"centre":${vertex(41.0, 44.0)}}`;
    const noCentre = '{"inclusion":true,"radius":120.0}';
    const otherFact = `{"centre":${vertex(41.0, 44.0)},"radius":null}`;

    expect(fenceCircles(circles(noRadius, noRadiusField, noCentre, otherFact)).length).toBe(0);
  });

  it('rally points read their coordinate', () => {
    const points = rallyPoints(rally(vertex(41.2, 44.2), vertex(0.0, 0.0), '{}'));

    expect(points.length).toBe(1);
    expect(points[0]?.latitude).toBeCloseTo(41.2, 9);
    expect(points[0]?.index).toBe(0);
  });

  it('an absent model yields nothing', async () => {
    expect((await fencePolygons(null)).length).toBe(0);
    expect(fenceCircles(null).length).toBe(0);
    expect(rallyPoints(null).length).toBe(0);
  });
});

describe('LaunchBreachesFenceTest', () => {
  const square = (inclusion: boolean) =>
    FencePolygon({ index: 0, inclusion, vertices: [{ latitude: 41.0, longitude: 44.0 }, { latitude: 41.0, longitude: 44.01 }, { latitude: 41.01, longitude: 44.01 }, { latitude: 41.01, longitude: 44.0 }] });
  const inside = { latitude: 41.005, longitude: 44.005 };
  const outside = { latitude: 41.02, longitude: 44.005 };

  it('a launch outside every keep-in zone breaches the fence on takeoff', () => {
    expect(launchBreachesFence(outside, [square(true)], [])).toBe(true);
    expect(launchBreachesFence(inside, [square(true)], [])).toBe(false);
  });

  it('a launch inside a keep-out zone breaches it, and no fence means nothing to breach', () => {
    expect(launchBreachesFence(inside, [square(false)], [])).toBe(true);
    expect(launchBreachesFence(outside, [square(false)], [])).toBe(false);
    expect(launchBreachesFence(outside, [], [])).toBe(false);
    expect(launchBreachesFence(null, [square(true)], [])).toBe(false);
  });

  it('a keep-in circle around the launch keeps it inside', () => {
    const circle = FenceCircle({ index: 0, inclusion: true, centre: inside, radius: 200, radiusMetres: 200 });
    expect(launchBreachesFence(inside, [], [circle])).toBe(false);
    expect(launchBreachesFence(outside, [], [circle])).toBe(true);
  });
});
