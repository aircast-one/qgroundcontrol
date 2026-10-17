import { describe, expect, it } from 'vitest';
import { FenceCircle, FencePolygon, RallyPoint } from './FenceBridge';
import { PlanBounds, centersOnVehicleAtEntry, clearWindow, fitPoints, fitsPlanOnEntry, longitudeArc, normaliseLongitude, planBounds, planIsDrawn, planPoints, stillFirstRead } from './PlanBounds';
import { KIND_SURVEY, MissionItem, takeoffMissing } from './PlanBridge';
import { SHAPE_AREA, Survey } from './SurveyBridge';
import { TrackPoint } from './VehicleMap';

const item = (latitude: number, longitude: number) => MissionItem({ index: 0, sequence: 1, latitude, longitude, command: 'Waypoint', selected: false, altitude: NaN });

describe('PlanBoundsTest', () => {
  it('bounds cover every corner of the plan', () => {
    const bounds = planBounds(planPoints([item(41.0, 44.0), item(41.5, 44.5), item(41.2, 43.8)]))!;

    expect(bounds.south).toBeCloseTo(41.0, 9);
    expect(bounds.west).toBeCloseTo(43.8, 9);
    expect(bounds.north).toBeCloseTo(41.5, 9);
    expect(bounds.east).toBeCloseTo(44.5, 9);
  });

  it('a circle contributes its ring so a fit does not cut it in half', () => {
    const centre = TrackPoint(41.0, 44.0);
    const circle = FenceCircle({ index: 0, inclusion: true, centre, radius: 500.0 });

    const bounds = planBounds(planPoints([], [], [circle]))!;

    expect(bounds.north).toBeGreaterThan(centre.latitude);
    expect(bounds.south).toBeLessThan(centre.latitude);
    expect(bounds.east).toBeGreaterThan(centre.longitude);
    expect(bounds.west).toBeLessThan(centre.longitude);
  });

  it('fences rally points and surveys all count', () => {
    const points = planPoints(
      [],
      [FencePolygon({ index: 0, inclusion: true, vertices: [TrackPoint(40.0, 43.0)] })],
      [],
      [RallyPoint({ index: 0, latitude: 42.0, longitude: 45.0 })],
      [Survey({ index: 0, area: [TrackPoint(41.0, 44.0)], transects: [TrackPoint(41.9, 44.9)], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: 'surveyAreaPolygon' })],
    );

    const bounds = planBounds(points)!;

    expect(bounds.south).toBeCloseTo(40.0, 9);
    expect(bounds.west).toBeCloseTo(43.0, 9);
    expect(bounds.north).toBeCloseTo(42.0, 9);
    expect(bounds.east).toBeCloseTo(45.0, 9);
  });

  it('an empty or unplottable plan has no bounds to fit', () => {
    expect(planBounds([])).toBeNull();
    expect(planBounds([TrackPoint(0.0, 0.0)])).toBeNull();
  });

  it('a single point still yields bounds centred on itself', () => {
    const bounds = planBounds([TrackPoint(41.0, 44.0)])!;

    expect(bounds.centre).toEqual(TrackPoint(41.0, 44.0));
    expect(bounds.spanDegrees).toBeCloseTo(0.0, 9);
  });
});

describe('FitPointsTest', () => {
  it('a plan is framed on its own terms', () => {
    const plan = planPoints([item(41.0, 44.0), item(41.5, 44.5)]);

    expect(fitPoints(plan, -35.36, 149.16)).toEqual(plan);
  });

  it('an empty plan falls back to the aircraft', () => {
    expect(fitPoints([], -35.36, 149.16)).toEqual([TrackPoint(-35.36, 149.16)]);
  });

  it('with no plan and no position there is nothing to frame', () => {
    expect(fitPoints([], NaN, NaN)).toEqual([]);
    expect(fitPoints([], 0.0, 0.0)).toEqual([]);
  });
});

describe('LongitudeWraparoundTest', () => {
  const at = (...longitudes: number[]) => planBounds(longitudes.map((it) => TrackPoint(10.0, it)))!;

  it('two points either side of the antimeridian do not frame the planet', () => {
    const bounds = at(179.9, -179.9);

    expect(bounds.longitudeSpan).toBeCloseTo(0.2, 9);
    expect(bounds.centre.longitude).toBeCloseTo(180.0, 9);
  });

  it('an ordinary region is unchanged by the arc treatment', () => {
    const bounds = at(44.7, 44.9, 44.8);

    expect(bounds.west).toBeCloseTo(44.7, 9);
    expect(bounds.east).toBeCloseTo(44.9, 9);
    expect(bounds.longitudeSpan).toBeCloseTo(0.2, 9);
    expect(bounds.centre.longitude).toBeCloseTo(44.8, 9);
  });

  it('the widest empty gap is the one left out of the arc', () => {
    const bounds = at(-10.0, 10.0, 170.0);

    expect(bounds.west).toBeCloseTo(-10.0, 9);
    expect(bounds.east).toBeCloseTo(170.0, 9);
    expect(bounds.longitudeSpan).toBeCloseTo(180.0, 9);
    expect(bounds.centre.longitude).toBeCloseTo(80.0, 9);
  });

  it('a single point spans nothing and centres on itself', () => {
    const bounds = at(-179.95);

    expect(bounds.longitudeSpan).toBeCloseTo(0.0, 9);
    expect(bounds.centre.longitude).toBeCloseTo(-179.95, 9);
  });

  it('the span never exceeds the planet', () => {
    const bounds = at(-179.0, -90.0, 0.0, 90.0, 179.0);

    expect(bounds.longitudeSpan).toBeLessThanOrEqual(360.0);
    expect(bounds.longitudeSpan).toBeCloseTo(270.0, 9);
  });

  it('normalising keeps a longitude in range', () => {
    expect(normaliseLongitude(181.0)).toBeCloseTo(-179.0, 9);
    expect(normaliseLongitude(-181.0)).toBeCloseTo(179.0, 9);
    expect(normaliseLongitude(180.0)).toBeCloseTo(180.0, 9);
    expect(normaliseLongitude(720.0)).toBeCloseTo(0.0, 9);
  });
});

describe('TakeoffMissingTest', () => {
  const kinded = (kind: string, name: string, lat = 41.0, lon = 44.0) => MissionItem({ index: 0, sequence: 1, latitude: lat, longitude: lon, command: name, selected: false, altitude: 50.0, kind });

  it('waypoints with no takeoff need one inserted first', () => {
    expect(takeoffMissing([kinded('waypoint', 'Waypoint'), kinded('waypoint', 'Waypoint')])).toBe(true);
  });

  it('a plan that already has a takeoff does not get another', () => {
    expect(takeoffMissing([kinded('takeoff', 'Takeoff'), kinded('waypoint', 'Waypoint')])).toBe(false);
  });

  it('a VTOL takeoff counts as a takeoff', () => {
    expect(takeoffMissing([kinded('takeoff', 'VTOL Takeoff'), kinded('waypoint', 'Waypoint')])).toBe(false);
  });

  it('a takeoff named in another language still counts', () => {
    expect(takeoffMissing([kinded('takeoff', 'Starten'), kinded('waypoint', 'Wegpunkt')])).toBe(false);
  });

  it('an empty plan needs nothing inserted before anything', () => {
    expect(takeoffMissing([])).toBe(false);
  });

  it('items with no coordinate do not by themselves require a takeoff', () => {
    expect(takeoffMissing([kinded('command', 'Return To Launch', 0.0, 0.0)])).toBe(false);
  });
});

describe('FitsPlanOnEntryTest', () => {
  it('a plan that was already there is what the tab opens on', () => {
    expect(fitsPlanOnEntry(true, true, true)).toBe(true);
  });

  it('an empty plan leaves the camera on the vehicle', () => {
    expect(fitsPlanOnEntry(true, true, false)).toBe(false);
  });

  it('the first item added later does not yank the camera', () => {
    expect(fitsPlanOnEntry(false, true, true)).toBe(false);
  });

  it('a read that never reached the plan does not spend the one chance to fit', () => {
    expect(fitsPlanOnEntry(true, false, false)).toBe(false);
    expect(stillFirstRead(true, false)).toBe(true);
  });

  it('a read that reached the plan settles it, whatever the plan held', () => {
    expect(stillFirstRead(true, true)).toBe(false);
    expect(stillFirstRead(false, false)).toBe(false);
  });

  it('a plan is drawn if anything at all is on the map', () => {
    expect(planIsDrawn([], [], [], [], [])).toBe(false);
    expect(planIsDrawn([], [], [], [], [RallyPoint({ index: 0, latitude: 41.0, longitude: 44.0 })])).toBe(true);
  });

  it('the plan map centres on the vehicle once on entry and then stays where the operator put it', () => {
    expect(centersOnVehicleAtEntry(false, true, 0)).toBe(true);
    expect(centersOnVehicleAtEntry(true, true, 0), 'PlanView never follows the vehicle; the Center menu moves the map').toBe(false);
    expect(centersOnVehicleAtEntry(false, true, 1), 'a mission fitted on entry wins').toBe(false);
    expect(centersOnVehicleAtEntry(false, false, 0)).toBe(false);
  });

  it('the clear window squares off a rotated view so a new fence lands where the operator can see it', () => {
    const rotated = [TrackPoint(41.002, 44.0), TrackPoint(41.001, 44.003), TrackPoint(40.998, 44.002), TrackPoint(40.999, 43.999)];

    expect(clearWindow(rotated)).toEqual([TrackPoint(41.002, 43.999), TrackPoint(41.002, 44.003), TrackPoint(40.998, 44.003), TrackPoint(40.998, 43.999)]);
  });
});

describe('LongitudeArcTest', () => {
  it('a plan either side of the antimeridian keeps the short arc across it', () => {
    expect(
      longitudeArc([170.0, -170.0]),
      'sorting puts -170 first, so the naive west-to-east span is the 340 degrees the plan does NOT occupy - the whole point of picking the widest gap is to return the 20 that it does',
    ).toEqual([170.0, -170.0]);
  });

  it('a fit across the antimeridian hands MapLibre an east past 180, never one below west', () => {
    expect(PlanBounds({ south: 0.0, west: 170.0, north: 1.0, east: -170.0 }).unwrappedEast).toBeCloseTo(190.0, 9);
    expect(PlanBounds({ south: 0.0, west: 10.0, north: 1.0, east: 20.0 }).unwrappedEast).toBeCloseTo(20.0, 9);
  });

  it('an ordinary plan spans west to east the obvious way', () => {
    expect(longitudeArc([20.0, 10.0])).toEqual([10.0, 20.0]);
    expect(longitudeArc([-10.0, -20.0])).toEqual([-20.0, -10.0]);
  });

  it('one point and no points are both answerable', () => {
    expect(longitudeArc([42.0])).toEqual([42.0, 42.0]);
    expect(longitudeArc([42.0, 42.0])).toEqual([42.0, 42.0]);
    expect(longitudeArc([])).toEqual([0.0, 0.0]);
  });

  it('a longitude past the wrap is brought back before anything is compared', () => {
    expect(normaliseLongitude(-180.0)).toBe(180.0);
    expect(normaliseLongitude(181.0)).toBe(-179.0);
    expect(normaliseLongitude(-181.0)).toBe(179.0);
    expect(normaliseLongitude(360.0)).toBe(0.0);
  });

  it('three points straddling the wrap still take the arc containing them', () => {
    expect(
      longitudeArc([175.0, -175.0, 10.0]),
      'going east the gaps are -175 to 10 (185 degrees), 10 to 175 (165) and 175 to -175 (10). The arc must exclude the widest, so it runs from 10 east across the antimeridian to -175 and is 175 degrees wide - starting at 175 instead would be 195 and would not be the containing arc',
    ).toEqual([10.0, -175.0]);
  });
});
