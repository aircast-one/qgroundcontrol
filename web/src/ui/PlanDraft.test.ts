import { del, get, set } from 'idb-keyval';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Qgc } from '@/bridge/Qgc';
import { PLAN_DRAFT_KEY, keepPlanDraft, restorePlanDraft } from './PlanDraft';

vi.mock('idb-keyval', () => ({ get: vi.fn(), set: vi.fn(() => Promise.resolve()), del: vi.fn(() => Promise.resolve()) }));

afterEach(() => vi.clearAllMocks());

describe('PlanDraftTest', () => {
  it('a reload hands the plan the browser kept back to the core, and the core decides whether it replaces anything', async () => {
    vi.mocked(get).mockResolvedValue('{"fileType":"Plan"}');
    const restored = vi.spyOn(Qgc, 'invokeResult').mockResolvedValue(true);
    expect(await restorePlanDraft()).toBe(true);
    expect(restored).toHaveBeenCalledWith('core.plan.restoreDraft', '{"fileType":"Plan"}');
    restored.mockRestore();
  });

  it('every change to the plan is kept, and an emptied plan forgets the old one', async () => {
    await keepPlanDraft('{"fileType":"Plan"}');
    expect(set).toHaveBeenCalledWith(PLAN_DRAFT_KEY, '{"fileType":"Plan"}');
    await keepPlanDraft('');
    expect(del).toHaveBeenCalledWith(PLAN_DRAFT_KEY);
  });
});
