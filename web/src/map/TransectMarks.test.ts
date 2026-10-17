import { describe, expect, it } from 'vitest';
import { EditableShape, edgeLabels } from './EditableShape';
import { FencePolygon } from './FenceBridge';
import { WAYPOINT_LABEL_PROPERTY, exitMarkers, flownRoute, legArrows, missionFeatures } from './MissionLayers';
import { MissionItem } from './PlanBridge';
import { polygonCentre, shapeMovedTo } from './PolygonTools';
import { gimbalWedges, transectMarks } from './TransectMarks';
import type { TrackPoint } from './VehicleMap';

const near = (actual: number | undefined, expected: number, delta: number, message?: string) => expect(Math.abs((actual ?? NaN) - expected), message).toBeLessThanOrEqual(delta);

describe('TransectMarksTest', () => {
  const lawnmower = (transects: number, turnaround: boolean): readonly TrackPoint[] =>
    Array.from({ length: transects }, (_, row) => row).flatMap((row) => {
      const latitude = 47.0 + row * 0.001;
      const pair: readonly TrackPoint[] = [
        { latitude, longitude: 8.0 },
        { latitude, longitude: 8.01 },
      ];
      const ends = row % 2 === 0 ? pair : pair.toReversed();
      const [a, b] = ends;
      return a === undefined || b === undefined ? [] : turnaround ? [a, a, b, b] : [a, b];
    });

  it('the current survey shows entry and exit arrows, two each once there are more than three transects', () => {
    expect(transectMarks(lawnmower(3, false), false, true).arrows.length).toBe(2);
    const many = transectMarks(lawnmower(5, false), false, true);
    expect(many.arrows.length).toBe(4);
    near(many.arrows[0]?.bearing, 90.0, 1.0);
    expect(many.stubs.length).toBe(0);
  });

  it('another survey with turnarounds shows only its entry and exit stubs', () => {
    const marks = transectMarks(lawnmower(4, true), true, false);
    expect(marks.arrows.length).toBe(0);
    expect(marks.stubs.length).toBe(2);
    expect(transectMarks(lawnmower(4, false), false, false).stubs.length).toBe(0);
  });

  const leg = (index: number, complex = false): MissionItem =>
    MissionItem({ index, sequence: index, latitude: 47.0 + index * 0.001, longitude: 8.0, command: '', selected: false, complexPattern: complex });

  it('mission legs carry arrows on the second leg, every sixth after, and at pattern boundaries', () => {
    const plain = Array.from({ length: 15 }, (_, it) => leg(it));
    const arrows = legArrows(plain, true);
    expect(arrows.length, 'MissionController: the home-to-first leg has none, one when the count passes five, and the last leg always').toBe(2);
    const withSurvey = Array.from({ length: 5 }, (_, it) => leg(it, it === 3));
    expect(legArrows(withSurvey, true).length, 'both legs touching a pattern get one').toBe(2);
    const first = arrows[0]?.at.latitude ?? NaN;
    expect(first > 47.007 && first < 47.008, 'arrows sit three quarters along the leg').toBe(true);
  });

  it("a waypoint's gimbal wedge points at its heading plus the gimbal yaw", () => {
    const looking = { ...leg(2), heading: 90.0, gimbalYaw: 30.0 };
    const bare = { ...leg(3), heading: 90.0 };
    expect(gimbalWedges([looking, bare]).map((it) => it.bearing)).toEqual([120.0]);
  });

  it('a pattern item gets a second marker at its exit labelled with its last sequence number', () => {
    const survey = { ...leg(2, true), exit: { latitude: 47.5, longitude: 8.1 }, foldedCommands: 6 };
    const plain = { ...leg(3), exit: { latitude: 47.6, longitude: 8.1 } };
    expect(exitMarkers([survey, plain])).toEqual([[survey, { latitude: 47.5, longitude: 8.1 }]]);
    const labels = missionFeatures([survey]).features.map((it) => it.properties?.[WAYPOINT_LABEL_PROPERTY]);
    expect(labels, 'TransectStyleMapVisuals labels the exit with lastSequenceNumber').toEqual(['2', '8']);
  });

  it('dragging a fence corner labels every edge of that fence with its length', () => {
    const shape = EditableShape({
      path: 'p',
      midpoints: [
        { latitude: 47.0, longitude: 8.0 },
        { latitude: 47.1, longitude: 8.0 },
      ],
      splitInvokable: '',
      canRemoveVertex: true,
      edgeLengths: ['12.0 m', '8.5 m'],
    });
    const fence = FencePolygon({ index: 0, inclusion: true, vertices: [], editable: shape });
    expect(edgeLabels({ kind: 'FenceVertex', polygon: 0, vertex: 1 }, [fence], []).map((it) => it.text)).toEqual(['12.0 m', '8.5 m']);
    expect(edgeLabels(null, [fence], []).length, 'nothing while no vertex is dragged').toBe(0);
  });

  it('an rtl closes the route back to home and its leg carries the last arrow', () => {
    const home = { ...leg(0), closesRoute: true };
    const route = flownRoute([home, leg(1), leg(2)], false);
    expect(route.map((it) => it.index)).toEqual([1, 2, 0]);
    near(legArrows([home, leg(1), leg(2)], false).at(-1)?.at.latitude, 47.0005, 0.0002, 'the last arrow sits on the leg home');
  });

  it("dragging a polygon's centre moves every corner by the same offset", () => {
    const square: readonly TrackPoint[] = [
      { latitude: 47.0, longitude: 8.0 },
      { latitude: 47.0, longitude: 8.002 },
      { latitude: 47.002, longitude: 8.002 },
      { latitude: 47.002, longitude: 8.0 },
    ];
    const centre = polygonCentre(square);
    near(centre?.latitude, 47.001, 1e-9);
    near(centre?.longitude, 8.001, 1e-9);
    const moved = shapeMovedTo(square, { latitude: 47.011, longitude: 8.001 });
    near(moved?.[0]?.latitude, 47.01, 1e-6, "QGCMapPolygon::setCenter shifts each vertex by the centre's displacement");
    near(moved?.[0]?.longitude, 8.0, 1e-6);
    expect(polygonCentre(square.slice(0, 2)), 'a line has no centre handle').toBeNull();
  });
});
