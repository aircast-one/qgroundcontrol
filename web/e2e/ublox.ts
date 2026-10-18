import type { Page } from '@playwright/test';

export const UBLOX_VENDOR = 5446;
export const ZED_F9P_PRODUCT = 425;

const installUblox = ({ vendor, product }: { vendor: number; product: number }) => {
  const SYNC = [0xb5, 0x62];
  const CLASS_ACK = 0x05;
  const CLASS_CFG = 0x06;
  const CLASS_MON = 0x0a;
  const CLASS_NAV = 0x01;
  const MON_VER = 0x04;
  const NAV_SVIN = 0x3b;
  const KEY_TMODE_MODE = 0x20030001;

  const checksum = (body: readonly number[]) => body.reduce(([a, b], byte) => [(a + byte) & 0xff, (b + ((a + byte) & 0xff)) & 0xff] as [number, number], [0, 0] as [number, number]);
  const frame = (cls: number, id: number, payload: readonly number[]) => {
    const body = [cls, id, payload.length & 0xff, payload.length >> 8, ...payload];
    return new Uint8Array([...SYNC, ...body, ...checksum(body)]);
  };
  const field = (text: string, size: number) => Array.from({ length: size }, (_, at) => text.charCodeAt(at) || 0);
  const monVer = [...field('ROM CORE 1.00', 30), ...field('00190000', 10), ...['FWVER=HPG 1.32', 'PROTVER=27.31', 'MOD=ZED-F9P'].flatMap((it) => field(it, 30))];
  const le32 = (value: number) => [value & 0xff, (value >> 8) & 0xff, (value >> 16) & 0xff, (value >>> 24) & 0xff];
  const svin = (seconds: number) => [...new Array(8).fill(0), ...le32(seconds), ...le32(400_000_000), ...le32(0), ...le32(400_000_000), 0, 0, 0, 0, ...le32(25_000), ...le32(seconds * 4), 0, 1, 0, 0];

  const state = { pending: [] as number[], controller: null as ReadableStreamDefaultController<Uint8Array> | null, surveying: false, seconds: 0 };
  const send = (bytes: Uint8Array) => state.controller?.enqueue(bytes);
  const heard = (bytes: Uint8Array) => {
    state.pending = [...state.pending, ...bytes];
    const take = (): void => {
      const start = state.pending.findIndex((byte, at) => byte === SYNC[0] && state.pending[at + 1] === SYNC[1]);
      if (start < 0 || state.pending.length < start + 8) return;
      const length = (state.pending[start + 4] ?? 0) | ((state.pending[start + 5] ?? 0) << 8);
      if (state.pending.length < start + 8 + length) return;
      const [cls, id] = [state.pending[start + 2] ?? 0, state.pending[start + 3] ?? 0];
      const payload = state.pending.slice(start + 6, start + 6 + length);
      state.pending = state.pending.slice(start + 8 + length);
      if (cls === CLASS_CFG) send(frame(CLASS_ACK, 0x01, [cls, id]));
      if (cls === CLASS_MON && id === MON_VER) send(frame(CLASS_MON, MON_VER, monVer));
      const key = (payload[4] ?? 0) | ((payload[5] ?? 0) << 8) | ((payload[6] ?? 0) << 16) | ((payload[7] ?? 0) << 24);
      if (cls === CLASS_CFG && id === 0x8a && key === KEY_TMODE_MODE) state.surveying = true;
      take();
    };
    take();
  };
  setInterval(() => {
    if (!state.surveying) return;
    state.seconds = state.seconds + 1;
    send(frame(CLASS_NAV, NAV_SVIN, svin(state.seconds)));
  }, 1000);

  const streams = { open: false, readable: null as ReadableStream<Uint8Array> | null, writable: null as WritableStream<Uint8Array> | null };
  const port = {
    getInfo: () => ({ usbVendorId: vendor, usbProductId: product }),
    open: async () => {
      if (streams.open) throw new DOMException('The port is already open.', 'InvalidStateError');
      streams.open = true;
      streams.readable = new ReadableStream<Uint8Array>({ start: (controller) => void (state.controller = controller) });
      streams.writable = new WritableStream<Uint8Array>({ write: (bytes) => heard(bytes) });
    },
    close: async () => {
      if (!streams.open) throw new DOMException('The port is already closed.', 'InvalidStateError');
      streams.open = false;
      streams.readable = null;
      streams.writable = null;
      state.controller = null;
    },
    get readable() {
      return streams.readable;
    },
    get writable() {
      return streams.writable;
    },
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  };
  const serial = { getPorts: async () => [port], requestPort: async () => port, addEventListener: () => undefined, removeEventListener: () => undefined };
  Object.defineProperty(navigator, 'serial', { value: serial, configurable: true });
};

export const plugInUblox = (page: Page) => page.addInitScript(installUblox, { vendor: UBLOX_VENDOR, product: ZED_F9P_PRODUCT });
