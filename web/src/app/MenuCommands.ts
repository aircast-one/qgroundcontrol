import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef } from 'react';
import { useUpdatedRef } from '@/kotlin/Compose';

export const MENU_EVENT = 'menu';

export type MenuCommand = 'about' | 'settings' | 'new-plan' | 'open-plan' | 'save-plan' | 'save-plan-as' | 'fly' | 'plan' | 'analyze' | 'setup';

export type KeyPress = { code: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean };

const NATIVE_KEYS: Readonly<Record<string, MenuCommand>> = {
  Comma: 'settings',
  KeyN: 'new-plan',
  KeyO: 'open-plan',
  KeyS: 'save-plan',
  Digit1: 'fly',
  Digit2: 'plan',
  Digit3: 'analyze',
  Digit4: 'setup',
};

const BROWSER_KEYS: Readonly<Record<string, MenuCommand>> = {
  Comma: 'settings',
  Digit1: 'fly',
  Digit2: 'plan',
  Digit3: 'analyze',
  Digit4: 'setup',
};

export const nativeMenuOwns = (press: KeyPress, mac: boolean): boolean =>
  (mac ? press.metaKey && !press.ctrlKey : press.ctrlKey && !press.metaKey) && !press.altKey && NATIVE_KEYS[press.code] !== undefined && (!press.shiftKey || press.code === 'KeyS');

export const nativeCommand = (press: KeyPress, mac: boolean): MenuCommand | null =>
  nativeMenuOwns(press, mac) ? (press.code === 'KeyS' && press.shiftKey ? 'save-plan-as' : (NATIVE_KEYS[press.code] ?? null)) : null;

export type KeyedCommand = { command: MenuCommand; at: number };

const MENU_ECHO_MS = 500;

export const echoOfKey = (keyed: KeyedCommand | null, command: MenuCommand, now: number): boolean =>
  keyed !== null && keyed.command === command && now - keyed.at < MENU_ECHO_MS;

export const browserShortcut = (press: KeyPress, typing: boolean): MenuCommand | null =>
  press.altKey && !press.metaKey && !press.ctrlKey && !press.shiftKey && !typing ? (BROWSER_KEYS[press.code] ?? null) : null;

const typingIn = (target: EventTarget | null): boolean => target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));

export const onMac = (): boolean => /Mac/.test(navigator.platform);

export const useMenuCommands = (run: (command: MenuCommand) => void): void => {
  const latest = useUpdatedRef(run);
  const keyed = useRef<KeyedCommand | null>(null);
  useEffect(() => {
    if (__DESKTOP__) {
      const mac = onMac();
      const hold = (event: KeyboardEvent) => {
        if (!nativeMenuOwns(event, mac)) return;
        event.stopImmediatePropagation();
        const command = mac ? null : nativeCommand(event, mac);
        if (command === null) return;
        event.preventDefault();
        keyed.current = { command, at: performance.now() };
        latest.current(command);
      };
      window.addEventListener('keydown', hold, true);
      const stop = listen<MenuCommand>(MENU_EVENT, ({ payload }) => {
        if (!echoOfKey(keyed.current, payload, performance.now())) latest.current(payload);
      });
      return () => {
        window.removeEventListener('keydown', hold, true);
        void stop.then((unlisten) => unlisten());
      };
    }
    const press = (event: KeyboardEvent) => {
      const command = browserShortcut(event, typingIn(event.target));
      if (command === null || event.defaultPrevented) return;
      event.preventDefault();
      latest.current(command);
    };
    window.addEventListener('keydown', press);
    return () => window.removeEventListener('keydown', press);
  }, []);
};
