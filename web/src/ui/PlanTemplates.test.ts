import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { zoomedOutForTemplates } from '@/map/PlanBounds';
import { ZOOM_IN_FOR_TEMPLATE, newPlanNote, planTemplates, templateChoices } from './PlanTemplates';

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

  it('a template waits until the map is zoomed in to a place, so it never fills a world view', () => {
    expect([zoomedOutForTemplates(null), zoomedOutForTemplates(2.0), zoomedOutForTemplates(14.9), zoomedOutForTemplates(15.0), zoomedOutForTemplates(17.0)]).toEqual([true, true, true, false, false]);
    expect(newPlanNote(true, true)).toBe(`Replaces the plan you are editing. ${ZOOM_IN_FOR_TEMPLATE}`);
    expect(newPlanNote(false, false)).toBe('Templates fill the area you can see on the map.');
  });
});
