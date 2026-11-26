import { mutableStateOf } from '@/kotlin/State';
import { DEVICE_RETRY_MS, latestWins, readDeviceSettings, sendDeviceSettings } from './deviceSettings';
import { desktopCall, desktopEvents } from './desktop';


const SETTINGS_KEY = 'aircast.settings';
const WEB_SERIAL_PREFIX = 'webserial-';

type Reply = { id: string; result: unknown };
type SerialOp =
  | { op: 'open'; id: number; name: string; baud: number; dataBits: number; stopBits: number; parity: number }
  | { op: 'write'; id: number; bytes: Uint8Array }
  | { op: 'close'; id: number };
type Message = { ready: true } | { loading: number } | { event: { path: string; json: string } } | { serial: SerialOp } | { settings: string } | Reply;

type EventListener = (path: string, json: string) => void;

const calls = { next: 0 };
const pending = new Map<string, (result: unknown) => void>();
const eventListeners = new Set<EventListener>();
const serialPorts = new Map<string, SerialPort>();
const serialSessions = new Map<number, { port: SerialPort; writer: Promise<WritableStreamDefaultWriter<Uint8Array> | null>; reader: ReadableStreamDefaultReader<Uint8Array> | null }>();

const connection: { worker: Worker | null; ready: Promise<Worker> | null; desktop: Promise<unknown> | null } = { worker: null, ready: null, desktop: null };

const deliver = (path: string, json: string) => eventListeners.forEach((listener) => listener(path, json));

export const coreLoaded = mutableStateOf(0);

const RELOADED_KEY = 'aircast.reloadedForBuild';
const RELOAD_GAP_MS = 60_000;

export const reloadDue = (last: number, now: number): boolean => now - last >= RELOAD_GAP_MS;

const reloadForThisBuild = () => {
  try {
    if (!reloadDue(Number(sessionStorage.getItem(RELOADED_KEY) ?? 0), Date.now())) return;
    sessionStorage.setItem(RELOADED_KEY, String(Date.now()));
  } catch {
    return;
  }
  location.reload();
};

const connect = (): Promise<Worker> => {
  connection.ready ??= import('./coreWorker').catch((error: unknown) => {
    connection.ready = null;
    reloadForThisBuild();
    throw error;
  }).then(({ coreWorker }) => new Promise<Worker>((resolve) => {
    const worker = coreWorker();
    connection.worker = worker;
    worker.onerror = reloadForThisBuild;
    worker.onmessage = ({ data }: MessageEvent<Message>) => {
      if ('ready' in data) {
        worker.onerror = null;
        return resolve(worker);
      }
      if ('loading' in data) return void (coreLoaded.value = data.loading);
      if ('event' in data) return deliver(data.event.path, data.event.json);
      if ('serial' in data) return void serialOp(data.serial);
      if ('settings' in data) return keep(data.settings);
      pending.get(data.id)?.(data.result);
      pending.delete(data.id);
    };
  }));
  return connection.ready;
};

export const callTransferring = <T = unknown>(name: string, transfer: Transferable[], ...args: unknown[]): Promise<T> =>
  __DESKTOP__ ? desktopCall<T>(name, args) : connect().then((worker) => new Promise<T>((resolve) => {
    const id = String(calls.next++);
    pending.set(id, resolve as (result: unknown) => void);
    worker.postMessage({ id, call: name, args }, transfer);
  }));

export const call = <T = unknown>(name: string, ...args: unknown[]): Promise<T> => callTransferring<T>(name, [], ...args);

export const coreReady = (): Promise<unknown> => (__DESKTOP__ ? (connection.desktop ??= desktopEvents(deliver)) : connect());

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

const device = { keepsSettings: false, base: '' };

const saveOnDevice = latestWins(async (mine: string): Promise<boolean> => {
  const theirs = await readDeviceSettings();
  if (theirs === null) return false;
  const merged = theirs === device.base || theirs === '' ? mine : await call<string>('settingsMerged', device.base, mine, theirs);
  if (merged !== theirs && !(await sendDeviceSettings(merged))) return false;
  device.base = merged;
  if (merged === mine) return true;
  store(merged);
  await call('settingsAdopt', merged);
  return true;
}, DEVICE_RETRY_MS);

const keep = (text: string) => {
  store(text);
  if (device.keepsSettings) void saveOnDevice(text);
};

const openSettings = async (): Promise<void> => {
  const kept = await readDeviceSettings();
  device.keepsSettings = kept !== null;
  device.base = kept ?? '';
  const text = kept || stored();
  await call('settingsOpen', text);
  if (kept === '' && text !== '') void saveOnDevice(text);
};

export const webSerialAvailable = (): boolean => !__DESKTOP__ && typeof navigator !== 'undefined' && 'serial' in navigator;

export const serialAvailable = (): boolean => __DESKTOP__ || webSerialAvailable();

const portLine = (port: SerialPort, index: number): [string, string] => {
  const { usbVendorId, usbProductId } = port.getInfo();
  const hex = (value?: number) => (value === undefined ? '' : value.toString(16).padStart(4, '0'));
  const location = `${WEB_SERIAL_PREFIX}${index}`;
  const description = usbVendorId === undefined ? 'Serial port' : `USB serial ${hex(usbVendorId)}:${hex(usbProductId)}`;
  return [location, [location, description, '', '', usbProductId ?? '', usbVendorId ?? ''].join('\t')];
};

export const refreshSerialPorts = async (): Promise<void> => {
  if (!webSerialAvailable()) return;
  const ports = await navigator.serial.getPorts();
  const lines = ports.map(portLine);
  serialPorts.clear();
  lines.forEach(([location], index) => serialPorts.set(location, ports[index]!));
  await call('setSerialPorts', lines.map(([, line]) => line));
};

export const requestSerialPort = async (): Promise<void> => {
  if (__DESKTOP__) return;
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
  if (serialSessions.get(id)?.reader !== reader) return undefined;
  if (done || !value) return serialClosed(id, 'The serial port closed.');
  void call('serialReceived', id, value);
  return readSerial(id, reader);
};

const portTurns = new Map<SerialPort, Promise<unknown>>();

export const onPort = <T,>(port: SerialPort, work: () => Promise<T>): Promise<T> => {
  const turn = (portTurns.get(port) ?? Promise.resolve()).then(work, work);
  portTurns.set(port, turn.catch(() => undefined));
  return turn;
};

const openSerial = async (op: Extract<SerialOp, { op: 'open' }>) => {
  const port = serialPorts.get(op.name);
  if (!port) return serialClosed(op.id, `${op.name} is not available. Allow the port in the browser first.`);
  const opened = onPort(port, () => port.open({ baudRate: op.baud, dataBits: op.dataBits === 7 ? 7 : 8, stopBits: op.stopBits === 2 ? 2 : 1, parity: parityOf(op.parity) })).then(() => port.writable?.getWriter() ?? null);
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
  return onPort(session.port, async () => {
    await session.reader?.cancel().catch(() => undefined);
    (await session.writer)?.releaseLock();
    return session.port.close().catch(() => undefined);
  });
};

const adoptOtherTabs = () =>
  window.addEventListener('storage', ({ key, newValue }) => {
    if (key === SETTINGS_KEY && newValue !== null) void call('settingsAdopt', newValue);
  });

const startDesktopCore = async (): Promise<void> => {
  await coreReady();
  await call('setMockLinksAvailable', true);
  await call('linksStart');
};

export const startCore = async (): Promise<void> => {
  if (__DESKTOP__) return startDesktopCore();
  await openSettings();
  adoptOtherTabs();
  await call('setMockLinksAvailable', true);
  await refreshSerialPorts();
  await call('linksStart');
  if (serialAvailable()) {
    navigator.serial.addEventListener('connect', () => void refreshSerialPorts());
    navigator.serial.addEventListener('disconnect', () => void refreshSerialPorts());
  }
};
