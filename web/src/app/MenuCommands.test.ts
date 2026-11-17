import { describe, expect, it } from 'vitest';
import { browserShortcut, echoOfKey, nativeCommand, nativeMenuOwns } from './MenuCommands';

const key = (code: string, fields: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }> = {}) => ({ code, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...fields });

describe('MenuCommandsTest', () => {
  it('the page leaves to the native menu exactly the keys its accelerators hold, so none is handled twice', () => {
    expect(nativeMenuOwns(key('KeyS', { metaKey: true }), true)).toBe(true);
    expect(nativeMenuOwns(key('KeyS', { metaKey: true, shiftKey: true }), true)).toBe(true);
    expect(nativeMenuOwns(key('Comma', { metaKey: true }), true)).toBe(true);
    expect(nativeMenuOwns(key('Digit2', { ctrlKey: true }), false)).toBe(true);
    expect(nativeMenuOwns(key('KeyS', { ctrlKey: true }), true), 'Ctrl+S on a Mac is not the menu’s').toBe(false);
    expect(nativeMenuOwns(key('KeyZ', { metaKey: true }), true), 'undo stays with the page and the text fields').toBe(false);
    expect(nativeMenuOwns(key('Digit2', { ctrlKey: true, shiftKey: true }), false)).toBe(false);
    expect(nativeMenuOwns(key('KeyN', { metaKey: true, altKey: true }), true)).toBe(false);
  });

  it('the browser uses Alt with a digit or comma, which no browser takes, and never while typing', () => {
    expect(browserShortcut(key('Digit1', { altKey: true }), false)).toBe('fly');
    expect(browserShortcut(key('Digit4', { altKey: true }), false)).toBe('setup');
    expect(browserShortcut(key('Comma', { altKey: true }), false)).toBe('settings');
    expect(browserShortcut(key('Digit1', { altKey: true }), true)).toBeNull();
    expect(browserShortcut(key('Digit1', { metaKey: true }), false), 'Cmd+1 switches browser tabs').toBeNull();
    expect(browserShortcut(key('KeyS', { altKey: true }), false)).toBeNull();
  });

  it('on Windows the page runs the menu keys itself, since WebView2 keeps them from the window’s accelerators', () => {
    expect(nativeCommand(key('KeyS', { ctrlKey: true }), false)).toBe('save-plan');
    expect(nativeCommand(key('KeyS', { ctrlKey: true, shiftKey: true }), false)).toBe('save-plan-as');
    expect(nativeCommand(key('Comma', { ctrlKey: true }), false)).toBe('settings');
    expect(nativeCommand(key('Digit3', { ctrlKey: true }), false)).toBe('analyze');
    expect(nativeCommand(key('KeyZ', { ctrlKey: true }), false)).toBeNull();
    expect(nativeCommand(key('KeyS', { metaKey: true }), false)).toBeNull();
  });

  it('a menu event right after the same key is its echo and runs nothing, while a later click on the menu still runs', () => {
    const keyed = { command: 'save-plan' as const, at: 1_000 };
    expect(echoOfKey(keyed, 'save-plan', 1_200)).toBe(true);
    expect(echoOfKey(keyed, 'save-plan', 1_600)).toBe(false);
    expect(echoOfKey(keyed, 'open-plan', 1_100)).toBe(false);
    expect(echoOfKey(null, 'save-plan', 1_100)).toBe(false);
  });
});
