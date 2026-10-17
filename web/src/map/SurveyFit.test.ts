import { describe, expect, it } from 'vitest';
import { KIND_SURVEY } from './PlanBridge';
import { SHAPE_AREA, Survey, fitSurveyArea, insetRing } from './SurveyBridge';

const view = [
  { latitude: 42.0, longitude: 44.0 },
  { latitude: 42.0, longitude: 45.0 },
  { latitude: 41.0, longitude: 45.0 },
  { latitude: 41.0, longitude: 44.0 },
];

const survey = (corners: number) =>
  Survey({
    index: 2,
    area: Array.from({ length: corners }, () => ({ latitude: 41.5, longitude: 44.5 })),
    transects: [],
    cameraShots: 0,
    kind: KIND_SURVEY,
    shape: SHAPE_AREA,
    property: 'surveyAreaPolygon',
  });

const mean = (values: readonly number[]) => values.reduce((sum, it) => sum + it, 0) / values.length;

describe('SurveyFitTest', () => {
  it('the fitted area sits inside what the operator can see, not flush against the edge', () => {
    const inset = insetRing(view, 0.8);
    expect(inset.length).toBe(4);
    expect(
      inset.every((it) => it.latitude < 42.0),
      'north edge must come in from the top',
    ).toBe(true);
    expect(
      inset.every((it) => it.latitude > 41.0),
      'south edge must come up from the bottom',
    ).toBe(true);
    expect(inset[0]?.latitude).toBeCloseTo(41.9, 9);
    expect(inset[0]?.longitude).toBeCloseTo(44.1, 9);
  });

  it('the inset keeps the centre where it was, so fitting does not walk the survey', () => {
    const inset = insetRing(view, 0.8);
    expect(mean(inset.map((it) => it.latitude))).toBeCloseTo(41.5, 9);
    expect(mean(inset.map((it) => it.longitude))).toBeCloseTo(44.5, 9);
  });

  it('a shape with fewer corners than the view is refused rather than half written', async () => {
    expect(await fitSurveyArea(survey(3), insetRing(view, 0.8))).toBe(false);
    expect(await fitSurveyArea(survey(4), [])).toBe(false);
  });

  it('nothing to inset yields nothing rather than a degenerate ring', () => {
    expect(insetRing([], 0.8)).toEqual([]);
    expect(insetRing(view.slice(0, 2), 0.8)).toEqual([]);
  });
});
