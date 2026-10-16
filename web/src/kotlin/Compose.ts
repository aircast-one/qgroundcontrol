import { type DependencyList, useEffect, useRef, useState } from 'react';

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

export const useBackHandler = (enabled: boolean, onBack: () => void): void => {
  const latest = useUpdatedRef(onBack);
  useEffect(() => {
    if (!enabled) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || event.defaultPrevented) return;
      event.preventDefault();
      latest.current();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [enabled]);
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
