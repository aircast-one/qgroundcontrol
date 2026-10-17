import { type DependencyList, useCallback, useEffect, useRef, useState } from 'react';

export type Scope = { isActive: () => boolean };

export const useLaunchedEffect = (keys: DependencyList, block: (scope: Scope) => void | Promise<void>): void => {
  const latest = useUpdatedRef(block);
  useEffect(() => {
    const live = { active: true };
    void Promise.resolve(latest.current({ isActive: () => live.active })).catch((error: unknown) => {
      if (live.active) console.error(error);
    });
    return () => {
      live.active = false;
    };
  }, keys);
};

export const useDisposableEffect = (keys: DependencyList, block: () => () => void): void => {
  useEffect(block, keys);
};

export const useUpdatedRef = <T,>(value: T): { readonly current: T } => {
  const ref = useRef(value);
  ref.current = value;
  return ref;
};

export const useProduceState = <T,>(initial: T, keys: DependencyList, producer: (set: (value: T) => void, scope: Scope) => void | Promise<void>): T => {
  const [value, setValue] = useState<T>(initial);
  useLaunchedEffect(keys, (scope) => producer((next) => scope.isActive() && setValue(next), scope));
  return value;
};

type BackEntry = { order: number; enabled: boolean; onBack: { readonly current: () => void } };

const backStack: { entries: readonly BackEntry[]; next: number; listening: boolean } = { entries: [], next: 0, listening: false };

const OPEN_POPUP = '[role="dialog"][data-open], [role="alertdialog"][data-open], [role="menu"][data-open], [role="listbox"][data-open]';

const listenForBack = () => {
  if (backStack.listening) return;
  backStack.listening = true;
  window.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape' || event.defaultPrevented || document.querySelector(OPEN_POPUP)) return;
    const top = backStack.entries.filter((entry) => entry.enabled).reduce<BackEntry | null>((best, entry) => (best === null || entry.order > best.order ? entry : best), null);
    if (top === null) return;
    event.preventDefault();
    top.onBack.current();
  });
};

export const useBackHandler = (enabled: boolean, onBack: () => void): void => {
  const latest = useUpdatedRef(onBack);
  const [order] = useState(() => backStack.next++);
  useEffect(() => {
    listenForBack();
    backStack.entries = [...backStack.entries.filter((entry) => entry.order !== order), { order, enabled, onBack: latest }];
    return () => {
      backStack.entries = backStack.entries.filter((entry) => entry.order !== order);
    };
  }, [order, enabled]);
};

export const useTicker = (periodMs: number, keys: DependencyList, tick: () => void | Promise<void>): void => {
  useLaunchedEffect([periodMs, ...keys], async (scope) => {
    const loop = async (): Promise<void> => {
      if (!scope.isActive()) return;
      await tick();
      await new Promise((resolve) => setTimeout(resolve, periodMs));
      return loop();
    };
    await loop();
  });
};

export const useKeyedState = <T,>(keys: readonly unknown[], initial: () => T): readonly [T, (next: T | ((previous: T) => T)) => void] => {
  const [held, setHeld] = useState(() => ({ keys, value: initial() }));
  const stale = held.keys.length !== keys.length || held.keys.some((key, at) => !Object.is(key, keys[at]));
  const current = stale ? { keys, value: initial() } : held;
  if (stale) setHeld(current);
  const set = useCallback((next: T | ((previous: T) => T)) => setHeld((previous) => ({ keys: previous.keys, value: typeof next === 'function' ? (next as (previous: T) => T)(previous.value) : next })), []);
  return [current.value, set];
};
