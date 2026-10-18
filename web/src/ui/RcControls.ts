import { type Json, isJson, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';

export const PWM_MIN = 1000;
export const PWM_CENTER = 1500;
export const PWM_MAX = 2000;

export type RcControlType = { readonly name: 'Slider' | 'Button' | 'Switch3' | 'Momentary'; readonly ordinal: number };

export const RcControlType = {
  Slider: { name: 'Slider', ordinal: 0 },
  Button: { name: 'Button', ordinal: 1 },
  Switch3: { name: 'Switch3', ordinal: 2 },
  Momentary: { name: 'Momentary', ordinal: 3 },
} as const satisfies Record<string, RcControlType>;

export const RcControlTypeEntries: readonly RcControlType[] = [RcControlType.Slider, RcControlType.Button, RcControlType.Switch3, RcControlType.Momentary];

export type RcControl = { label: string; channel: number; type: RcControlType };

const TYPE_NAMES: ReadonlyMap<string, RcControlType> = new Map<string, RcControlType>([
  ['slider', RcControlType.Slider],
  ['button', RcControlType.Button],
  ['switch3', RcControlType.Switch3],
  ['momentary', RcControlType.Momentary],
]);

const typeOf = (raw: string): RcControlType | null => TYPE_NAMES.get(raw) ?? null;

const array = (json: string | null): unknown[] | null => {
  try {
    const parsed: unknown = JSON.parse(json ?? '');
    return Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
};

export const parseRcControls = (json: string | null): readonly RcControl[] =>
  (array(json) ?? []).filter(isJson).flatMap((entry) => {
    const channel = optInt(entry, 'channel', 0);
    const type = typeOf(optText(entry, 'type'));
    const label = optText(entry, 'label');
    return channel <= 0 || type === null ? [] : [{ label: label.trim() === '' ? `CH${channel}` : label, channel, type }];
  });

export const RC_CONTROLS_VIEW = 'view.rcControls';

export type RcControlState = RcControl & { pwm: number | null; position: number | null; on: boolean | null; held: boolean };

const wholeOrNull = (json: Json, key: string): number | null => (typeof json[key] === 'number' ? Math.trunc(json[key]) : null);

export const parseRcControlStates = (view: Json | null): readonly RcControlState[] =>
  (optJSONArray(view, 'controls') ?? []).filter(isJson).flatMap((entry) => {
    const channel = optInt(entry, 'channel', 0);
    const type = typeOf(optText(entry, 'type'));
    const on = entry['on'];
    return channel <= 0 || type === null
      ? []
      : [{ label: optText(entry, 'label'), channel, type, pwm: wholeOrNull(entry, 'pwm'), position: wholeOrNull(entry, 'position'), on: typeof on === 'boolean' ? on : null, held: optBoolean(entry, 'held') }];
  });

export const switch3Pwms = (): readonly number[] => [PWM_MIN, PWM_CENTER, PWM_MAX];

export const RC_CHANNEL_MIN = 1;
export const RC_CHANNEL_MAX = 18;

export const typeName = (type: RcControlType): string => ({ Slider: 'slider', Button: 'button', Switch3: 'switch3', Momentary: 'momentary' })[type.name];

export const typeLabel = (type: RcControlType): string => ({ Slider: 'Slider', Button: 'Button', Switch3: 'Three-way switch', Momentary: 'Momentary' })[type.name];

const entries = (json: string | null): readonly Json[] => (array(json) ?? []).map((entry) => (isJson(entry) ? entry : {}));

const encode = (list: readonly Json[]): string => JSON.stringify(list);

const patched = (entry: Json, label: string, channel: number, type: RcControlType): Json => ({ ...entry, label, channel, type: typeName(type) });

export const rcControlsAdded = (json: string | null, label: string, channel: number, type: RcControlType): string => encode([...entries(json), patched({}, label, channel, type)]);

export const rcControlsRemoved = (json: string | null, index: number): string => encode(entries(json).filter((_, at) => at !== index));

export const rcControlsPatched = (json: string | null, index: number, label: string, channel: number, type: RcControlType): string =>
  encode(entries(json).map((entry, at) => (at === index ? patched(entry, label, channel, type) : entry)));

export const channelOwner = (json: string | null, channel: number, ignoring: number, reserved: ReadonlyMap<number, string>): string | null => {
  const owner = reserved.get(channel);
  if (owner !== undefined) return owner;
  const at = entries(json).findIndex((entry, index) => index !== ignoring && optInt(entry, 'channel', 0) === channel);
  if (at < 0) return null;
  const label = optText(entries(json)[at], 'label');
  return label.trim() === '' ? `control ${at + 1}` : label;
};

export const firstFreeChannel = (json: string | null, reserved: ReadonlyMap<number, string>): number =>
  Array.from({ length: RC_CHANNEL_MAX - RC_CHANNEL_MIN + 1 }, (_, at) => RC_CHANNEL_MIN + at).find((it) => channelOwner(json, it, -1, reserved) === null) ?? RC_CHANNEL_MIN;

export const channelUsable = (channel: number): boolean => channel >= RC_CHANNEL_MIN && channel <= RC_CHANNEL_MAX;

export const RC_SEND_INTERVAL_MS = 100;

export const rcSendDue = (nowMs: number, lastSentMs: number, finished: boolean): boolean => finished || nowMs - lastSentMs >= RC_SEND_INTERVAL_MS;
