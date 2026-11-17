// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { mutableStateOf } from '@/kotlin/State';
import type { PlanRequest } from '@/ui/AppNavigation';
import { type PlanRequestHandlers, usePlanRequests } from './PlanRequests';

describe('a plan request from the desktop menu', () => {
  afterEach(cleanup);

  const handlers = (overrides: Partial<PlanRequestHandlers> = {}) => ({ new: vi.fn(), open: vi.fn(), save: vi.fn(), saveAs: vi.fn(), ...overrides });

  it('runs the same action as the plan menu once the plan has answered, and is then cleared', () => {
    const requests = mutableStateOf<PlanRequest | null>('open');
    const actions = { new: vi.fn(), open: vi.fn(), save: vi.fn(), saveAs: vi.fn() };
    const refused = vi.fn();
    const view = renderHook(({ ready }) => usePlanRequests(requests, ready, actions, refused), { initialProps: { ready: false } });
    expect(actions.open).not.toHaveBeenCalled();
    view.rerender({ ready: true });
    expect(actions.open).toHaveBeenCalledOnce();
    expect(requests.value).toBeNull();
    act(() => {
      requests.value = 'saveAs';
    });
    expect([actions.saveAs.mock.calls.length, refused.mock.calls.length]).toEqual([1, 0]);
  });

  it('says so when the action is not available, rather than dropping it', () => {
    const requests = mutableStateOf<PlanRequest | null>('save');
    const refused = vi.fn();
    renderHook(() => usePlanRequests(requests, true, handlers({ save: null }), refused));
    expect(refused).toHaveBeenCalledWith('This plan cannot be saved right now.');
    act(() => {
      requests.value = 'open';
    });
    act(() => {
      requests.value = 'new';
    });
    expect(refused.mock.calls.length).toBe(1);
  });

  it('names each refused request', () => {
    const refused = vi.fn();
    const requests = mutableStateOf<PlanRequest | null>(null);
    renderHook(() => usePlanRequests(requests, true, { new: null, open: null, save: null, saveAs: null }, refused));
    (['new', 'open', 'saveAs'] as const).map((request) =>
      act(() => {
        requests.value = request;
      }),
    );
    expect(refused.mock.calls).toEqual([['A new plan cannot be started right now.'], ['A plan cannot be opened right now.'], ['This plan cannot be saved right now.']]);
  });
});
