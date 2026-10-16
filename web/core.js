const SETTINGS_KEY = 'aircast.settings';
const SETTINGS_SAVE_MS = 2000;

const worker = new Worker('worker.js', { type: 'module' });
const pending = new Map();
const transports = new Map();
const links = new Map();

const ready = new Promise((resolve) => {
  worker.onmessage = ({ data }) => {
    if (data.ready) {
      resolve();
    } else if (data.write) {
      transports.get(data.write.id)?.send(data.write.bytes);
    } else {
      pending.get(data.id)?.(data.result);
      pending.delete(data.id);
    }
  };
});

const call = (name, ...args) => ready.then(() => new Promise((resolve) => {
  const id = crypto.randomUUID();
  pending.set(id, resolve);
  worker.postMessage({ id, call: name, args });
}));

const parsed = (text) => {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
};

const stored = () => {
  try {
    return localStorage.getItem(SETTINGS_KEY) ?? '';
  } catch {
    return '';
  }
};

const store = (text) => {
  try {
    localStorage.setItem(SETTINGS_KEY, text);
  } catch {
    return;
  }
};

const keepSettings = (revision) => setTimeout(async () => {
  const now = await call('settingsRevision');
  if (now !== revision) store(await call('settingsText'));
  keepSettings(now);
}, SETTINGS_SAVE_MS);

export const start = async () => {
  await call('settingsOpen', stored());
  keepSettings(await call('settingsRevision'));
};

export const views = async (paths) => {
  const answers = await call('getMany', paths);
  return Object.fromEntries(paths.map((path, at) => [path, parsed(answers[at])]));
};

export const set = async (path, value) => parsed(await call('set', path, JSON.stringify({ value })));

export const invoke = async (path, ...args) => parsed(await call('invoke', path, JSON.stringify(args)));

export const openLinks = () => [...links.values()];

const opened = (answer, name, close) => {
  const reply = parsed(answer);
  if (reply?.ok) links.set(reply.id, { id: reply.id, name, close });
  return reply;
};

export const closeLink = (id) => links.get(id)?.close(id);

const hostClosed = (id, reason) => {
  transports.delete(id);
  links.delete(id);
  return call('hostLinkClosed', id, reason);
};

export const openMock = async (firmwareType, name) => opened(
  await call('linkOpen', JSON.stringify({ kind: 'mock', name, firmwareType, vehicleType: 2 })),
  name,
  (id) => {
    links.delete(id);
    return call('linkClose', id, 'Disconnected by the operator');
  },
);

export const openSerial = async (baudRate) => {
  const port = await navigator.serial.requestPort();
  await port.open({ baudRate });
  const { usbVendorId, usbProductId } = port.getInfo();
  const name = `Serial ${usbVendorId?.toString(16) ?? '?'}:${usbProductId?.toString(16) ?? '?'}`;
  const reader = port.readable.getReader();
  const writer = port.writable.getWriter();
  const reply = opened(await call('hostLinkOpen', 'serial', name), name, () => reader.cancel());
  if (!reply?.ok) {
    await port.close();
    return reply;
  }
  transports.set(reply.id, { send: (bytes) => writer.write(bytes) });
  const read = async () => {
    const { value, done } = await reader.read().catch(() => ({ done: true }));
    if (!done) {
      call('hostLinkBytes', reply.id, value);
      return read();
    }
    writer.releaseLock();
    reader.releaseLock();
    await port.close().catch(() => undefined);
    return hostClosed(reply.id, 'The serial port closed');
  };
  read();
  return reply;
};

export const openSocket = async (url) => {
  const socket = new WebSocket(url);
  socket.binaryType = 'arraybuffer';
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = () => reject(new Error(`No MAVLink WebSocket answered at ${url}`));
  });
  const reply = opened(await call('hostLinkOpen', 'websocket', url), url, () => socket.close());
  if (!reply?.ok) {
    socket.close();
    return reply;
  }
  transports.set(reply.id, { send: (bytes) => socket.send(bytes) });
  socket.onmessage = ({ data }) => call('hostLinkBytes', reply.id, new Uint8Array(data));
  socket.onclose = () => hostClosed(reply.id, 'The WebSocket closed');
  return reply;
};
