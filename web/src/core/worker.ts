import init, * as core from './pkg/groundstation.js';
import { cloudHost } from './cloud';
import { socketHost } from './socket';

const PUMP_MS = 20;

type Scope = {
  postMessage(message: unknown, transfer?: Transferable[]): void;
  onmessage: ((event: MessageEvent<{ id: string; call: keyof typeof calls; args: unknown[] }>) => void) | null;
};

const scope = self as unknown as Scope;
const owned = (bytes: Uint8Array) => [bytes.buffer as ArrayBuffer];

await init();
core.onEvent((path: string, json: string) => scope.postMessage({ event: { path, json } }));
core.setSerialHost({
  open: (id: number, name: string, baud: number, dataBits: number, stopBits: number, parity: number) => {
    scope.postMessage({ serial: { op: 'open', id, name, baud, dataBits, stopBits, parity } });
    return true;
  },
  write: (id: number, bytes: Uint8Array) => {
    scope.postMessage({ serial: { op: 'write', id, bytes } }, owned(bytes));
    return true;
  },
  close: (id: number) => scope.postMessage({ serial: { op: 'close', id } }),
});
core.setCloudHost(cloudHost(core));
core.setSocketHost(socketHost(core));
const settings: { revision: bigint | null } = { revision: null };

const openSettings = (text: string) => {
  const opened = core.settingsOpen(text);
  settings.revision = core.settingsRevision();
  return opened;
};

const keepSettings = () => {
  if (settings.revision === null) return;
  const revision = core.settingsRevision();
  if (revision === settings.revision) return;
  settings.revision = revision;
  scope.postMessage({ settings: core.settingsText() });
};

const pumpAndKeepSettings = () => {
  if (settings.revision === null) return;
  core.pump();
  keepSettings();
};

setInterval(pumpAndKeepSettings, PUMP_MS);

const calls = {
  get: core.get,
  getFields: core.getFields,
  getMany: (paths: string[]) => paths.map((path) => core.get(path)),
  set: core.set,
  invoke: core.invoke,
  deviceInvoke: core.deviceInvoke,
  guided: core.guided,
  watchClient: core.watchClient,
  linkOpen: core.linkOpen,
  linkClose: core.linkClose,
  linksStart: core.linksStart,
  settingsOpen: openSettings,
  setMockLinksAvailable: core.setMockLinksAvailable,
  setSerialPorts: core.setSerialPorts,
  serialReceived: core.serialReceived,
  serialClosed: core.serialClosed,
  mapTile: core.mapTile,
  vfsWrite: core.vfsWrite,
  vfsRead: core.vfsRead,
  vfsList: core.vfsList,
  vfsRemove: core.vfsRemove,
  vfsRemovePrefixed: core.vfsRemovePrefixed,
  gcsPositionSource: core.gcsPositionSource,
  gcsPositionUpdate: core.gcsPositionUpdate,
  gcsPositionError: core.gcsPositionError,
  notifyDeepLink: core.notifyDeepLink,
  syntheticAttach: core.syntheticAttach,
  syntheticResize: core.syntheticResize,
  syntheticRelease: core.syntheticRelease,
} as const;

const reply = (id: string) => (result: unknown) => {
  keepSettings();
  scope.postMessage({ id, result });
};

scope.onmessage = ({ data: { id, call, args } }) => {
  const result = (calls[call] as (...given: unknown[]) => unknown)(...args);
  return result instanceof Promise
    ? void result.then(reply(id), (reason: unknown) => {
        console.error(`core call ${call} failed`, reason);
        reply(id)(null);
      })
    : reply(id)(result);
};
scope.postMessage({ ready: true });
