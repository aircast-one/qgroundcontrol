import { type Json, isNull, optBoolean } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import type { Scope } from '@/kotlin/Compose';
import { coerceIn, roundToInt, same } from '@/kotlin/Std';

export const JOYSTICK_DEVICES = 'joystick.devices';
export const JOYSTICK_INPUT = 'joystick.input';
const SAMPLE_MS = 2;
const IDLE_MS = 250;
const JOYSTICK_VIEW = 'view.joystick';

export const streams = (view: Json | null): boolean => view !== null && ((optBoolean(view, 'vehicle') && optBoolean(view, 'enabled')) || !isNull(view, 'calibration'));

const AXIS_SCALE = 32767;
const HAT_UP = 0x01;
const HAT_RIGHT = 0x02;
const HAT_DOWN = 0x04;
const HAT_LEFT = 0x08;
const HAT_THRESHOLD = 0.5;

const STANDARD_LEFT_TRIGGER = 6;
const STANDARD_RIGHT_TRIGGER = 7;
const STANDARD_DPAD = { up: 12, down: 13, left: 14, right: 15 } as const;

export const GAMEPAD_BUTTONS: readonly number[] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, 10, 11, 17, 18, 19, 20, 21, 22, 23, 24];

const BatteryState = { STATUS_CHARGING: 2, STATUS_DISCHARGING: 3, STATUS_NOT_CHARGING: 4, STATUS_FULL: 5 } as const;

export const powerStateText = (status: number): string => {
  switch (status) {
    case BatteryState.STATUS_CHARGING: return 'Charging';
    case BatteryState.STATUS_DISCHARGING: return 'Discharging';
    case BatteryState.STATUS_FULL: return 'Full';
    case BatteryState.STATUS_NOT_CHARGING: return 'Not charging';
    default: return '';
  }
};

const isStandard = (pad: Gamepad): boolean => pad.mapping === 'standard';

const usbIds = (id: string): [number, number] => {
  const chrome = /Vendor: ([0-9a-f]{1,4}) Product: ([0-9a-f]{1,4})/i.exec(id);
  const firefox = /^([0-9a-f]{1,4})-([0-9a-f]{1,4})-/i.exec(id);
  const found = chrome ?? firefox;
  return found ? [parseInt(found[1] ?? '0', 16), parseInt(found[2] ?? '0', 16)] : [0, 0];
};

const padName = (id: string): string => id.replace(/^[0-9a-f]{1,4}-[0-9a-f]{1,4}-/i, '').replace(/\s*\([^)]*\)\s*$/, '').trim() || id;

const deviceDetails = (pad: Gamepad): Json => {
  const [vendorId, productId] = usbIds(pad.id);
  return {
    vendorId,
    productId,
    guid: pad.id,
    playerIndex: pad.index,
    batteryPercent: -1,
    powerState: '',
    rumble: pad.vibrationActuator !== null && pad.vibrationActuator !== undefined,
    led: false,
    gyroscope: false,
    accelerometer: false,
  };
};

export const hatBits = (x: number, y: number): number =>
  (y < -HAT_THRESHOLD ? HAT_UP : 0) | (y > HAT_THRESHOLD ? HAT_DOWN : 0) | (x < -HAT_THRESHOLD ? HAT_LEFT : 0) | (x > HAT_THRESHOLD ? HAT_RIGHT : 0);

export const scaled = (value: number): number => roundToInt(coerceIn(value, -1, 1) * AXIS_SCALE);

const held = (pad: Gamepad, index: number): boolean => pad.buttons[index]?.pressed === true;

const axesOf = (pad: Gamepad): number[] =>
  isStandard(pad) ? [...pad.axes, pad.buttons[STANDARD_LEFT_TRIGGER]?.value ?? 0, pad.buttons[STANDARD_RIGHT_TRIGGER]?.value ?? 0] : [...pad.axes];

const hatOf = (pad: Gamepad): number =>
  isStandard(pad)
    ? hatBits((held(pad, STANDARD_DPAD.right) ? 1 : 0) - (held(pad, STANDARD_DPAD.left) ? 1 : 0), (held(pad, STANDARD_DPAD.down) ? 1 : 0) - (held(pad, STANDARD_DPAD.up) ? 1 : 0))
    : 0;

const pressedOf = (pad: Gamepad): boolean[] => (isStandard(pad) ? GAMEPAD_BUTTONS.map((index) => held(pad, index)) : GAMEPAD_BUTTONS.map((_, index) => held(pad, index)));

const shown = (): boolean => typeof document === 'undefined' || document.visibilityState === 'visible';

const pads = (): Gamepad[] => (typeof navigator !== 'undefined' && 'getGamepads' in navigator && shown() ? navigator.getGamepads().filter((pad): pad is Gamepad => pad !== null && pad.connected) : []);

type Sample = { name: string; axes: number[]; pressed: boolean[]; hat: number };

const sampleOf = (pad: Gamepad): Sample => ({ name: padName(pad.id), axes: axesOf(pad).map(scaled), pressed: pressedOf(pad), hat: hatOf(pad) });

const gamepad: { sampler: Scope | null; listening: boolean } = { sampler: null, listening: false };

const rescan = () => {
  const devices = pads().map((pad) => ({ ...deviceDetails(pad), name: padName(pad.id), axes: axesOf(pad).length, buttons: GAMEPAD_BUTTONS.length, hats: isStandard(pad) ? 1 : 0, gamepad: isStandard(pad) }));
  void Qgc.invoke(JOYSTICK_DEVICES, devices);
};

const sample = async (scope: Scope, sent: Sample[], streaming: boolean, checkedAt: number): Promise<void> => {
  if (!scope.isActive() || gamepad.sampler !== scope) return;
  const sampled = pads().map(sampleOf);
  const now = Date.now();
  const checking = sampled.length > 0 && now - checkedAt >= IDLE_MS;
  const streamingNow = checking ? streams(await Qgc.get(JOYSTICK_VIEW)) : streaming;
  const sending = streamingNow || !same(sampled, sent);
  if (sending) await Promise.all(sampled.map((it) => Qgc.invoke(JOYSTICK_INPUT, it.name, it.axes, it.pressed, [it.hat])));
  setTimeout(() => void sample(scope, sending ? sampled : sent, streamingNow, checking ? now : checkedAt), sampled.length === 0 ? IDLE_MS : SAMPLE_MS);
};

export const GamepadInput = {
  start(scope: Scope): void {
    if (!gamepad.listening && typeof window !== 'undefined') {
      gamepad.listening = true;
      window.addEventListener('gamepadconnected', rescan);
      window.addEventListener('gamepaddisconnected', rescan);
      document.addEventListener('visibilitychange', rescan);
    }
    rescan();
    gamepad.sampler = scope;
    void sample(scope, [], false, 0);
  },
};
