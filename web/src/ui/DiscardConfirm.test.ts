import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { planContainsItems, planIsDirty, planIsSyncing, planSyncProgress } from './PlanFileRules';

describe('DiscardConfirmTest', () => {
  it('dirty and syncing come off the plan view the tab already reads', () => {
    const idle = parseJson('{"kind":"object","class":"PlanStatus","dirty":false,"sync":{"state":"ready"}}');
    const busy = parseJson('{"kind":"object","class":"PlanStatus","dirty":true,"sync":{"state":"busy"}}');
    expect(planIsDirty(idle)).toBe(false);
    expect(planIsDirty(busy)).toBe(true);
    expect(planIsSyncing(idle)).toBe(false);
    expect(planIsSyncing(busy), 'plan.rs serves busy for a sync in flight; the head compared against a syncing it never sends').toBe(true);
    expect(planIsSyncing(parseJson('{"kind":"object","sync":{"state":"offline"}}'))).toBe(false);
    expect(planContainsItems(parseJson('{"kind":"object","containsItems":true}'))).toBe(true);
    expect(planContainsItems(null)).toBe(false);
  });

  it('no plan view is not a clean plan and not a finished sync', () => {
    expect(planIsDirty(null)).toBe(false);
    expect(planIsSyncing(null)).toBe(false);
    expect(planIsSyncing(parseJson('{"kind":"object","class":"PlanStatus"}'))).toBe(false);
  });

  it('sync progress comes off the plan view, clamped, and reads zero without one', () => {
    expect(planSyncProgress(parseJson('{"sync":{"state":"busy","progress":0.4}}'))).toBeCloseTo(0.4, 6);
    expect(planSyncProgress(parseJson('{"sync":{"progress":3}}'))).toBe(1);
    expect(planSyncProgress(null)).toBe(0);
  });
});
