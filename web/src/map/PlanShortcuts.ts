import { useEffect } from 'react';
import { useUpdatedRef } from '@/kotlin/Compose';

export type PlanAction = 'save' | 'undo' | 'redo' | 'deleteSelected';

const OPEN_POPUP = '[role="dialog"][data-open], [role="alertdialog"][data-open], [role="menu"][data-open], [role="listbox"][data-open]';

const registry: { actions: Readonly<Partial<Record<PlanAction, () => void>>> } = { actions: {} };

export const PlanActions = {
  available: (action: PlanAction): boolean => registry.actions[action] !== undefined,
  run: (action: PlanAction): boolean => {
    const run = registry.actions[action];
    run?.();
    return run !== undefined;
  },
};

export const usePlanAction = (action: PlanAction, run: () => void): void => {
  const latest = useUpdatedRef(run);
  useEffect(() => {
    const call = () => latest.current();
    registry.actions = { ...registry.actions, [action]: call };
    return () => {
      if (registry.actions[action] === call) registry.actions = Object.fromEntries(Object.entries(registry.actions).filter(([name]) => name !== action));
    };
  }, [action]);
};

export const planShortcut = (key: string, modifier: boolean, shift: boolean, typing: boolean): PlanAction | null => {
  const letter = key.toLowerCase();
  if (modifier && letter === 's') return 'save';
  if (typing) return null;
  if (modifier && letter === 'z') return shift ? 'redo' : 'undo';
  if (modifier && letter === 'y') return 'redo';
  return !modifier && (key === 'Delete' || key === 'Backspace') ? 'deleteSelected' : null;
};

export const typingIn = (target: EventTarget | null): boolean =>
  target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));

const apple = (): boolean => typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

export const shortcutHint = (action: PlanAction, mac: boolean = apple()): string => {
  const modifier = mac ? '⌘' : 'Ctrl+';
  switch (action) {
    case 'save':
      return `${modifier}S`;
    case 'undo':
      return `${modifier}Z`;
    case 'redo':
      return mac ? '⇧⌘Z' : 'Ctrl+Shift+Z';
    case 'deleteSelected':
      return mac ? '⌫' : 'Del';
  }
};

export const usePlanShortcuts = (): void =>
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || document.querySelector(OPEN_POPUP) !== null) return;
      const action = planShortcut(event.key, event.metaKey || event.ctrlKey, event.shiftKey, typingIn(event.target));
      if (action === null || !PlanActions.available(action)) return;
      event.preventDefault();
      PlanActions.run(action);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
