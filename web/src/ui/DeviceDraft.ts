import { useEffect, useState } from 'react';

export const SETTLE_MS = 2_000;

export type Fields<T> = Readonly<Record<string, T>>;

export type Draft<T> = { readonly value: T; readonly sent: boolean };

export type Drafts<T> = Fields<Draft<T>>;

const same = (a: unknown, b: unknown): boolean => JSON.stringify(a) === JSON.stringify(b);

export const edited = <T,>(drafts: Drafts<T>, name: string, value: T): Drafts<T> => ({ ...drafts, [name]: { value, sent: false } });

export const accepted = <T,>(drafts: Drafts<T>, sent: Fields<T>): Drafts<T> =>
  Object.fromEntries(Object.entries(drafts).map(([name, draft]) => [name, { ...draft, sent: draft.sent || (name in sent && same(draft.value, sent[name])) }]));

export const reconciled = <T,>(drafts: Drafts<T>, before: Fields<T>, after: Fields<T>): Drafts<T> =>
  Object.fromEntries(Object.entries(drafts).filter(([name, draft]) => same(before[name], after[name]) || !same(draft.value, after[name])));

export const settled = <T,>(drafts: Drafts<T>): Drafts<T> => Object.fromEntries(Object.entries(drafts).filter(([, draft]) => !draft.sent));

export const shownValues = <T,>(reported: Fields<T>, drafts: Drafts<T>): Fields<T> => ({ ...reported, ...Object.fromEntries(Object.entries(drafts).map(([name, draft]) => [name, draft.value])) });

export type DeviceDraft<T> = {
  readonly values: Fields<T>;
  readonly edit: (name: string, value: T) => void;
  readonly accept: (sent: Fields<T>) => void;
  readonly discard: () => void;
};

export const useDeviceDraft = <T,>(reported: Fields<T>): DeviceDraft<T> => {
  const [drafts, setDrafts] = useState<Drafts<T>>({});
  const [seen, setSeen] = useState(reported);
  const [saves, setSaves] = useState(0);
  if (!same(seen, reported)) {
    setSeen(reported);
    setDrafts(reconciled(drafts, seen, reported));
  }
  useEffect(() => {
    if (saves === 0) return undefined;
    const timer = setTimeout(() => setDrafts(settled), SETTLE_MS);
    return () => clearTimeout(timer);
  }, [saves]);
  return {
    values: shownValues(reported, drafts),
    edit: (name, value) => setDrafts((it) => edited(it, name, value)),
    accept: (sent) => {
      setDrafts((it) => accepted(it, sent));
      setSaves((it) => it + 1);
    },
    discard: () => setDrafts({}),
  };
};
