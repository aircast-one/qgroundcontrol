// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { call } from '@/core/client';
import type { MapHit } from './MissionEditing';
import { MAV_CMD_NAV_RETURN_TO_LAUNCH, MissionItem, reorderOf } from './PlanBridge';
import { deletableSelection, itemMoves, usePlanEditing, usePlanFeedback } from './PlanEditing';
import { useRemembered } from './Remembered';

vi.mock('@/core/client', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/core/client')>()), call: vi.fn() }));

const called = vi.mocked(call);

const item = (index: number, command = 'Waypoint') => MissionItem({ index, sequence: index, stop: index, latitude: 1, longitude: 1, command, selected: false });

const items = [item(0, 'Mission Start'), item(1), item(2), item(3)];

const useEditing = (hit: MapHit | null) => {
  const feedback = usePlanFeedback();
  const selected = useRemembered<MapHit | null>(hit);
  return { feedback, selected, editing: usePlanEditing({ feedback, selected, allItems: items, rallyCount: 2 }) };
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

describe('an item offers only the moves the core makes', () => {
  const mission = [
    MissionItem({ ...item(0, 'Mission Start'), kind: 'settings' }),
    MissionItem({ ...item(1, 'Takeoff'), kind: 'takeoff' }),
    MissionItem({ ...item(2), kind: 'waypoint' }),
    MissionItem({ ...item(3), kind: 'waypoint' }),
    MissionItem({ ...item(4), kind: 'waypoint' }),
    MissionItem({ ...item(5, 'Land'), kind: 'land' }),
  ];

  it('the takeoff and the landing keep their places, and nothing trades places with them', () => {
    expect([1, 2, 3, 4, 5].map((it) => itemMoves(mission, it))).toEqual([
      { earlier: false, later: false },
      { earlier: false, later: true },
      { earlier: true, later: true },
      { earlier: true, later: false },
      { earlier: false, later: false },
    ]);
  });

  it('a return keeps its place, and the ends of the mission go no further', () => {
    const route = [...mission.slice(0, 5), MissionItem({ ...item(5, 'Return'), kind: 'command', commandId: MAV_CMD_NAV_RETURN_TO_LAUNCH })];
    expect([itemMoves(route, 4), itemMoves(route, 5), itemMoves(items, 1), itemMoves(items, 3), itemMoves(items, 9)]).toEqual([
      { earlier: true, later: false },
      { earlier: false, later: false },
      { earlier: false, later: true },
      { earlier: true, later: false },
      { earlier: false, later: false },
    ]);
  });

  it("a move lands where the core says, and a refusal is the core's sentence", () => {
    expect(reorderOf({ kind: 'object', ok: true, index: 4 })).toEqual({ refusal: null, index: 4 });
    expect(reorderOf({ kind: 'object', ok: false, reason: 'The takeoff, landing and return keep their places in the mission.' })).toEqual({ refusal: 'The takeoff, landing and return keep their places in the mission.' });
    expect(reorderOf(null)).toEqual({ refusal: 'The item could not be moved.' });
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

  it('a moved item stays selected at the index the core returns, and a refusal is shown', async () => {
    answered('{"ok":true,"index":7}');
    const view = renderHook(() => useEditing({ kind: 'Waypoint', index: 3 }));
    await act(async () => view.result.current.editing.moveItem(3, false));
    expect(called).toHaveBeenLastCalledWith('invoke', 'core.plan.moveItem', '[3,-1]');
    expect(view.result.current.selected.value).toEqual({ kind: 'Waypoint', index: 7 });
    answered('{"ok":false,"reason":"The takeoff, landing and return keep their places in the mission."}');
    await act(async () => view.result.current.editing.moveItem(2, false));
    expect(view.result.current.feedback.busy.value).toBe('The takeoff, landing and return keep their places in the mission.');
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
