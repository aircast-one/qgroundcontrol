import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { planTemplates, templateChoices } from './PlanTemplates';

describe('PlanTemplatesTest', () => {
  it('templates read from the plan view in the order QGC offers them', () => {
    const read = planTemplates(
      parseJson(
        '{"templates":{"show":true,"enabled":false,"prompt":"Click in map to set position",' +
          '"names":["Survey","Corridor Scan","Structure Scan","No Template"],"homeSet":false,"blank":"No Template"}}',
      ),
    );
    expect(read).toEqual({ enabled: false, names: ['Survey', 'Corridor Scan', 'Structure Scan', 'No Template'], blank: 'No Template' });
    expect(planTemplates(parseJson('{}'))).toBeNull();
  });

  it('the blank mission comes first and stands for a new empty plan', () => {
    const state = { enabled: true, names: ['Survey', 'Corridor Scan', 'No Template'], blank: 'No Template' };
    expect(templateChoices(state)).toEqual([
      [null, 'Blank mission'],
      ['Survey', 'Survey'],
      ['Corridor Scan', 'Corridor scan'],
    ]);
  });

  it('with no answer from the core a blank plan is still offered', () => {
    expect(templateChoices(null)).toEqual([[null, 'Blank mission']]);
  });
});
