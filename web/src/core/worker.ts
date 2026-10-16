import init, * as core from './pkg/groundstation.js';

const PUMP_MS = 20;

type Scope = {
  postMessage(message: unknown, transfer?: Transferable[]): void;
  onmessage: ((event: MessageEvent<{ id: string; call: keyof typeof calls; args: unknown[] }>) => void) | null;
};

const scope = self as unknown as Scope;
const owned = (bytes: Uint8Array) => [bytes.buffer as ArrayBuffer];

await init();
core.onEvent((path: string, json: string) => scope.postMessage({ event: { path, json } }));
core.onLinkWrite((id: number, bytes: Uint8Array) => scope.postMessage({ write: { id, bytes } }, owned(bytes)));
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
setInterval(core.pump, PUMP_MS);

const calls = {
  get: core.get,
  getFields: core.getFields,
  getMany: (paths: string[]) => paths.map((path) => core.get(path)),
  set: core.set,
  invoke: core.invoke,
  guided: core.guided,
  watchClient: core.watchClient,
  linkOpen: core.linkOpen,
  linkClose: core.linkClose,
  hostLinkOpen: core.hostLinkOpen,
  hostLinkBytes: core.hostLinkBytes,
  hostLinkClosed: core.hostLinkClosed,
  settingsOpen: core.settingsOpen,
  settingsText: core.settingsText,
  settingsRevision: core.settingsRevision,
  setMockLinksAvailable: core.setMockLinksAvailable,
  setSerialPorts: core.setSerialPorts,
  serialReceived: core.serialReceived,
  serialClosed: core.serialClosed,
} as const;

scope.onmessage = ({ data: { id, call, args } }) => {
  const run = calls[call] as (...given: unknown[]) => unknown;
  scope.postMessage({ id, result: run(...args) });
};
scope.postMessage({ ready: true });
