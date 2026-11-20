import { useMemo } from 'react';
import { type Json, isNull, optBoolean, optDouble, optInt, optJSONObject, optText } from '@/bridge/Json';
import { useQgcDerived } from '@/bridge/QgcState';
import { sentenceCase } from '@/map/SentenceCase';
import { type PayloadProblem, payloadProblem } from '@/ui/GuidedActions';
import { useRedIndicators, withRedIndicators } from '@/ui/OsdDetails';
import { flightTimeText } from '@/ui/StatusStrip';

export const FLY_STATE = 'view.flyState';

export type FlyState = {
  connected: boolean;
  armed: boolean;
  contactLost: boolean | null;
  state: string;
  stateText: string;
  staleNotice: string;
  mode: string;
  rcSupported: boolean;
  rcRequired: boolean;
  rcSignalText: string;
  rcSignal: number | null;
  rcOverride: boolean | null;
  telemetry: TelemetryLink | null;
  summaryDetail: string;
  summaryProblem: PayloadProblem | null;
  nominal: boolean;
  fault: boolean;
  canArm: boolean;
  readyToFly: boolean;
};

export const FlyState = (fields: Omit<FlyState, 'summaryDetail' | 'summaryProblem' | 'nominal' | 'fault' | 'canArm' | 'readyToFly' | 'rcRequired'> & Partial<Pick<FlyState, 'summaryDetail' | 'summaryProblem' | 'nominal' | 'fault' | 'canArm' | 'readyToFly' | 'rcRequired'>>): FlyState => ({
  rcRequired: true,
  summaryDetail: '',
  summaryProblem: null,
  nominal: true,
  fault: false,
  canArm: true,
  readyToFly: true,
  ...fields,
});

export type ChipTone = { readonly name: 'Error' | 'Neutral' | 'Warning' | 'Success'; readonly ordinal: number };
export const ChipTone = {
  Error: { name: 'Error', ordinal: 0 },
  Neutral: { name: 'Neutral', ordinal: 1 },
  Warning: { name: 'Warning', ordinal: 2 },
  Success: { name: 'Success', ordinal: 3 },
} as const satisfies Record<string, ChipTone>;
export const ChipToneEntries: readonly ChipTone[] = [ChipTone.Error, ChipTone.Neutral, ChipTone.Warning, ChipTone.Success];

export const notReadyToFly = (state: FlyState | null): boolean =>
  state !== null && state.connected && !state.armed && state.contactLost !== true && !state.readyToFly;

export const chipTone = (state: FlyState | null, lost: boolean): ChipTone => {
  if (lost || state?.fault === true) return ChipTone.Error;
  if (state?.connected !== true) return ChipTone.Neutral;
  if (!state.nominal) return ChipTone.Warning;
  if (notReadyToFly(state)) return ChipTone.Neutral;
  return ChipTone.Success;
};

const blankToNull = (text: string): string | null => (text.trim() === '' ? null : text);

export const readinessWarning = (state: FlyState | null): string | null => {
  if (state === null || !state.connected || state.armed || state.contactLost === true || !(notReadyToFly(state) || !state.nominal)) return null;
  return blankToNull([blankToNull(state.stateText), blankToNull(state.summaryDetail)].filter((part): part is string => part !== null).join('. '));
};

export type Readiness = { text: string; blocks: boolean };
export const Readiness = (fields: Readiness): Readiness => ({ ...fields });

export const READINESS_BLOCKED = 'The vehicle will refuse to arm until this is fixed.';

export const guidedReadiness = (state: FlyState | null, red: readonly string[]): Readiness | null => {
  const text = readinessWarning(withRedIndicators(state, red));
  if (text === null) return null;
  const blocks = state?.canArm === false;
  return Readiness({ text: blocks ? `${text.endsWith('.') ? text.slice(0, -1) : text}. ${READINESS_BLOCKED}` : text, blocks });
};

export const useGuidedReadiness = (state: FlyState | null): Readiness | null => {
  const red = useRedIndicators(state);
  return useMemo(() => guidedReadiness(state, red), [state, red.join('|')]);
};

export const LANDED_AND_DISARMED = 'Landed and disarmed';
export const MOTORS_STOPPED = 'Motors stopped';

export type DisarmCause = { readonly killed: boolean; readonly lands: boolean };

export const LANDING: DisarmCause = { killed: false, lands: true };

export const disarmNotice = (wasArmed: boolean, flewWhileArmed: boolean, armedNow: boolean, connectedNow: boolean, cause: DisarmCause = LANDING): string | null =>
  wasArmed && !armedNow && connectedNow ? (cause.killed ? MOTORS_STOPPED : flewWhileArmed && cause.lands ? LANDED_AND_DISARMED : 'Disarmed') : null;

export type ArmedSession = { readonly vehicle: number | null; readonly armed: boolean; readonly flew: boolean };

export const NO_ARMED_SESSION: ArmedSession = { vehicle: null, armed: false, flew: false };

const IN_AIR_STATES: readonly string[] = ['flying', 'landing'];

const inAirState = (token: string): boolean => IN_AIR_STATES.includes(token);

export type ArmedSessionStep = { readonly session: ArmedSession; readonly notice: string | null; readonly landed: boolean; readonly started: boolean };

export const armedSessionStep = (held: ArmedSession, connected: boolean, vehicle: number | null, armed: boolean, state: string | null, cause: DisarmCause = LANDING): ArmedSessionStep => {
  if (!connected || (vehicle === null && held.vehicle !== null)) return { session: held, notice: null, landed: false, started: false };
  const same = held.vehicle === null || held.vehicle === vehicle;
  const was = same && held.armed;
  const flew = same && held.flew;
  return {
    session: { vehicle, armed, flew: armed && (flew || (state !== null && inAirState(state))) },
    notice: disarmNotice(was, flew, armed, true, cause),
    landed: flew,
    started: armed && !was,
  };
};

export const VEHICLE_FLIGHT_DISTANCE = 'vehicle.flightDistance';

export const flownDistanceText = (view: Json | null): string | null => {
  if (view === null || !(optDouble(view, 'value', 0) > 0)) return null;
  const value = blankToNull(optText(view, 'valueString'));
  return value === null ? null : `${value} ${optText(view, 'units')}`.trim();
};

export const landedSummary = (seconds: number | null, distance: string | null, batteryUsed: number | null): string =>
  [
    seconds !== null && seconds >= 1 ? flightTimeText(seconds) : null,
    distance === null ? null : blankToNull(distance),
    batteryUsed !== null && batteryUsed > 0 ? `${batteryUsed}% battery used` : null,
  ]
    .filter((part): part is string => part !== null)
    .join(' · ');

export type TelemetryLink = {
  localRssiDbm: number;
  remoteRssiDbm: number | null;
  localNoise: number | null;
  remoteNoise: number | null;
  receiveErrors: number | null;
  errorsFixed: number | null;
  txBuffer: number | null;
};

export const TelemetryLink = (fields: Omit<TelemetryLink, 'errorsFixed' | 'txBuffer'> & Partial<Pick<TelemetryLink, 'errorsFixed' | 'txBuffer'>>): TelemetryLink => ({
  errorsFixed: null,
  txBuffer: null,
  ...fields,
});

const intOrNull = (json: Json, key: string): number | null => (isNull(json, key) ? null : optInt(json, key));

const booleanOrNull = (json: Json, key: string): boolean | null => (isNull(json, key) ? null : optBoolean(json, key));

export const telemetryLink = (view: Json | null): TelemetryLink | null => {
  const radio = optJSONObject(view, 'telemetry');
  if (radio === null || isNull(radio, 'localRssiDbm')) return null;
  return TelemetryLink({
    localRssiDbm: optInt(radio, 'localRssiDbm'),
    remoteRssiDbm: intOrNull(radio, 'remoteRssiDbm'),
    localNoise: intOrNull(radio, 'localNoise'),
    remoteNoise: intOrNull(radio, 'remoteNoise'),
    receiveErrors: intOrNull(radio, 'receiveErrors'),
    errorsFixed: intOrNull(radio, 'errorsFixed'),
    txBuffer: intOrNull(radio, 'txBuffer'),
  });
};

export const flyState = (view: Json | null): FlyState | null => {
  if (view === null || optText(view, 'class') !== 'FlyState') return null;
  const problem = optJSONObject(view, 'summaryProblem');
  return FlyState({
    connected: optBoolean(view, 'connected'),
    armed: optBoolean(view, 'armed'),
    contactLost: booleanOrNull(view, 'contactLost'),
    state: optText(view, 'state'),
    stateText: optText(view, 'stateText').split(' · ').map(sentenceCase).join(' · '),
    summaryDetail: optText(view, 'summaryDetail'),
    summaryProblem: problem === null ? null : payloadProblem(problem),
    nominal: optBoolean(view, 'nominal', true),
    fault: optBoolean(view, 'fault'),
    canArm: optBoolean(view, 'canArm', true),
    readyToFly: optBoolean(view, 'readyToFly', true),
    staleNotice: optText(view, 'staleNotice'),
    mode: optText(view, 'mode'),
    rcSupported: optBoolean(view, 'rcSupported'),
    rcRequired: optBoolean(view, 'rcRequired', true),
    rcSignalText: optText(view, 'rcSignalText'),
    rcSignal: intOrNull(view, 'rcSignal'),
    rcOverride: booleanOrNull(view, 'rcOverride'),
    telemetry: telemetryLink(view),
  });
};

export const offlineMainStatus = (view: Json | null): string | null => {
  const status = view === null ? null : blankToNull(optText(view, 'mainStatus'));
  return status === null ? null : sentenceCase(status);
};

export const vehicleSubtitle = (state: FlyState | null, offline: string | null = null): string => {
  if (state === null || !state.connected) return offline ?? 'No vehicle';
  if (state.contactLost === true) return state.stateText;
  return [blankToNull(state.mode), state.stateText].filter((part): part is string => part !== null).join(' · ');
};

export const STALE_OPACITY = 0.45;

const staleNoticeOf = (json: Json | null): string => flyState(json)?.staleNotice.trim() ?? '';

export const useStaleNotice = (): string => useQgcDerived(FLY_STATE, staleNoticeOf);

const connectedOf = (json: Json | null): boolean => flyState(json)?.connected === true;

export const useHasVehicle = (): boolean => useQgcDerived(FLY_STATE, connectedOf);

const armedOf = (json: Json | null): boolean => flyState(json)?.armed === true;

export const useArmed = (): boolean => useQgcDerived(FLY_STATE, armedOf);

export const inAir = (state: FlyState | null): boolean => state !== null && (inAirState(state.state) || state.contactLost === true);

const inAirOf = (json: Json | null): boolean => inAir(flyState(json));

export const useInAir = (): boolean => useQgcDerived(FLY_STATE, inAirOf);
