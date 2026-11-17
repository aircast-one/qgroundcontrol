import { useEffect, useState } from 'react';

const POPUP = '[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]';
const FOCUSABLE = 'button, a[href], input, select, textarea, [tabindex]';

const last: { element: HTMLElement | null; tracking: boolean } = { element: null, tracking: false };

const remember = (target: EventTarget | null) => {
  const element = target instanceof Element ? target.closest<HTMLElement>(FOCUSABLE) : null;
  if (element !== null && element.closest(POPUP) === null) last.element = element;
};

export const trackFocus = (): void => {
  if (last.tracking) return;
  last.tracking = true;
  window.addEventListener('focusin', (event) => remember(event.target), true);
  window.addEventListener('pointerdown', (event) => remember(event.target), true);
};

const focusLost = (): boolean => document.activeElement === null || document.activeElement === document.body;

export const useFocusReturn = (): void => {
  const [opener] = useState(() => last.element);
  useEffect(
    () => () => {
      if (opener !== null && opener.isConnected && focusLost()) opener.focus({ preventScroll: true });
    },
    [],
  );
};
