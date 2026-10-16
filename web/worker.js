import init, * as core from './pkg/groundstation.js';

await init();
core.setMockLinksAvailable(true);
core.onLinkWrite((id, bytes) => postMessage({ write: { id, bytes } }, [bytes.buffer]));
setInterval(core.pump, 20);

const calls = {
  get: core.get,
  getMany: (paths) => paths.map((path) => core.get(path)),
  set: core.set,
  invoke: core.invoke,
  linkOpen: core.linkOpen,
  linkClose: core.linkClose,
  hostLinkOpen: core.hostLinkOpen,
  hostLinkBytes: core.hostLinkBytes,
  hostLinkClosed: core.hostLinkClosed,
  settingsOpen: core.settingsOpen,
  settingsText: core.settingsText,
  settingsRevision: core.settingsRevision,
};
onmessage = ({ data: { id, call, args } }) => postMessage({ id, result: calls[call](...args) });
postMessage({ ready: true });
