import { describe, expect, it } from 'vitest';
import { planShortcut, shortcutHint } from './PlanShortcuts';

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
