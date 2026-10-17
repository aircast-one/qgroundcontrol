import { describe, expect, it } from 'vitest';
import { EditableShape } from './EditableShape';
import { FENCE_POLYGONS } from './FenceBridge';
import { PLAN_ITEMS } from './PlanBridge';
import {
  CORRIDOR_PROPERTY,
  ShapeTarget,
  circleRadiusMetres,
  defaultCircle,
  defaultLine,
  defaultRectangle,
  fileShape,
  mainShapeExtension,
  shapeCaption,
  shapeTarget,
  traceCaption,
  traceOutline,
} from './PolygonTools';
import { Survey } from './SurveyBridge';
import { metresBetween } from './TerrainProfile';
import type { TrackPoint } from './VehicleMap';

const point = (latitude: number, longitude: number): TrackPoint => ({ latitude, longitude });

const small = [point(47.401, 8.499), point(47.401, 8.501), point(47.399, 8.501), point(47.399, 8.499)] as const;

const at = (list: readonly TrackPoint[], index: number): TrackPoint => list[index] ?? point(NaN, NaN);

describe('PolygonToolsTest', () => {
  it('rectangle fills three quarters of the view', () => {
    const corners = defaultRectangle(small);
    expect(corners.length).toBe(4);
    const width = metresBetween(small[0], small[1]);
    expect(Math.abs(width * 0.75 - metresBetween(at(corners, 0), at(corners, 1)))).toBeLessThanOrEqual(1.0);
    expect(at(corners, 0).latitude > at(corners, 3).latitude && at(corners, 0).longitude < at(corners, 1).longitude).toBe(true);
  });

  it('a zoomed out view caps the shape at three kilometres', () => {
    const wide = [point(48.0, 8.0), point(48.0, 9.0), point(47.0, 9.0), point(47.0, 8.0)];
    expect(Math.abs(3000.0 - metresBetween(at(defaultRectangle(wide), 0), at(defaultRectangle(wide), 1)))).toBeLessThanOrEqual(5.0);
  });

  it('circle has sixteen corners at the smaller half extent', () => {
    const ring = defaultCircle(small);
    expect(ring.length).toBe(16);
    const centre = point(47.4, 8.5);
    const half = (Math.min(metresBetween(small[0], small[1]), metresBetween(small[0], small[3])) * 0.75) / 2;
    expect(ring.filter((it) => Math.abs(half - metresBetween(centre, it)) > 1.0)).toEqual([]);
  });

  it('no view no shape', () => {
    expect(defaultRectangle([])).toEqual([]);
    expect(defaultCircle(small.slice(0, 3))).toEqual([]);
  });

  it('shape path names the fence or the survey area', () => {
    expect(shapeTarget(2, null)).toEqual(ShapeTarget({ path: `${FENCE_POLYGONS}.2`, line: false }));
    expect(shapeTarget(null, null)).toBeNull();
    const corridor = Survey({ index: 4, area: [], transects: [], cameraShots: 0, kind: '', shape: '', property: CORRIDOR_PROPERTY });
    expect(shapeTarget(null, corridor)).toEqual(ShapeTarget({ path: `${PLAN_ITEMS}.4.${CORRIDOR_PROPERTY}`, line: true }));
  });

  it('a file polygon is read and anything else is refused', () => {
    const area = { shape: 'polygon', error: '', points: [{ latitude: 1.0, longitude: 2.0 }, { latitude: 3.0, longitude: 4.0 }] };
    const polygon = ShapeTarget({ path: 'p', line: false });
    const polyline = ShapeTarget({ path: 'l', line: true });
    expect(fileShape(area, polygon)).toEqual([[point(1.0, 2.0), point(3.0, 4.0)], '']);
    expect(fileShape(area, polyline)[1]).toBe('No polylines found in file');
    expect(fileShape({ shape: 'polyline', error: '' }, polygon)[1]).toBe('No polygons found in file');
    expect(fileShape({ valid: false, error: 'bad coordinate: x' }, polygon)[1]).toBe('bad coordinate: x');
    expect(fileShape(null, polygon)[1]).toBe('No polygons found in file');
  });

  it('a trace closes once it is a polygon', () => {
    expect(traceOutline(small.slice(0, 2))).toEqual(small.slice(0, 2));
    expect(traceOutline(small.slice(0, 3))).toEqual([...small.slice(0, 3), small[0]]);
    expect(traceOutline(small.slice(0, 3), true)).toEqual(small.slice(0, 3));
  });

  it('a default line runs down the middle of the view', () => {
    const [top, bottom] = defaultLine(small);
    expect(top?.longitude).toBeCloseTo(8.5, 9);
    expect(top?.latitude).toBeCloseTo(47.4005, 9);
    expect(bottom?.latitude).toBeCloseTo(47.3995, 9);
  });

  it('a shapefile picked with its prj is read through the shp', () => {
    expect(mainShapeExtension(['area.prj', 'area.SHP'])).toBe('shp');
    expect(mainShapeExtension(['area.kml'])).toBe('kml');
    expect(mainShapeExtension([])).toBeNull();
  });

  it("a typed circle radius is in the app distance units like QGC's Set Radius dialog", () => {
    expect(circleRadiusMetres('100', 0.3048)).toBeCloseTo(30.48, 9);
    expect(circleRadiusMetres('12,5', 1.0)).toBeCloseTo(12.5, 9);
    expect(circleRadiusMetres('0', 0.3048)).toBeNull();
    expect(circleRadiusMetres('wide', 1.0)).toBeNull();
  });

  it("the shape toolbar caption follows QGC's polygon and trace captions", () => {
    const shape = EditableShape({ path: 'p', midpoints: [], splitInvokable: '', canRemoveVertex: false, caption: '1.2 ha · 440 m', circleCaption: 'Radius 50.0 m' });
    expect(shapeCaption(shape, true)).toBe('Radius 50.0 m');
    expect(shapeCaption(shape, false)).toBe('1.2 ha · 440 m');
    expect(shapeCaption({ ...shape, circleCaption: '' }, true)).toBe('1.2 ha · 440 m');
    expect(traceCaption(2, 3)).toBe('Click the map to add points · 2 of 3');
    expect(traceCaption(3, 3)).toBe('3 points');
  });
});
