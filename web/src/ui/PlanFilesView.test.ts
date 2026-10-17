import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { currentPlanPath, patternNames } from './PlanFiles';

describe('PlanFilesViewTest', () => {
  it('patterns are offered by the canonical name the insert takes', () => {
    const view = parseJson('{"patterns":[{"name":"Survey","title":"Survey"},{"name":"Corridor Scan","title":"Korridor-Scan"},{"name":"","title":"x"}]}');
    expect(patternNames(view)).toEqual(['Survey', 'Corridor Scan']);
    expect(patternNames(null)).toEqual([]);
  });

  it('the plan file is the whole path, and none is null', () => {
    expect(currentPlanPath(parseJson('{"filePath":"/plans/ridge.plan"}'))).toBe('/plans/ridge.plan');
    expect(currentPlanPath(parseJson('{"filePath":null}'))).toBeNull();
    expect(currentPlanPath(null)).toBeNull();
  });
});
