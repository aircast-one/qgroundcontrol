import { type Json, optBoolean, optJSONObject, optLong } from '@/bridge/Json';

export const FLIGHT_MODES = 'view.flightModes';
export const MODE_REPLY_MS = 3000;

export type FlightModeOption = {
  name: string;
  summary: string;
  current: boolean;
  needsConfirm: boolean;
  section: string;
  quick: boolean;
  caution: string;
};

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

export type ModeAck = { serial: number; accepted: boolean };

export type ModeOutcome = { readonly kind: 'Pending' } | { readonly kind: 'Settled' } | { readonly kind: 'Unsettled' };

export const ModeOutcome = {
  Pending: { kind: 'Pending' },
  Settled: { kind: 'Settled' },
  Unsettled: { kind: 'Unsettled' },
} as const satisfies Record<string, ModeOutcome>;

export const flightModesView = (_view: Json | null): FlightModesView | null => null;

export const modeAck = (view: Json | null): ModeAck | null => {
  const ack = optJSONObject(view, 'modeAck');
  return ack === null ? null : { serial: optLong(ack, 'serial'), accepted: optBoolean(ack, 'accepted') };
};

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
  const summary = modes === null || modes.currentSummary.trim() === '' ? null : modes.currentSummary;
  if (summary === null || modes === null) return null;
  const name = modes.current.trim() === '' ? null : modes.current;
  return name === null ? null : `${name} — ${summary}`;
};
