const storage = (): Storage | null => {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
};

const key = (store: string, name: string) => `aircast.${store}.${name}`;

export type SharedPreferences = {
  getString(name: string, fallback: string | null): string | null;
  getBoolean(name: string, fallback: boolean): boolean;
  getInt(name: string, fallback: number): number;
  getFloat(name: string, fallback: number): number;
  getStringSet(name: string, fallback: Set<string>): Set<string>;
  contains(name: string): boolean;
  putString(name: string, value: string | null): void;
  putBoolean(name: string, value: boolean): void;
  putInt(name: string, value: number): void;
  putFloat(name: string, value: number): void;
  putStringSet(name: string, value: Set<string>): void;
  remove(name: string): void;
};

export const getSharedPreferences = (store: string): SharedPreferences => {
  const read = (name: string): string | null => {
    try {
      return storage()?.getItem(key(store, name)) ?? null;
    } catch {
      return null;
    }
  };
  const write = (name: string, value: string | null) => {
    try {
      if (value === null) storage()?.removeItem(key(store, name));
      else storage()?.setItem(key(store, name), value);
    } catch {
      return;
    }
  };
  const number = (name: string, fallback: number) => {
    const text = read(name);
    const parsed = text === null ? NaN : Number(text);
    return Number.isNaN(parsed) ? fallback : parsed;
  };
  return {
    getString: (name, fallback) => read(name) ?? fallback,
    getBoolean: (name, fallback) => {
      const text = read(name);
      return text === null ? fallback : text === 'true';
    },
    getInt: (name, fallback) => Math.trunc(number(name, fallback)),
    getFloat: number,
    getStringSet: (name, fallback) => {
      const text = read(name);
      if (text === null) return fallback;
      try {
        const parsed: unknown = JSON.parse(text);
        return Array.isArray(parsed) ? new Set(parsed.map(String)) : fallback;
      } catch {
        return fallback;
      }
    },
    contains: (name) => read(name) !== null,
    putString: write,
    putBoolean: (name, value) => write(name, String(value)),
    putInt: (name, value) => write(name, String(Math.trunc(value))),
    putFloat: (name, value) => write(name, String(value)),
    putStringSet: (name, value) => write(name, JSON.stringify([...value])),
    remove: (name) => write(name, null),
  };
};

export const openDocument = (accept: string[]): Promise<File | null> =>
  new Promise((resolve) => {
    const input = Object.assign(document.createElement('input'), { type: 'file', accept: accept.join(',') });
    input.addEventListener('change', () => resolve(input.files?.[0] ?? null), { once: true });
    input.addEventListener('cancel', () => resolve(null), { once: true });
    input.click();
  });

export const openDocuments = (accept: string[]): Promise<File[]> =>
  new Promise((resolve) => {
    const input = Object.assign(document.createElement('input'), { type: 'file', accept: accept.join(','), multiple: true });
    input.addEventListener('change', () => resolve([...(input.files ?? [])]), { once: true });
    input.addEventListener('cancel', () => resolve([]), { once: true });
    input.click();
  });

export const createDocument = (name: string, mimeType: string, contents: BlobPart): void => {
  const url = URL.createObjectURL(new Blob([contents], { type: mimeType }));
  const anchor = Object.assign(document.createElement('a'), { href: url, download: name });
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
};

export const copyToClipboard = (text: string): Promise<void> => navigator.clipboard.writeText(text);

export const vibrate = (milliseconds: number): void => {
  navigator.vibrate?.(milliseconds);
};

export const openUri = (uri: string): void => {
  window.open(uri, '_blank', 'noopener,noreferrer');
};
