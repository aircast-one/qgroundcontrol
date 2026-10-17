import { describe, expect, it } from 'vitest';
import { isTextMission, loadFailureMessage, openedFileName } from './PlanFiles';

describe('WaypointsOpenTest', () => {
  it('a text mission is recognised by its suffix like loadFromFile and never adopted as the save target', () => {
    expect(isTextMission('mission.waypoints')).toBe(true);
    expect(isTextMission('mission.txt')).toBe(true);
    expect(isTextMission('mission.plan')).toBe(false);
    expect(isTextMission('mission.WAYPOINTS')).toBe(false);
    expect(isTextMission('waypoints')).toBe(false);
  });

  it('the staged copy keeps the picked file name so the suffix survives', () => {
    expect(openedFileName('survey.waypoints')).toBe('survey.waypoints');
    expect(openedFileName('a/b.plan')).toBe('b.plan');
    expect(openedFileName(null)).toBe('opened.plan');
    expect(openedFileName('..')).toBe('opened.plan');
  });

  it('a refused load says nothing itself because the plan controller posts the reason', () => {
    expect(loadFailureMessage(true)).toBeNull();
    expect(loadFailureMessage(false)).toBeNull();
    expect(loadFailureMessage(null)).toBe('The plan could not be loaded. The current plan is unchanged.');
  });
});
