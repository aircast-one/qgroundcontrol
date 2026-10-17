import { describe, expect, it } from 'vitest';
import { EditableShape } from './EditableShape';
import { FenceCircle, FencePolygon, RallyPoint, cornerRemovable } from './FenceBridge';
import type { MapHit } from './MissionEditing';
import { KIND_SURVEY, MissionItem } from './PlanBridge';
import { selectedSurvey, selectionSurvives } from './SelectionValidity';
import { SHAPE_AREA, Survey } from './SurveyBridge';

describe('SelectionValidityTest', () => {
  const item = (index: number) => MissionItem({ index, sequence: index + 1, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: NaN });

  const survives = (selected: MapHit | null, items: readonly MissionItem[] = []) => selectionSurvives(selected, items, [], [], [], []);

  it('no selection always survives', () => {
    expect(survives(null)).toBe(true);
  });

  it('a waypoint that is still there survives', () => {
    expect(survives({ kind: 'Waypoint', index: 2 }, [item(1), item(2)])).toBe(true);
  });

  it('a waypoint the plan no longer holds does not', () => {
    expect(survives({ kind: 'Waypoint', index: 2 }, [item(0), item(1)])).toBe(false);
    expect(survives({ kind: 'Waypoint', index: 0 }, [])).toBe(false);
  });

  it('a circle selected by either handle follows the circle', () => {
    const circles = [FenceCircle({ index: 1, inclusion: true, centre: { latitude: 41.0, longitude: 44.0 }, radius: 100.0 })];
    const gone: readonly FenceCircle[] = [];

    expect(selectionSurvives({ kind: 'Circle', index: 1 }, [], [], circles, [], [])).toBe(true);
    expect(selectionSurvives({ kind: 'CircleCentre', index: 1 }, [], [], circles, [], [])).toBe(true);
    expect(selectionSurvives({ kind: 'Circle', index: 1 }, [], [], gone, [], [])).toBe(false);
  });

  it('a vertex outside the shape it names does not survive', () => {
    const polygons = [FencePolygon({ index: 0, inclusion: true, vertices: [{ latitude: 41.0, longitude: 44.0 }, { latitude: 41.1, longitude: 44.1 }] })];

    expect(selectionSurvives({ kind: 'FenceVertex', polygon: 0, vertex: 1 }, [], polygons, [], [], [])).toBe(true);
    expect(selectionSurvives({ kind: 'FenceVertex', polygon: 0, vertex: 5 }, [], polygons, [], [], [])).toBe(false);
  });

  it('a rally point that has gone does not survive', () => {
    const rally = [RallyPoint({ index: 3, latitude: 41.0, longitude: 44.0 })];

    expect(selectionSurvives({ kind: 'Rally', index: 3 }, [], [], [], rally, [])).toBe(true);
    expect(selectionSurvives({ kind: 'Rally', index: 4 }, [], [], [], rally, [])).toBe(false);
  });
});

describe('SelectedSurveyTest', () => {
  const survey = (index: number) => Survey({ index, area: [{ latitude: 41.0, longitude: 44.0 }], transects: [], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: 'surveyAreaPolygon' });

  const surveys = [survey(1), survey(4)];

  it('the selected survey is the one acted on, not the first', () => {
    expect(selectedSurvey({ kind: 'SurveyVertex', item: 4, vertex: 0 }, surveys)?.index).toBe(4);
    expect(selectedSurvey({ kind: 'SurveyVertex', item: 1, vertex: 0 }, surveys)?.index).toBe(1);
  });

  it('nothing selected means no survey to act on', () => {
    expect(selectedSurvey(null, surveys)).toBeNull();
    expect(selectedSurvey({ kind: 'Waypoint', index: 2 }, surveys)).toBeNull();
  });

  it('a survey picked from the list is the same survey as one picked by its corner', () => {
    expect(selectedSurvey({ kind: 'Waypoint', index: 4 }, surveys)?.index).toBe(4);
    expect(selectedSurvey({ kind: 'Waypoint', index: 1 }, surveys)).toEqual(selectedSurvey({ kind: 'SurveyVertex', item: 1, vertex: 0 }, surveys));
  });

  it('one resolver serving both kinds does not hand a landing hit back as a survey', () => {
    expect(selectedSurvey({ kind: 'LandingPlace', index: 7, place: 0 }, surveys)).toBeNull();
    expect(selectedSurvey({ kind: 'LandingPlace', index: 4, place: 0 }, surveys)?.index).toBe(4);
  });

  it('a selection naming a survey that is gone yields nothing', () => {
    expect(selectedSurvey({ kind: 'SurveyVertex', item: 9, vertex: 0 }, surveys)).toBeNull();
  });
});

describe('CornerRemovalTest', () => {
  const polygon = (canRemove: boolean | null) =>
    FencePolygon({
      index: 0,
      inclusion: true,
      vertices: [0, 1, 2, 3].map((it) => ({ latitude: it, longitude: it })),
      editable: canRemove === null ? null : EditableShape({ path: 'plan.geoFenceController.polygons.0', midpoints: [], splitInvokable: 'splitPolygonSegment', canRemoveVertex: canRemove }),
    });

  it('the core decides whether a corner can go, counting vertices here would be a second opinion', () => {
    expect(cornerRemovable(polygon(false))).toBe(false);
    expect(cornerRemovable(polygon(true))).toBe(true);
  });

  it('a polygon that is not there offers nothing', () => {
    expect(cornerRemovable(null)).toBe(false);
  });

  it('a shape the core did not answer for hides the removal rather than guessing', () => {
    expect(cornerRemovable(polygon(null))).toBe(false);
  });
});
