// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { call } from '@/core/client';
import type { MapHit } from './MissionEditing';
import { MissionItem } from './PlanBridge';
import { deletableSelection, usePlanEditing, usePlanFeedback } from './PlanEditing';
import { useRemembered } from './Remembered';

vi.mock('@/core/client', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/core/client')>()), call: vi.fn() }));

const called = vi.mocked(call);

const item = (index: number, command = 'Waypoint') => MissionItem({ index, sequence: index, latitude: 1, longitude: 1, command, selected: false });

const items = [item(0, 'Mission Start'), item(1), item(2), item(3)];

const useEditing = (hit: MapHit | null) => {
  const feedback = usePlanFeedback();
  const selected = useRemembered<MapHit | null>(hit);
  return { feedback, editing: usePlanEditing({ feedback, selected, allItems: items, rallyCount: 2 }) };
};

const answered = (reply: string) => called.mockImplementation(() => Promise.resolve(reply as never));

describe('one deletable selection names and removes what can be deleted', () => {
  const remove = { item: vi.fn(), rally: vi.fn(), circle: vi.fn() };

  it('names the waypoint, the rally point and the circle, and never the launch', () => {
    expect(deletableSelection({ kind: 'Waypoint', index: 3 }, items, remove)?.label).toBe('Delete waypoint 3');
    expect(deletableSelection({ kind: 'Waypoint', index: 0 }, items, remove), 'the launch is never deleted').toBeNull();
    expect([deletableSelection({ kind: 'Rally', index: 1 }, items, remove)?.label, deletableSelection({ kind: 'CircleCentre', index: 0 }, items, remove)?.label, deletableSelection(null, items, remove)]).toEqual([
      'Delete rally point',
      'Delete circular fence',
      null,
    ]);
  });

  it('runs the removal that matches the label', () => {
    [deletableSelection({ kind: 'Waypoint', index: 2 }, items, remove), deletableSelection({ kind: 'Rally', index: 1 }, items, remove), deletableSelection({ kind: 'CircleRadius', index: 4 }, items, remove)].map((it) => it?.run());
    expect([remove.item.mock.calls, remove.rally.mock.calls, remove.circle.mock.calls]).toEqual([[[items[2]]], [[1]], [[4]]]);
  });
});

describe('plan editing', () => {
  afterEach(() => {
    cleanup();
    called.mockReset();
  });

  it('a held or double-pressed delete removes one item, and deleting is off until it lands', async () => {
    const reply = new Promise<(text: string) => void>((ready) => called.mockImplementation(() => new Promise((resolve) => ready(resolve as (text: string) => void))));
    const view = renderHook(() => useEditing({ kind: 'Waypoint', index: 2 }));
    const deletion = view.result.current.editing.deletion;
    act(() => {
      deletion?.run();
      deletion?.run();
    });
    expect(called).toHaveBeenCalledOnce();
    expect(view.result.current.editing.removing).toBe(true);
    await act(async () => (await reply)('{"ok":true,"index":2}'));
    expect(view.result.current.editing.removing).toBe(false);
    expect(view.result.current.editing.removals).toBe(1);
  });

  it('undo and redo report a failure the same way', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    answered('{"ok":false}');
    const view = renderHook(() => useEditing(null));
    await act(async () => view.result.current.editing.undo());
    expect(view.result.current.feedback.busy.value).toBe('That did not work');
    expect(called).toHaveBeenLastCalledWith('invoke', 'plan.undo', '[]');
    await act(async () => view.result.current.editing.redo());
    expect(view.result.current.feedback.busy.value).toBe('That did not work');
    expect(called).toHaveBeenLastCalledWith('invoke', 'plan.redo', '[]');
  });
});
