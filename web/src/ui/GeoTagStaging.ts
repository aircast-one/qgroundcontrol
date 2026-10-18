import { listStaged, stageFile, stagedFile, unstageAll } from '@/core/files';
import { createDocument } from '@/kotlin/Platform';
import { DEFAULT_GEOTAG_OUTPUT } from '@/ui/GeoTagRun';
import { isGeoTagImage } from '@/ui/GeoTagView';

const GEOTAG_CACHE = 'geotag';
const MIME_TYPE_DIR = 'inode/directory';
const OCTET_STREAM = 'application/octet-stream';
const LOG_EXTENSIONS: ReadonlySet<string> = new Set(['ulg', 'bin']);
const IMAGE_MIMES: Readonly<Record<string, string>> = { jpg: 'image/jpeg', jpeg: 'image/jpeg', tif: 'image/tiff', tiff: 'image/tiff', dng: 'image/x-adobe-dng' };

export type TreeEntry = { documentId: string; name: string; mime: string };

export type GeoTagTree = { name: string; handle: FileSystemDirectoryHandle | null; files: readonly File[] };

export type StagedDir = { absolutePath: string };

export type DownloadedLog = { name: string; absolutePath: string; lastModified: number };

type DirectoryPicker = (options: { mode: 'readwrite' }) => Promise<FileSystemDirectoryHandle>;

const collect = async <T,>(items: AsyncIterator<T>, seen: readonly T[] = []): Promise<readonly T[]> => {
  const next = await items.next();
  return next.done === true ? seen : collect(items, [...seen, next.value]);
};

const extension = (name: string): string => {
  const dot = name.lastIndexOf('.');
  return dot < 0 ? '' : name.slice(dot + 1).toLowerCase();
};

const imageMime = (name: string): string => IMAGE_MIMES[extension(name)] ?? OCTET_STREAM;

const KEEP = '.keep';

const stagingDir = async (name: string): Promise<StagedDir> => {
  const absolutePath = `/${GEOTAG_CACHE}/${name}`;
  await unstageAll(`${absolutePath}/`);
  await stageFile(`${absolutePath}/${KEEP}`, new Blob(['.']));
  return { absolutePath };
};

export const taggedOutputDir = (): Promise<StagedDir> => stagingDir('tagged');

const directoryPicker = (): DirectoryPicker | null => {
  const picker: unknown = Reflect.get(window, 'showDirectoryPicker');
  return typeof picker === 'function' ? (options) => (picker as DirectoryPicker).call(window, options) : null;
};

const pickedFiles = (): Promise<readonly File[] | null> =>
  new Promise((resolve) => {
    const input = Object.assign(document.createElement('input'), { type: 'file', webkitdirectory: true });
    input.addEventListener('change', () => resolve([...(input.files ?? [])]), { once: true });
    input.addEventListener('cancel', () => resolve(null), { once: true });
    input.click();
  });

export const openDocumentTree = async (): Promise<GeoTagTree | null> => {
  const picker = directoryPicker();
  if (picker !== null) {
    try {
      const handle = await picker({ mode: 'readwrite' });
      return { name: handle.name, handle, files: [] };
    } catch {
      return null;
    }
  }
  const files = await pickedFiles();
  return files === null ? null : { name: files[0]?.webkitRelativePath.split('/')[0] ?? '', handle: null, files };
};

const directoryChildren = async (folder: FileSystemDirectoryHandle): Promise<readonly TreeEntry[]> =>
  (await collect(folder.values())).map((it) => ({ documentId: it.name, name: it.name, mime: it.kind === 'directory' ? MIME_TYPE_DIR : imageMime(it.name) }));

const fileTreeChildren = (files: readonly File[]): readonly TreeEntry[] => {
  const paths = files.map((file) => [file, file.webkitRelativePath.split('/').slice(1)] as const);
  const leaves = paths.filter(([, rest]) => rest.length === 1).map(([file, rest]) => ({ documentId: file.webkitRelativePath, name: rest[0] ?? '', mime: file.type || OCTET_STREAM }));
  const folders = [...new Set(paths.filter(([, rest]) => rest.length > 1).map(([, rest]) => rest[0] ?? ''))].map((name) => ({ documentId: name, name, mime: MIME_TYPE_DIR }));
  return [...leaves, ...folders];
};

export const treeChildren = async (tree: GeoTagTree): Promise<readonly TreeEntry[]> => {
  try {
    return tree.handle !== null ? await directoryChildren(tree.handle) : fileTreeChildren(tree.files);
  } catch {
    return [];
  }
};

const treeFile = async (tree: GeoTagTree, entry: TreeEntry): Promise<File | null> => {
  try {
    return tree.handle !== null ? await (await tree.handle.getFileHandle(entry.documentId)).getFile() : (tree.files.find((it) => it.webkitRelativePath === entry.documentId) ?? null);
  } catch {
    return null;
  }
};

const stageInto = async (from: File | null, folder: StagedDir, name: string): Promise<boolean> => from !== null && stageFile(`${folder.absolutePath}/${name}`, from).catch(() => false);

const copyInto = async (from: File | null, folder: FileSystemDirectoryHandle, name: string): Promise<boolean> => {
  if (from === null) return false;
  try {
    const writable = await (await folder.getFileHandle(name, { create: true })).createWritable();
    await writable.write(from);
    await writable.close();
    return true;
  } catch {
    return false;
  }
};

export const stageLog = async (source: File, name: string): Promise<string | null> => {
  const dir = await stagingDir('log');
  const target = name.trim() === '' ? 'flight.log' : name;
  return (await stageInto(source, dir, target)) ? `${dir.absolutePath}/${target}` : null;
};

export const stageImages = async (tree: GeoTagTree): Promise<readonly [string, number]> => {
  const dir = await stagingDir('images');
  const images = (await treeChildren(tree)).filter((it) => it.mime !== MIME_TYPE_DIR && isGeoTagImage(it.name));
  const copied = await Promise.all(images.map(async (entry) => stageInto(await treeFile(tree, entry), dir, entry.name)));
  return [dir.absolutePath, copied.filter((it) => it).length];
};

export const hasTaggedFolder = async (tree: GeoTagTree): Promise<boolean> =>
  (await treeChildren(tree)).some((it) => it.name === DEFAULT_GEOTAG_OUTPUT && it.mime === MIME_TYPE_DIR);

export const holdsImages = async (tree: GeoTagTree): Promise<boolean> =>
  (await treeChildren(tree)).some((it) => it.mime !== MIME_TYPE_DIR && isGeoTagImage(it.name));

export const treeName = (tree: GeoTagTree): string => tree.name;

export const documentName = (file: File): string => file.name;

export const downloadedLogs = async (logSavePath: string): Promise<readonly DownloadedLog[]> => {
  if (logSavePath.trim() === '') return [];
  const folder = logSavePath.replace(/\/+$/, '');
  return (await listStaged(folder))
    .filter((it) => !it.dir && LOG_EXTENSIONS.has(extension(it.name)))
    .map((it) => ({ name: it.name, absolutePath: `${folder}/${it.name}`, lastModified: it.modified ?? 0 }))
    .toSorted((a, b) => b.lastModified - a.lastModified || b.name.localeCompare(a.name));
};

const folderIn = async (root: FileSystemDirectoryHandle, name: string): Promise<FileSystemDirectoryHandle | null> => {
  try {
    return await root.getDirectoryHandle(name, { create: true });
  } catch {
    return null;
  }
};

const stagedImages = async (staged: StagedDir): Promise<readonly File[]> =>
  (await Promise.all((await listStaged(staged.absolutePath)).filter((it) => !it.dir && isGeoTagImage(it.name)).map((it) => stagedFile(`${staged.absolutePath}/${it.name}`)))).filter((it): it is File => it !== null);

export const publishTagged = async (staged: StagedDir, tree: GeoTagTree, subfolder: string | null): Promise<number> => {
  const files = await stagedImages(staged).catch((): readonly File[] => []);
  const root = tree.handle;
  if (root === null) {
    files.forEach((file) => createDocument(file.name, imageMime(file.name), file));
    return files.length;
  }
  const folder = (subfolder === null ? null : await folderIn(root, subfolder)) ?? root;
  const present = new Set((await directoryChildren(folder).catch((): readonly TreeEntry[] => [])).map((it) => it.name));
  const written = await Promise.all(
    files.map(async (file) => {
      const done = await copyInto(file, folder, file.name);
      if (!done && !present.has(file.name)) await folder.removeEntry(file.name).catch(() => undefined);
      return done;
    }),
  );
  return written.filter((it) => it).length;
};
