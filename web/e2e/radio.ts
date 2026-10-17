import type { Page } from '@playwright/test';

export type Sticks = { roll: number; pitch: number; throttle: number; yaw: number };

type RadioWindow = Window & { radioSticks?: (axes: readonly number[]) => void };

const installRadio = () => {
  const radio = { axes: [0, 0, 0, 0, 0, 0, 0, 0], since: performance.now() };
  const pad = () => ({
    id: 'RM TX16S Joystick (Vendor: 1209 Product: 4f54)',
    index: 0,
    connected: true,
    mapping: '',
    timestamp: radio.since,
    axes: [...radio.axes],
    buttons: Array.from({ length: 21 }, () => ({ pressed: false, touched: false, value: 0 })),
    vibrationActuator: null,
  });
  Object.defineProperty(navigator, 'getGamepads', { value: () => [pad(), null, null, null] });
  (window as RadioWindow).radioSticks = (axes) => {
    radio.axes = [...axes, 0, 0, 0, 0];
    radio.since = performance.now();
  };
  addEventListener('load', () => dispatchEvent(new Event('gamepadconnected')));
};

export const plugInRadio = (page: Page) => page.addInitScript(installRadio);

export const moveSticks = (page: Page, { roll, pitch, throttle, yaw }: Sticks) =>
  page.evaluate((axes) => (window as RadioWindow).radioSticks?.(axes), [roll, pitch, throttle, yaw]);
