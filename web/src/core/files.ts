import { del, entries, set } from 'idb-keyval';
import { call } from './client';

const KEPT_PREFIXES: readonly string[] = ['/replay-links/'];

const kept = (path: string): boolean => KEPT_PREFIXES.some((prefix) => path.startsWith(prefix));

const keptPaths = async (): Promise<readonly [string, Uint8Array][]> =>
  (await entries<string, Uint8Array>().catch(() => [])).filter(([path]) => typeof path === 'string' && kept(path));

export const stageFile = async (path: string, file: Blob): Promise<boolean> => {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const staged = bytes.length > 0 && (await call<boolean>('vfsWrite', path, bytes));
  if (staged && kept(path)) await set(path, bytes).catch(() => undefined);
  return staged;
};

export const stagedFile = async (path: string, name: string = path.slice(path.lastIndexOf('/') + 1)): Promise<File | null> => {
  const bytes = await call<Uint8Array | undefined>('vfsRead', path);
  return bytes === undefined ? null : new File([bytes as Uint8Array<ArrayBuffer>], name);
};

export const unstage = async (path: string): Promise<boolean> => {
  if (kept(path)) await del(path).catch(() => undefined);
  return call<boolean>('vfsRemove', path);
};

export const unstageAll = async (prefix: string, keep?: string): Promise<void> => {
  const dropped = (await keptPaths()).filter(([path]) => path.startsWith(prefix) && path !== keep);
  await Promise.all(dropped.map(([path]) => del(path).catch(() => undefined)));
  return call<void>('vfsRemovePrefixed', prefix, keep);
};

export type StagedEntry = { name: string; size: number; dir: boolean; modified?: number };

export const listStaged = async (dir: string): Promise<readonly StagedEntry[]> => JSON.parse((await call<string | undefined>('vfsList', dir)) ?? '[]') as StagedEntry[];

export const restoreKeptFiles = async (): Promise<void> => {
  await Promise.all((await keptPaths()).map(([path, bytes]) => call<boolean>('vfsWrite', path, bytes)));
};
