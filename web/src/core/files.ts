import { invoke } from '@tauri-apps/api/core';
import { del, entries, set } from 'idb-keyval';
import { call } from './client';

const desktop: boolean = import.meta.env.DESKTOP === true;

const KEPT_PREFIXES: readonly string[] = ['/replay-links/'];

const staging = { root: '' };

const kept = (path: string): boolean => KEPT_PREFIXES.some((prefix) => path.startsWith(prefix));

const keptPaths = async (): Promise<readonly [string, Uint8Array][]> =>
  (await entries<string, Uint8Array>().catch(() => [])).filter(([path]) => typeof path === 'string' && kept(path));

export const stagingPath = (path: string, root: string = staging.root): string => (root !== '' && path.startsWith(root) ? path.slice(root.length) : path);

export const stagedLocation = (path: string, root: string = staging.root): string => (desktop ? `${root}${stagingPath(path, root)}` : path);

const bytesOf = async (file: Blob): Promise<Uint8Array> => new Uint8Array(await file.arrayBuffer());

export const stageFile = async (path: string, file: Blob): Promise<boolean> => {
  const bytes = await bytesOf(file);
  if (desktop) return bytes.length > 0 && invoke('staged_write', bytes, { headers: { path: encodeURIComponent(stagingPath(path)) } }).then(() => true);
  const staged = bytes.length > 0 && (await call<boolean>('vfsWrite', path, bytes));
  if (staged && kept(path)) await set(path, bytes).catch(() => undefined);
  return staged;
};

const fileOf = (bytes: Uint8Array | ArrayBuffer | undefined, name: string): File | null => (bytes === undefined ? null : new File([bytes as Uint8Array<ArrayBuffer>], name));

export const stagedFile = async (path: string, name: string = path.slice(path.lastIndexOf('/') + 1)): Promise<File | null> =>
  fileOf(desktop ? await invoke<ArrayBuffer>('staged_read', { path: stagingPath(path) }) : await call<Uint8Array | undefined>('vfsRead', path), name);

export const savedFile = async (path: string): Promise<File | null> =>
  desktop ? fileOf(await invoke<ArrayBuffer>('saved_read', { path }).catch(() => undefined), path.slice(Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\')) + 1)) : stagedFile(path);

export const unstage = async (path: string): Promise<boolean> => {
  if (desktop) return invoke<boolean>('staged_remove', { path: stagingPath(path) });
  if (kept(path)) await del(path).catch(() => undefined);
  return call<boolean>('vfsRemove', path);
};

export const unstageAll = async (prefix: string, keep?: string): Promise<void> => {
  if (desktop) return invoke('staged_remove_prefixed', { prefix: stagingPath(prefix), keep: keep === undefined ? null : stagingPath(keep) });
  const dropped = (await keptPaths()).filter(([path]) => path.startsWith(prefix) && path !== keep);
  await Promise.all(dropped.map(([path]) => del(path).catch(() => undefined)));
  return call<void>('vfsRemovePrefixed', prefix, keep);
};

export type StagedEntry = { name: string; size: number; dir: boolean; modified?: number };

export const listStaged = async (dir: string): Promise<readonly StagedEntry[]> =>
  desktop ? invoke<StagedEntry[]>('staged_list', { dir: stagingPath(dir) }) : (JSON.parse((await call<string | undefined>('vfsList', dir)) ?? '[]') as StagedEntry[]);

export const prepareStaging = async (): Promise<void> => {
  if (desktop) {
    staging.root = await invoke<string>('staged_root');
    return;
  }
  await Promise.all((await keptPaths()).map(([path, bytes]) => call<boolean>('vfsWrite', path, bytes)));
};
