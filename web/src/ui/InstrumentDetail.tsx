import type { Json } from '@/bridge/Json';
import { silenceDuration } from '@/map/Silence';
import type { BatteryLevel } from './StatusStrip';
import type { LossFailsafe, VehicleLinks } from './VehicleLinks';

export const SIGNAL_LOST = 'Signal lost';
export const LOST_LINK_HINT = 'Point the antenna at the aircraft or move closer.';
export const BATTERY_VIEW = 'view.battery';
export const RETURN_NOW_SPOKEN = 'Battery needed to return home. Return now.';
export const RETURN_NOW_CLIP = 'battery_needed_to_return_home_return_now';
export const SEVERITY_SECONDARY = -1;

const LOST = 'Lost';

export const failsafeCountdown = (seconds: number | null, failsafe: LossFailsafe | null): string | null => {
  if (failsafe === null) return null;
  const left = failsafe.after === null ? null : Math.ceil(failsafe.after - (seconds ?? 0));
  return left === null || left <= 0 ? failsafe.action : `${failsafe.action} in ${silenceDuration(left)}`;
};

export const signalLostTitle = (seconds: number | null, failsafe: LossFailsafe | null = null, compact: boolean = false): string =>
  [
    compact ? [LOST, ...(seconds === null ? [] : [silenceDuration(seconds)])].join(' ') : SIGNAL_LOST,
    seconds === null || compact ? null : silenceDuration(seconds),
    failsafeCountdown(seconds, failsafe),
  ]
    .filter((it): it is string => it !== null)
    .join(' · ');

export const silenceText = (seconds: number): string => `No data from the aircraft for ${silenceDuration(seconds)}`;

export type DetailRow = { label: string; value: string; severity: number };

export const DetailRow = (label: string, value: string, severity: number = 0): DetailRow => ({ label, value, severity });

export type BatteryHeadline = {
  text: string;
  detail: string;
  severity: number;
  level: BatteryLevel;
  margin: string;
  index: number;
  percent: number | null;
  timeLeft: string;
  reserve: number;
  returnAt: number | null;
  returnNow: boolean;
};

export type GpsStatus = { satellites: number | null; lock: number; lockText: string; rows: readonly DetailRow[] };

export const batteryHeadline = (_view: Json | null): BatteryHeadline | null => null;

export const batteryDetail = (_view: Json | null): readonly DetailRow[] => [];

export const batteryReturnOffered = (_view: Json | null): boolean => false;

export const gpsStatus = (_view: Json | null): GpsStatus | null => null;

export const gpsDetail = (gps: GpsStatus | null): readonly DetailRow[] => [
  ...(gps !== null && gps.lockText.trim() !== '' ? [DetailRow('GPS Lock', gps.lockText)] : []),
  ...(gps?.rows ?? []),
];

export const linkDetail = (_links: VehicleLinks | null, _names: readonly string[], _primary: string | null): readonly DetailRow[] => [];

export const totalCurrent = (_view: Json | null): string | null => null;

export const totalDraw = (_view: Json | null): string | null => null;
