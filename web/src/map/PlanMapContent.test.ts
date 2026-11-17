import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { FENCE_POLYGONS, FencePolygon } from './FenceBridge';
import { MISSION_SPLIT_INVOKABLE, MISSION_SPLIT_PATH } from './MissionLayers';
import { MissionItem, PLAN_ITEMS } from './PlanBridge';
import { PlanLayer, cornerPosition, dragAllowed, fenceWindow, inclusionText, mapTypes, missionFitPoints, parsedCoordinate, sidePanelWidth, tapAddsRally } from './PlanMapContent';
import { Survey } from './SurveyBridge';
import { TrackPoint } from './VehicleMap';

describe('CenterMenuTest', () => {
  it('a typed coordinate centres only when it is on the globe', () => {
    expect(parsedCoordinate(' 47.5 ', '8.25')).toEqual(TrackPoint(47.5, 8.25));
    expect(parsedCoordinate('91', '8')).toBeNull();
    expect(parsedCoordinate('47', 'east')).toBeNull();
  });

  it('mission fit uses the mission items that have a position', () => {
    const items = [
      MissionItem({ index: 0, sequence: 0, latitude: 47.0, longitude: 8.0, command: 'Home', selected: false }),
      MissionItem({ index: 1, sequence: 1, latitude: NaN, longitude: NaN, command: 'Delay', selected: false }),
      MissionItem({ index: 2, sequence: 2, latitude: 47.1, longitude: 8.1, command: 'Waypoint', selected: false }),
    ];
    expect(missionFitPoints(items)).toEqual([TrackPoint(47.0, 8.0), TrackPoint(47.1, 8.1)]);
  });

  it("a selected corner's position comes from its fence or survey", () => {
    const fence = FencePolygon({ index: 3, inclusion: true, vertices: [TrackPoint(1.0, 2.0), TrackPoint(3.0, 4.0)] });
    const survey = Survey({ index: 5, area: [TrackPoint(5.0, 6.0)], transects: [], cameraShots: 0, kind: 'survey', shape: 'polygon', property: 'surveyAreaPolygon' });
    expect(cornerPosition({ kind: 'FenceVertex', polygon: 3, vertex: 1 }, [fence], [survey])).toEqual(TrackPoint(3.0, 4.0));
    expect(cornerPosition({ kind: 'SurveyVertex', item: 5, vertex: 0 }, [fence], [survey])).toEqual(TrackPoint(5.0, 6.0));
    expect(cornerPosition({ kind: 'FenceVertex', polygon: 3, vertex: 9 }, [fence], [survey])).toBeNull();
  });
});

describe('DragAllowedTest', () => {
  it('any marker on the active layer drags, while shape handles need their shape selected', () => {
    expect(dragAllowed({ kind: 'Waypoint', index: 2 }, { kind: 'Waypoint', index: 2 }, PlanLayer.Mission)).toBe(true);
    expect(dragAllowed({ kind: 'Waypoint', index: 3 }, { kind: 'Waypoint', index: 2 }, PlanLayer.Mission), 'an unselected waypoint drags without being selected first').toBe(true);
    expect(dragAllowed({ kind: 'Waypoint', index: 3 }, null, PlanLayer.Mission)).toBe(true);
    expect(dragAllowed({ kind: 'Rally', index: 1 }, null, PlanLayer.Rally)).toBe(true);
    expect(dragAllowed({ kind: 'Waypoint', index: 3 }, null, PlanLayer.Fence), 'a waypoint stays put while the fence is edited').toBe(false);
    expect(dragAllowed({ kind: 'SurveyVertex', item: 4, vertex: 1 }, { kind: 'Waypoint', index: 2 }, PlanLayer.Mission), "another survey's corners stay put").toBe(false);
    expect(dragAllowed({ kind: 'SurveyVertex', item: 2, vertex: 1 }, { kind: 'Waypoint', index: 2 }, PlanLayer.Mission), "a selected survey's corners move").toBe(true);
    expect(dragAllowed({ kind: 'Rally', index: 0 }, { kind: 'Rally', index: 0 }, PlanLayer.Mission), 'a rally point is not dragged from the mission layer').toBe(false);
    expect(dragAllowed({ kind: 'Rally', index: 0 }, { kind: 'Rally', index: 0 }, PlanLayer.Rally)).toBe(true);
    expect(dragAllowed({ kind: 'FenceVertex', polygon: 0, vertex: 1 }, null, PlanLayer.Fence), 'nothing selected, nothing dragged').toBe(false);
    expect(dragAllowed({ kind: 'FenceVertex', polygon: 0, vertex: 1 }, { kind: 'FenceVertex', polygon: 0, vertex: 0 }, PlanLayer.Fence)).toBe(true);
    expect(dragAllowed({ kind: 'ShapeRadius', fence: true, owner: 1 }, { kind: 'FenceVertex', polygon: 1, vertex: 0 }, PlanLayer.Fence), "a selected polygon's radius handle is its own").toBe(true);
  });

  it('a midpoint splits only the selected shape', () => {
    expect(dragAllowed({ kind: 'Midpoint', path: `${FENCE_POLYGONS}.1`, invokable: 'split', segment: 0 }, { kind: 'FenceVertex', polygon: 1, vertex: 0 }, PlanLayer.Fence)).toBe(true);
    expect(dragAllowed({ kind: 'Midpoint', path: `${FENCE_POLYGONS}.1`, invokable: 'split', segment: 0 }, { kind: 'FenceVertex', polygon: 0, vertex: 0 }, PlanLayer.Fence)).toBe(false);
    expect(dragAllowed({ kind: 'Midpoint', path: `${PLAN_ITEMS}.2.surveyAreaPolygon`, invokable: 'split', segment: 0 }, { kind: 'Waypoint', index: 2 }, PlanLayer.Mission)).toBe(true);
    expect(dragAllowed({ kind: 'Midpoint', path: `${PLAN_ITEMS}.2.surveyAreaPolygon`, invokable: 'split', segment: 0 }, { kind: 'Waypoint', index: 3 }, PlanLayer.Mission)).toBe(false);
    expect(dragAllowed({ kind: 'Midpoint', path: MISSION_SPLIT_PATH, invokable: MISSION_SPLIT_INVOKABLE, segment: 2 }, { kind: 'Waypoint', index: 2 }, PlanLayer.Mission)).toBe(true);
  });
});

describe('MapTypesTest', () => {
  it("the plan map menu lists the provider's types and marks the current one", () => {
    const read = mapTypes(parseJson('{"current":"Satellite","types":["Street Map","Satellite"],"path":"settings.flightMapSettings.mapType.rawValue"}'));
    expect(read).toEqual({ current: 'Satellite', types: ['Street Map', 'Satellite'], path: 'settings.flightMapSettings.mapType.rawValue' });
    expect(mapTypes(parseJson('{"kind":"null"}'))).toBeNull();
  });
});

describe('FenceInclusionTest', () => {
  it("a new fence is sized from the visible map, like GeoFenceEditor's viewport corners", () => {
    const view = [TrackPoint(48.0, 8.0), TrackPoint(48.0, 9.0), TrackPoint(47.0, 9.0), TrackPoint(47.0, 8.0)];
    expect(fenceWindow(view, TrackPoint(47.5, 8.5))).toEqual([TrackPoint(48.0, 8.0), TrackPoint(47.0, 9.0)]);
    const [topLeft, bottomRight] = fenceWindow([], TrackPoint(47.5, 8.5));
    expect(topLeft.latitude).toBeCloseTo(47.502, 9);
    expect(topLeft.longitude).toBeCloseTo(8.498, 9);
    expect(bottomRight.latitude).toBeCloseTo(47.498, 9);
    expect(bottomRight.longitude).toBeCloseTo(8.502, 9);
  });

  it('each click on the empty map in Rally adds a point, even with the last one selected', () => {
    expect(tapAddsRally(PlanLayer.Rally, { kind: 'Rally', index: 0 }, true)).toBe(true);
    expect(tapAddsRally(PlanLayer.Rally, null, true)).toBe(true);
    expect(tapAddsRally(PlanLayer.Rally, { kind: 'Waypoint', index: 2 }, true)).toBe(false);
    expect(tapAddsRally(PlanLayer.Mission, null, true)).toBe(false);
    expect(tapAddsRally(PlanLayer.Rally, null, false)).toBe(false);
  });

  it('the fence switch says whether the shape keeps the aircraft in or out', () => {
    expect([inclusionText(true), inclusionText(false)]).toEqual(['Keep-in', 'Keep-out']);
  });

  it('the side panel narrows on a laptop-width window so the map keeps room', () => {
    expect([sidePanelWidth(1024), sidePanelWidth(1199), sidePanelWidth(1440)]).toEqual([320, 320, 380]);
  });
});
