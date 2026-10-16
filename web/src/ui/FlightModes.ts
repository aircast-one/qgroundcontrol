import { type Json, jsonObjects, optBoolean, optJSONArray, optJSONObject, optLong, optText } from '@/bridge/Json';

export const FLIGHT_MODES = 'view.flightModes';

export type FlightModeOption = {
  name: string;
  summary: string;
  current: boolean;
  needsConfirm: boolean;
  section: string;
  quick: boolean;
  caution: string;
};

export const FlightModeOption = (fields: { name: string; summary: string; current: boolean; needsConfirm: boolean; section?: string; quick?: boolean; caution?: string }): FlightModeOption => ({
  section: 'normal',
  quick: false,
  caution: '',
  ...fields,
});

export type FlightModesView = {
  canSet: boolean;
  current: string;
  currentSummary: string;
  all: readonly FlightModeOption[];
  quick: readonly FlightModeOption[];
  pinnedSetting: string | null;
  unknownModeNotice: string;
  cannotSetNotice: string;
};

const blankToNull = (text: string): string | null => (text.trim() === '' ? null : text);

const options = (view: Json | null, key: string): readonly FlightModeOption[] =>
  jsonObjects(optJSONArray(view, key)).map((mode) =>
    FlightModeOption({
      name: optText(mode, 'name'),
      summary: optText(mode, 'summary'),
      current: optBoolean(mode, 'current'),
      needsConfirm: optBoolean(mode, 'needsConfirm'),
      section: blankToNull(optText(mode, 'section')) ?? 'normal',
      quick: optBoolean(mode, 'quick'),
      caution: optText(mode, 'caution'),
    }),
  );

export const flightModesView = (view: Json | null): FlightModesView | null =>
  view === null || !optBoolean(view, 'available')
    ? null
    : {
        canSet: optBoolean(view, 'canSet'),
        current: optText(view, 'current'),
        currentSummary: optText(view, 'currentSummary'),
        all: options(view, 'modes'),
        quick: options(view, 'quick'),
        pinnedSetting: blankToNull(optText(view, 'pinnedSetting')),
        unknownModeNotice: optText(view, 'unknownModeNotice'),
        cannotSetNotice: optText(view, 'cannotSetNotice'),
      };

export type ModeAck = { serial: number; accepted: boolean };

export const modeAck = (view: Json | null): ModeAck | null => {
  const ack = optJSONObject(view, 'modeAck');
  return ack === null ? null : { serial: optLong(ack, 'serial'), accepted: optBoolean(ack, 'accepted') };
};

export const MODE_REPLY_MS = 3000;

export type ModeOutcome = { readonly kind: 'Pending' } | { readonly kind: 'Settled' } | { readonly kind: 'Unsettled' };

export const ModeOutcome = {
  Pending: { kind: 'Pending' },
  Settled: { kind: 'Settled' },
  Unsettled: { kind: 'Unsettled' },
} as const satisfies Record<string, ModeOutcome>;

export const modeOutcome = (before: ModeAck | null, now: ModeAck | null, reached: boolean, elapsedMs: number): ModeOutcome =>
  reached
    ? ModeOutcome.Settled
    : now !== null && now.serial !== before?.serial && !now.accepted
      ? ModeOutcome.Unsettled
      : elapsedMs >= MODE_REPLY_MS
        ? ModeOutcome.Unsettled
        : ModeOutcome.Pending;

export const pinsAfter = (quick: readonly string[], mode: string, pin: boolean): string => [...quick.filter((it) => it !== mode), ...(pin ? [mode] : [])].join(',');

export const startsSection = (shown: readonly FlightModeOption[], index: number): boolean => index > 0 && shown[index - 1]?.section !== shown[index]?.section;

export const modeHeading = (modes: FlightModesView | null): string | null => {
  const summary = modes === null ? null : blankToNull(modes.currentSummary);
  if (modes === null || summary === null) return null;
  const name = blankToNull(modes.current);
  return name === null ? null : `${name} — ${summary}`;
};
