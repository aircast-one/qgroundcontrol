import { useSyncExternalStore } from 'react';

type Listener = () => void;

export class MutableState<T> {
  private current: T;
  private readonly listeners = new Set<Listener>();

  constructor(initial: T) {
    this.current = initial;
  }

  get value(): T {
    return this.current;
  }

  set value(next: T) {
    if (Object.is(next, this.current)) return;
    this.current = next;
    this.listeners.forEach((listener) => listener());
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  readonly snapshot = (): T => this.current;
}

export const mutableStateOf = <T,>(initial: T): MutableState<T> => new MutableState(initial);

export class MutableStateList<T> extends MutableState<readonly T[]> {
  add(item: T): void {
    this.value = [...this.value, item];
  }

  remove(item: T): void {
    const at = this.value.indexOf(item);
    if (at >= 0) this.value = [...this.value.slice(0, at), ...this.value.slice(at + 1)];
  }

  clear(): void {
    this.value = [];
  }

  contains(item: T): boolean {
    return this.value.includes(item);
  }
}

export const mutableStateListOf = <T,>(...items: T[]): MutableStateList<T> => new MutableStateList<T>(items);

export class MutableStateMap<K, V> extends MutableState<ReadonlyMap<K, V>> {
  get(key: K): V | undefined {
    return this.value.get(key);
  }

  set(key: K, item: V): void {
    this.value = new Map(this.value).set(key, item);
  }

  remove(key: K): void {
    if (!this.value.has(key)) return;
    const next = new Map(this.value);
    next.delete(key);
    this.value = next;
  }

  clear(): void {
    this.value = new Map();
  }
}

export const mutableStateMapOf = <K, V>(): MutableStateMap<K, V> => new MutableStateMap<K, V>(new Map());

export const useValue = <T,>(state: MutableState<T>): T => useSyncExternalStore(state.subscribe, state.snapshot);
