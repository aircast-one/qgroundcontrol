import { useEffect } from 'react';
import { useUpdatedRef } from '@/kotlin/Compose';
import { onMac, popupOpen, typingIn } from '@/kotlin/Keys';

export type PlanAction = 'save' | 'undo' | 'redo' | 'deleteSelected';

export type PlanShortcuts = Readonly<Record<PlanAction, (() => void) | null>>;

export const planShortcut = (key: string, modifier: boolean, shift: boolean, typing: boolean): PlanAction | null => {
  const letter = key.toLowerCase();
  if (modifier && letter === 's') return 'save';
  if (typing) return null;
  if (modifier && letter === 'z') return shift ? 'redo' : 'undo';
  if (modifier && letter === 'y') return 'redo';
  return !modifier && (key === 'Delete' || key === 'Backspace') ? 'deleteSelected' : null;
};

export const shortcutHint = (action: PlanAction, mac: boolean = onMac()): string => {
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

export const usePlanShortcuts = (actions: PlanShortcuts): void => {
  const latest = useUpdatedRef(actions);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || popupOpen()) return;
      const action = planShortcut(event.key, event.metaKey || event.ctrlKey, event.shiftKey, typingIn(event.target));
      if (action === null) return;
      event.preventDefault();
      if (!event.repeat) latest.current[action]?.();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
};
