const SETTINGS_KEY = 'aircast.settings';
const SETTINGS_SAVE_MS = 2000;
const WEB_SERIAL_PREFIX = 'webserial-';

type Reply = { id: string; result: unknown };
type SerialOp =
  | { op: 'open'; id: number; name: string; baud: number; dataBits: number; stopBits: number; parity: number }
  | { op: 'write'; id: number; bytes: Uint8Array }
  | { op: 'close'; id: number };
type Message = { ready: true } | { event: { path: string; json: string } } | { write: { id: number; bytes: Uint8Array } } | { serial: SerialOp } | Reply;

type EventListener = (path: string, json: string) => void;

const pending = new Map<string, (result: unknown) => void>();
const eventListeners = new Set<EventListener>();
const hostLinks = new Map<number, (bytes: Uint8Array) => void>();
const serialPorts = new Map<string, SerialPort>();
const serialSessions = new Map<number, { port: SerialPort; writer: Promise<WritableStreamDefaultWriter<Uint8Array> | null>; reader: ReadableStreamDefaultReader<Uint8Array> | null }>();

const connection: { worker: Worker | null; ready: Promise<Worker> | null } = { worker: null, ready: null };

const connect = (): Promise<Worker> => {
  connection.ready ??= new Promise<Worker>((resolve) => {
    const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
    connection.worker = worker;
    worker.onmessage = ({ data }: MessageEvent<Message>) => {
      if ('ready' in data) return resolve(worker);
      if ('event' in data) return eventListeners.forEach((listener) => listener(data.event.path, data.event.json));
      if ('write' in data) return hostLinks.get(data.write.id)?.(data.write.bytes);
      if ('serial' in data) return void serialOp(data.serial);
      pending.get(data.id)?.(data.result);
      pending.delete(data.id);
    };
  });
  return connection.ready;
};

export const call = <T = unknown>(name: string, ...args: unknown[]): Promise<T> =>
  connect().then((worker) => new Promise<T>((resolve) => {
    const id = crypto.randomUUID();
    pending.set(id, resolve as (result: unknown) => void);
    worker.postMessage({ id, call: name, args });
  }));

export const onCoreEvent = (listener: EventListener): (() => void) => {
  eventListeners.add(listener);
  return () => eventListeners.delete(listener);
};

const stored = (): string => {
  try {
    return localStorage.getItem(SETTINGS_KEY) ?? '';
  } catch {
    return '';
  }
};

const store = (text: string) => {
  try {
    localStorage.setItem(SETTINGS_KEY, text);
  } catch {
    return;
  }
};

const keepSettings = (revision: bigint) => setTimeout(async () => {
  const now = await call<bigint>('settingsRevision');
  if (now !== revision) store(await call<string>('settingsText'));
  keepSettings(now);
}, SETTINGS_SAVE_MS);

const serialAvailable = () => typeof navigator !== 'undefined' && 'serial' in navigator;

const portLine = (port: SerialPort, index: number): [string, string] => {
  const { usbVendorId, usbProductId } = port.getInfo();
  const hex = (value?: number) => (value === undefined ? '' : value.toString(16).padStart(4, '0'));
  const location = `${WEB_SERIAL_PREFIX}${index}`;
  const description = usbVendorId === undefined ? 'Serial port' : `USB serial ${hex(usbVendorId)}:${hex(usbProductId)}`;
  return [location, [location, description, '', '', usbProductId ?? '', usbVendorId ?? ''].join('\t')];
};

export const refreshSerialPorts = async (): Promise<void> => {
  if (!serialAvailable()) return;
  const ports = await navigator.serial.getPorts();
  const lines = ports.map(portLine);
  serialPorts.clear();
  lines.forEach(([location], index) => serialPorts.set(location, ports[index]!));
  await call('setSerialPorts', lines.map(([, line]) => line));
};

export const requestSerialPort = async (): Promise<void> => {
  await navigator.serial.requestPort();
  await refreshSerialPorts();
};

const parityOf = (qtParity: number): ParityType => ({ 2: 'even', 3: 'odd' } as Record<number, ParityType>)[qtParity] ?? 'none';

const serialClosed = (id: number, reason: string) => {
  serialSessions.delete(id);
  return call('serialClosed', id, reason);
};

const readSerial = async (id: number, reader: ReadableStreamDefaultReader<Uint8Array>): Promise<unknown> => {
  const { value, done } = await reader.read().catch(() => ({ value: undefined, done: true }));
  if (done || !value) return serialClosed(id, 'The serial port closed.');
  void call('serialReceived', id, value);
  return readSerial(id, reader);
};

const openSerial = async (op: Extract<SerialOp, { op: 'open' }>) => {
  const port = serialPorts.get(op.name);
  if (!port) return serialClosed(op.id, `${op.name} is not available. Allow the port in the browser first.`);
  const opened = port
    .open({ baudRate: op.baud, dataBits: op.dataBits === 7 ? 7 : 8, stopBits: op.stopBits === 2 ? 2 : 1, parity: parityOf(op.parity) })
    .then(() => port.writable?.getWriter() ?? null);
  serialSessions.set(op.id, { port, writer: opened.catch(() => null), reader: null });
  try {
    await opened;
  } catch (error) {
    return serialClosed(op.id, error instanceof Error ? error.message : String(error));
  }
  const reader = port.readable?.getReader() ?? null;
  const session = serialSessions.get(op.id);
  if (!reader || !session) return serialClosed(op.id, 'The serial port has no readable stream.');
  serialSessions.set(op.id, { ...session, reader });
  return readSerial(op.id, reader);
};

const serialOp = async (op: SerialOp) => {
  if (op.op === 'open') return openSerial(op);
  const session = serialSessions.get(op.id);
  if (!session) return undefined;
  if (op.op === 'write') return (await session.writer)?.write(op.bytes);
  serialSessions.delete(op.id);
  await session.reader?.cancel().catch(() => undefined);
  (await session.writer)?.releaseLock();
  return session.port.close().catch(() => undefined);
};

export const openWebSocketLink = async (url: string): Promise<{ ok: boolean; id?: number; reason?: string }> => {
  const socket = new WebSocket(url);
  socket.binaryType = 'arraybuffer';
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = () => reject(new Error(`No MAVLink WebSocket answered at ${url}`));
  });
  const reply = JSON.parse(await call<string>('hostLinkOpen', 'websocket', url)) as { ok: boolean; id?: number; reason?: string };
  if (!reply.ok || reply.id === undefined) {
    socket.close();
    return reply;
  }
  const id = reply.id;
  hostLinks.set(id, (bytes) => socket.send(bytes as Uint8Array<ArrayBuffer>));
  socket.onmessage = ({ data }) => void call('hostLinkBytes', id, new Uint8Array(data as ArrayBuffer));
  socket.onclose = () => {
    hostLinks.delete(id);
    void call('hostLinkClosed', id, 'The WebSocket closed');
  };
  return reply;
};

export const startCore = async (): Promise<void> => {
  await call('settingsOpen', stored());
  await call('setMockLinksAvailable', true);
  await refreshSerialPorts();
  if (serialAvailable()) {
    navigator.serial.addEventListener('connect', () => void refreshSerialPorts());
    navigator.serial.addEventListener('disconnect', () => void refreshSerialPorts());
  }
  keepSettings(await call<bigint>('settingsRevision'));
};
