import type { Page } from '@playwright/test';

export const STM32_VCP_VENDOR = 0x0483;
export const STM32_VCP_PRODUCT = 0x5740;
export const RADIO_PORT_LABEL = /USB serial 0483:5740/;

type RadioWindow = Window & { radioChannels?: (raw: Readonly<Record<number, number>>) => void; radioStreaming?: (on: boolean) => void };

const installRadio = ({ vendor, product }: { vendor: number; product: number }) => {
  const radio = { raw: Array.from({ length: 24 }, (_, at) => (at === 2 ? -1024 : 0)), streaming: true, controller: null as ReadableStreamDefaultController<Uint8Array> | null };
  const line = () => new TextEncoder().encode(`$${radio.raw.map((value) => `${value},`).join('')}\r\n`);
  setInterval(() => {
    if (radio.streaming) radio.controller?.enqueue(line());
  }, 20);
  (window as RadioWindow).radioChannels = (raw) => {
    radio.raw = radio.raw.map((value, at) => raw[at + 1] ?? value);
  };
  (window as RadioWindow).radioStreaming = (on) => {
    radio.streaming = on;
  };

  const streams = { open: false, readable: null as ReadableStream<Uint8Array> | null, writable: null as WritableStream<Uint8Array> | null };
  const port = {
    getInfo: () => ({ usbVendorId: vendor, usbProductId: product }),
    open: async () => {
      if (streams.open) throw new DOMException('The port is already open.', 'InvalidStateError');
      streams.open = true;
      streams.readable = new ReadableStream<Uint8Array>({ start: (controller) => void (radio.controller = controller) });
      streams.writable = new WritableStream<Uint8Array>();
    },
    close: async () => {
      if (!streams.open) throw new DOMException('The port is already closed.', 'InvalidStateError');
      streams.open = false;
      streams.readable = null;
      streams.writable = null;
      radio.controller = null;
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

export const plugInR2d2Radio = (page: Page) => page.addInitScript(installRadio, { vendor: STM32_VCP_VENDOR, product: STM32_VCP_PRODUCT });

export const setRadioChannels = (page: Page, raw: Readonly<Record<number, number>>) => page.evaluate((values) => (window as RadioWindow).radioChannels?.(values), raw);

export const setRadioStreaming = (page: Page, on: boolean) => page.evaluate((value) => (window as RadioWindow).radioStreaming?.(value), on);
