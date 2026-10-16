import init, { setMockLinksAvailable, linkOpen, hostLinkOpen, hostLinkBytes, hostLinkClosed, onLinkWrite, pump, get, guided } from './pkg/groundstation.js';

await init();
setMockLinksAvailable(true);
onLinkWrite((id, bytes) => postMessage({ write: { id, bytes } }, [bytes.buffer]));
setInterval(pump, 20);

const calls = { linkOpen, hostLinkOpen, hostLinkBytes, hostLinkClosed, get, guided };
onmessage = ({ data: { id, call, args } }) => postMessage({ id, result: calls[call](...args) });
postMessage({ ready: true });
