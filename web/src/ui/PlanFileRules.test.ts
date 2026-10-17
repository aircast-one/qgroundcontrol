import { describe, expect, it } from 'vitest';
import { KML_EXTENSION, PLAN_EXTENSION, mainBoundaryName, saveBlockedReason, withExtension } from './PlanFileRules';

describe('PlanFileRulesTest', () => {
  it('a reason the core declines to give does not reach the screen as the word null', () => {
    expect(
      saveBlockedReason({ readiness: { ready: false, reason: null } }),
      "optString renders a JSON null as the four letters n-u-l-l on Android, which is not blank, so ifBlank would not fire and the operator would be told 'null'. The core is moving these sentences to null-when-absent, which makes that reachable",
    ).toBe('The plan could not be checked for saving.');
    expect(saveBlockedReason({ readiness: { ready: false, reason: '' } })).toBe('The plan could not be checked for saving.');
    expect(saveBlockedReason({ readiness: { ready: false, reason: 'This plan has no items.' } })).toBe('This plan has no items.');
  });

  it('the shp of a picked shapefile set is the boundary to read', () => {
    expect(mainBoundaryName(['area.prj', 'area.SHP', 'area.dbf'])).toBe('area.SHP');
    expect(mainBoundaryName(['route.kml'])).toBe('route.kml');
    expect(mainBoundaryName([])).toBeNull();
  });

  it('a saved name without the plan suffix gets one, as QGCFileDialog appends it', () => {
    expect(withExtension('survey', PLAN_EXTENSION)).toBe('survey.plan');
    expect(withExtension('survey.PLAN', PLAN_EXTENSION)).toBe('survey.PLAN');
    expect(withExtension('survey.txt', PLAN_EXTENSION)).toBe('survey.txt.plan');
    expect(withExtension('area', KML_EXTENSION)).toBe('area.kml');
  });
});
