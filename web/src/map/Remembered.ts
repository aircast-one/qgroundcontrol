import { useState } from 'react';
import { same } from '@/kotlin/Std';
import { MutableState, useValue } from '@/kotlin/State';

class Remembered<T> extends MutableState<T> {
  override get value(): T {
    return super.value;
  }

  override set value(next: T) {
    if (!same(super.value, next)) super.value = next;
  }
}

export const useRemembered = <T,>(initial: T): MutableState<T> => {
  const [state] = useState(() => new Remembered(initial));
  useValue(state);
  return state;
};

export const firstWhere = <T,>(state: MutableState<T>, predicate: (value: T) => boolean): Promise<T> =>
  new Promise((resolve) => {
    if (predicate(state.value)) {
      resolve(state.value);
      return;
    }
    const unsubscribe = state.subscribe(() => {
      if (!predicate(state.value)) return;
      unsubscribe();
      resolve(state.value);
    });
  });
