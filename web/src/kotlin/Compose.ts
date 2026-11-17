import { type DependencyList, useCallback, useEffect, useRef, useState } from 'react';
import { flushSync } from 'react-dom';
import { OPEN_POPUP } from './Keys';

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

type Handler = { order: number; enabled: boolean; run: { readonly current: () => void } };

type HandlerKind = 'back' | 'escape';

const handlers: { back: readonly Handler[]; escape: readonly Handler[]; next: number; listening: boolean } = { back: [], escape: [], next: 0, listening: false };

const trail = { pushed: 0, travelling: false };

const DEPTH = 'aircastDepth';

const enabledOf = (kind: HandlerKind): readonly Handler[] => handlers[kind].filter((entry) => entry.enabled);

const topOf = (kind: HandlerKind): Handler | null => enabledOf(kind).reduce<Handler | null>((best, entry) => (best === null || entry.order > best.order ? entry : best), null);

export const historyDepth = (state: unknown): number => {
  const depth = typeof state === 'object' && state !== null ? (state as Record<string, unknown>)[DEPTH] : undefined;
  return typeof depth === 'number' && Number.isInteger(depth) && depth > 0 ? depth : 0;
};

const syncHistory = () => {
  if (trail.travelling) return;
  const wanted = enabledOf('back').length;
  if (wanted > trail.pushed) {
    Array.from({ length: wanted - trail.pushed }, (_, at) => history.pushState({ [DEPTH]: trail.pushed + at + 1 }, ''));
    trail.pushed = wanted;
  } else if (wanted < trail.pushed) {
    trail.travelling = true;
    history.go(wanted - trail.pushed);
  }
};

const listen = () => {
  if (handlers.listening) return;
  handlers.listening = true;
  trail.pushed = historyDepth(history.state);
  window.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape' || event.defaultPrevented || document.querySelector(OPEN_POPUP)) return;
    const top = topOf('escape');
    if (top === null) return;
    event.preventDefault();
    top.run.current();
  });
  window.addEventListener('popstate', (event) => {
    const depth = historyDepth(event.state);
    const asked = !trail.travelling && depth < trail.pushed;
    trail.travelling = false;
    trail.pushed = depth;
    const top = asked ? topOf('back') : null;
    if (top !== null) flushSync(() => top.run.current());
    syncHistory();
  });
};

const useHandler = (kind: HandlerKind, enabled: boolean, run: () => void): void => {
  const latest = useUpdatedRef(run);
  const [order] = useState(() => handlers.next++);
  useEffect(() => {
    listen();
    handlers[kind] = [...handlers[kind].filter((entry) => entry.order !== order), { order, enabled, run: latest }];
    if (kind === 'back') syncHistory();
    return () => {
      handlers[kind] = handlers[kind].filter((entry) => entry.order !== order);
      if (kind === 'back') queueMicrotask(syncHistory);
    };
  }, [order, enabled]);
};

export const useBackHandler = (enabled: boolean, onBack: () => void): void => useHandler('back', enabled, onBack);

export const useEscapeHandler = (enabled: boolean, onEscape: () => void): void => useHandler('escape', enabled, onEscape);

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
