import { type Json, jsonObjects, optBoolean, optJSONArray, optText } from '@/bridge/Json';

export const FOLLOW_ME_VIEW = 'view.followMe';

export type FollowMeReading = {
  readonly mode: string;
  readonly enabled: boolean;
  readonly wouldSend: boolean;
  readonly reason: string;
  readonly following: number;
  readonly vehicles: number;
};

export const followMeReading = (view: Json | null): FollowMeReading | null => {
  if (view === null || optText(view, 'class') !== 'FollowMe') return null;
  const vehicles = jsonObjects(optJSONArray(view, 'vehicles'));
  return {
    mode: optText(view, 'mode'),
    enabled: optBoolean(view, 'enabled'),
    wouldSend: optBoolean(view, 'wouldSend'),
    reason: optText(view, 'reason'),
    following: vehicles.filter((it) => optBoolean(it, 'following')).length,
    vehicles: vehicles.length,
  };
};

const TROUBLES: ReadonlyMap<string, string> = new Map([
  ['modeUnknown', 'the Follow Me setting is not one this app understands'],
  ['modeNever', 'Follow Me is switched off'],
  ['noVehicles', 'no vehicle is connected'],
  ['noVehicleInFollowMode', 'no vehicle is in Follow Me mode'],
  ['noFix', 'this phone has no position yet'],
  ['fixInvalid', "this phone's position is not valid"],
  ['fixStale', "this phone's position has stopped updating"],
  ['fixUnusable', "this phone's position cannot be used"],
  ['allVehiclesRefused', 'the vehicle refused the position'],
]);

export const followMeTrouble = (reason: string): string => TROUBLES.get(reason) ?? 'the position is not being sent';

export const followMeAsked = (mode: string): boolean => mode === 'always' || mode === 'followMe';

export const followMeResting = (reason: string): boolean => reason === 'noVehicleInFollowMode';

export const followMeLabel = (reading: FollowMeReading | null): string | null => {
  if (reading === null || !followMeAsked(reading.mode) || reading.vehicles === 0) return null;
  if (followMeResting(reading.reason)) return null;
  if (reading.wouldSend) return reading.following <= 1 ? 'Following you' : `Following you · ${reading.following} vehicles`;
  return `Not following you — ${followMeTrouble(reading.reason)}`;
};
