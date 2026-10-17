import { call } from './client';

export const stageFile = async (path: string, file: Blob): Promise<boolean> => {
  const bytes = new Uint8Array(await file.arrayBuffer());
  return bytes.length > 0 && (await call<boolean>('vfsWrite', path, bytes));
};

export const stagedFile = async (path: string, name: string = path.slice(path.lastIndexOf('/') + 1)): Promise<File | null> => {
  const bytes = await call<Uint8Array | undefined>('vfsRead', path);
  return bytes === undefined ? null : new File([bytes as Uint8Array<ArrayBuffer>], name);
};

export const unstage = (path: string): Promise<boolean> => call<boolean>('vfsRemove', path);

export const unstageAll = (prefix: string): Promise<void> => call<void>('vfsRemovePrefixed', prefix);
