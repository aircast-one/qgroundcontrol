import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { missionSummaryText } from './MissionSummary';

const view = (...rows: (readonly [string, string])[]): Json => ({ rows: rows.map(([label, value]) => ({ label, value })) });

describe('MissionSummaryTest', () => {
  it("the summary reads the core's words rather than formatting its own", () => {
    expect(missionSummaryText(view(['Distance', '4.82 km'], ['Time', '19:37']))).toBe('Distance 4.82 km · Time 19:37');
  });

  it('imperial units come through untouched, because the core already converted them', () => {
    expect(missionSummaryText(view(['Distance', '2.99 miles'], ['Time', '19:37']))).toBe('Distance 2.99 miles · Time 19:37');
  });

  it('a row the core left out is left out here', () => {
    expect(missionSummaryText(view(['Distance', '4.82 km']))).toBe('Distance 4.82 km');
    expect(missionSummaryText(view(['Time', '19:37']))).toBe('Time 19:37');
  });

  it('rows the summary does not show are ignored', () => {
    expect(missionSummaryText(view(['Cruise', '3 km'], ['Distance', '4.82 km'], ['Hover', '1 km']))).toBe('Distance 4.82 km');
  });

  it('no plan is an empty summary, not a stray separator', () => {
    expect(missionSummaryText({ rows: [] })).toBe('');
    expect(missionSummaryText(null)).toBe('');
  });

  it("the furthest point from launch reads as QGC's Max telem, each total labelled like PlanToolBarIndicators", () => {
    expect(missionSummaryText(view(['Distance', '4.82 km'], ['Time', '19:37'], ['Furthest from launch', '1.2 km']))).toBe('Distance 4.82 km · Time 19:37 · Max telem 1.2 km');
  });
});
