// @vitest-environment jsdom
import { cleanup, fireEvent, render, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { type PlanShortcuts, planShortcut, shortcutHint, usePlanShortcuts } from './PlanShortcuts';

describe('PlanShortcutsTest', () => {
  it('the usual editing keys reach the plan', () => {
    expect([planShortcut('z', true, false, false), planShortcut('Z', true, true, false), planShortcut('y', true, false, false), planShortcut('s', true, false, false)]).toEqual(['undo', 'redo', 'redo', 'save']);
    expect([planShortcut('Delete', false, false, false), planShortcut('Backspace', false, false, false)]).toEqual(['deleteSelected', 'deleteSelected']);
  });

  it('typing in a field keeps its own undo and delete, but save still saves', () => {
    expect([planShortcut('z', true, false, true), planShortcut('Backspace', false, false, true), planShortcut('s', true, false, true)]).toEqual([null, null, 'save']);
  });

  it('plain letters and modified deletes do nothing', () => {
    expect([planShortcut('z', false, false, false), planShortcut('Delete', true, false, false), planShortcut('Escape', false, false, false)]).toEqual([null, null, null]);
  });

  it('the menu hints read the way each platform writes them', () => {
    expect([shortcutHint('undo', true), shortcutHint('redo', true), shortcutHint('save', true)]).toEqual(['⌘Z', '⇧⌘Z', '⌘S']);
    expect([shortcutHint('undo', false), shortcutHint('redo', false), shortcutHint('save', false)]).toEqual(['Ctrl+Z', 'Ctrl+Shift+Z', 'Ctrl+S']);
  });
});

describe('the plan keys on the page', () => {
  afterEach(cleanup);

  const listening = (overrides: Partial<PlanShortcuts> = {}) => {
    const actions = { save: vi.fn(), undo: vi.fn(), redo: vi.fn(), deleteSelected: vi.fn() };
    renderHook(() => usePlanShortcuts({ ...actions, ...overrides }));
    return actions;
  };

  it('a held Delete removes once, not once per key repeat', () => {
    const actions = listening();
    fireEvent.keyDown(window, { key: 'Delete' });
    fireEvent.keyDown(window, { key: 'Delete', repeat: true });
    fireEvent.keyDown(window, { key: 'Delete', repeat: true });
    expect(actions.deleteSelected).toHaveBeenCalledOnce();
  });

  it('typing in a field keeps its own delete and undo, but save still saves', () => {
    const actions = listening();
    const field = render(<input aria-label="Altitude" />).getByLabelText('Altitude');
    fireEvent.keyDown(field, { key: 'Backspace' });
    fireEvent.keyDown(field, { key: 'z', ctrlKey: true });
    fireEvent.keyDown(field, { key: 's', ctrlKey: true });
    expect([actions.deleteSelected.mock.calls.length, actions.undo.mock.calls.length, actions.save.mock.calls.length]).toEqual([0, 0, 1]);
  });

  it('an open dialog takes the keys for itself', () => {
    const actions = listening();
    render(<div role="dialog" data-open="" />);
    fireEvent.keyDown(window, { key: 'Delete' });
    fireEvent.keyDown(window, { key: 'z', metaKey: true });
    expect([actions.deleteSelected.mock.calls.length, actions.undo.mock.calls.length]).toEqual([0, 0]);
  });

  it('a key whose action is unavailable does nothing and is still kept from the browser', () => {
    listening({ deleteSelected: null, save: null });
    expect(fireEvent.keyDown(window, { key: 'Delete' }), 'the default was prevented').toBe(false);
    expect(fireEvent.keyDown(window, { key: 's', ctrlKey: true }), 'the browser does not offer to save the page').toBe(false);
  });
});
