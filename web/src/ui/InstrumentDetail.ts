import { type Json, arrayJSONObject, isNull, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optString, optText } from '@/bridge/Json';
import { format, replaceFirstCharUpper } from '@/kotlin/Std';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { silenceDuration } from '@/map/Silence';
import { BatteryLevel, batteryLevelOf } from './StatusStrip';
import type { LossFailsafe, VehicleLinks } from './VehicleLinks';

export const SEVERITY_SECONDARY = -1;

export type DetailRow = { label: string; value: string; severity: number };

export const DetailRow = (fields: { label: string; value: string; severity?: number }): DetailRow => ({ severity: 0, ...fields });

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
  drainPerMinute: number | null;
};

export const BatteryHeadline = (fields: {
  text: string;
  detail: string;
  severity: number;
  level?: BatteryLevel;
  margin?: string;
  index?: number;
  percent?: number | null;
  timeLeft?: string;
  reserve?: number;
  returnAt?: number | null;
  returnNow?: boolean;
  drainPerMinute?: number | null;
}): BatteryHeadline => ({
  level: BatteryLevel.Normal,
  margin: '',
  index: 0,
  percent: null,
  timeLeft: '',
  reserve: 0,
  returnAt: null,
  returnNow: false,
  drainPerMinute: null,
  ...fields,
});

const optNumber = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isFinite(value) ? value : null;
};

const availableView = (view: Json | null): Json | null => (view !== null && optBoolean(view, 'available') ? view : null);

export const batteryHeadline = (view: Json | null): BatteryHeadline | null => {
  const it = optJSONObject(availableView(view), 'headline');
  return it === null
    ? null
    : BatteryHeadline({
        text: optText(it, 'text'),
        detail: optText(it, 'detail'),
        severity: optInt(it, 'severity'),
        level: batteryLevelOf(optText(it, 'level')),
        margin: optText(it, 'margin'),
        index: optInt(it, 'index'),
        percent: optNumber(it, 'percent'),
        timeLeft: optText(it, 'timeLeft'),
        reserve: optDouble(it, 'reserve', 0),
        returnAt: optNumber(it, 'returnAt'),
        returnNow: optBoolean(it, 'returnNow'),
        drainPerMinute: optNumber(it, 'drainPerMinute'),
      });
};

export const BATTERY_VIEW = 'view.battery';

const LIMITING_PACK = 'lowest';

export const RETURN_NOW_SPOKEN = 'Battery needed to return home. Return now.';
export const RETURN_NOW_CLIP = 'battery_needed_to_return_home_return_now';

export const batteryDetail = (view: Json | null): readonly DetailRow[] => {
  const packs = optJSONArray(availableView(view), 'packs');
  if (packs === null) return [];
  const rowsOf = (index: number): Json[] => jsonObjects(optJSONArray(arrayJSONObject(packs, index), 'rows'));
  if (packs.length < 2) return rowsOf(0).map((it) => DetailRow({ label: optText(it, 'label'), value: optText(it, 'value'), severity: optInt(it, 'severity', SEVERITY_SECONDARY) }));
  const limiting = batteryHeadline(view)?.index ?? null;
  return packs.map((_, index) => {
    const rows = rowsOf(index);
    return DetailRow({
      label: [`Battery ${index + 1}`, ...(index === limiting ? [LIMITING_PACK] : [])].join(' · '),
      value: rows.map((it) => optText(it, 'value')).join(' · '),
      severity: rows.length === 0 ? 0 : Math.max(0, ...rows.map((it) => optInt(it, 'severity', SEVERITY_SECONDARY))),
    });
  });
};

export const silenceText = (seconds: number): string => `No data from the aircraft for ${silenceDuration(seconds)}`;

export const SIGNAL_LOST = 'Signal lost';

export const failsafeCountdown = (seconds: number | null, failsafe: LossFailsafe | null): string | null => {
  if (failsafe === null) return null;
  const counted = failsafe.after === null ? null : Math.ceil(failsafe.after - (seconds ?? 0));
  const left = counted !== null && counted > 0 ? counted : null;
  return left === null ? failsafe.action : `${failsafe.action} in ${silenceDuration(left)}`;
};

const LOST = 'Lost';

export const LOST_LINK_HINT = 'Point the antenna at the aircraft or move closer.';

export const signalLostTitle = (seconds: number | null, failsafe: LossFailsafe | null = null, compact: boolean = false): string =>
  [
    compact ? [LOST, ...(seconds === null ? [] : [silenceDuration(seconds)])].join(' ') : SIGNAL_LOST,
    seconds === null || compact ? null : silenceDuration(seconds),
    failsafeCountdown(seconds, failsafe),
  ]
    .filter((it): it is string => it !== null)
    .join(' · ');

const CRITICAL_CHARGE_STATES = { start: 3, endInclusive: 6 } as const;

export const batteryReturnOffered = (view: Json | null): boolean => {
  const packs = optJSONArray(availableView(view), 'packs');
  if (packs === null) return false;
  return packs.some((_, index) => {
    const pack = arrayJSONObject(packs, index);
    const state = pack === null ? null : optInt(pack, 'chargeState', 0);
    return state !== null && state >= CRITICAL_CHARGE_STATES.start && state <= CRITICAL_CHARGE_STATES.endInclusive;
  });
};

const packFactTotal = (view: Json | null, name: string): number | null => {
  const packs = optJSONArray(availableView(view), 'packs');
  if (packs === null) return null;
  const values = jsonObjects(packs).map((pack) => {
    const fact = jsonObjects(optJSONArray(pack, 'facts')).find((it) => optText(it, 'name') === name) ?? null;
    const value = fact === null ? NaN : optDouble(fact, 'value');
    return Number.isNaN(value) ? null : value;
  });
  const present = values.filter((it): it is number => it !== null);
  return present.length > 0 && present.length === values.length ? present.reduce((sum, it) => sum + it, 0) : null;
};

export const totalCurrent = (view: Json | null): string | null => {
  const total = packFactTotal(view, 'current');
  return total === null ? null : format('%.1fA', total);
};

export const totalDraw = (view: Json | null): string | null => {
  const total = packFactTotal(view, 'instantPower');
  return total === null ? totalCurrent(view) : `${Math.round(total)}W`;
};

export const factLabel = (name: string): string => {
  switch (name) {
    case 'voltage':
      return 'Voltage';
    case 'current':
      return 'Current';
    case 'instantPower':
      return 'Power';
    case 'mahConsumed':
      return 'Consumed';
    case 'timeRemainingStr':
      return 'Time remaining';
    case 'temperature':
      return 'Temperature';
    case 'percentRemaining':
      return 'Remaining';
    default:
      return replaceFirstCharUpper(name);
  }
};

export type GpsStatus = { satellites: number | null; lock: number; lockText: string; rows: readonly DetailRow[] };

export const GpsStatus = (fields: GpsStatus): GpsStatus => ({ ...fields });

export const gpsStatus = (view: Json | null): GpsStatus | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  return GpsStatus({
    satellites: isNull(view, 'satellites') ? null : optInt(view, 'satellites'),
    lock: isNull(view, 'lock') ? NaN : optDouble(view, 'lock', NaN),
    lockText: optText(view, 'lockText'),
    rows: jsonObjects(optJSONArray(view, 'rows'))
      .map((it) => DetailRow({ label: optString(it, 'label'), value: optString(it, 'value') }))
      .filter((it) => it.label.trim() !== '' && it.value.trim() !== ''),
  });
};

export const gpsDetail = (gps: GpsStatus | null): readonly DetailRow[] => [
  ...(gps !== null && gps.lockText.trim() !== '' ? [DetailRow({ label: 'GPS Lock', value: gps.lockText })] : []),
  ...(gps?.rows ?? []),
];

export const notYetComputed = (shown: string): boolean => shown.trim() === '' || [...shown].every((it) => it === '-' || it === ':' || it === '.' || it === ' ');

export const usableDop = (shown: string): boolean => {
  if (notYetComputed(shown)) return false;
  const dop = toDoubleOrNull(shown);
  return dop !== null && dop > 0 && dop < 100;
};

export const linkDetail = (links: VehicleLinks | null, names: readonly string[], primary: string | null): readonly DetailRow[] => {
  if (links === null || !links.available) return [];
  return names.map((name, index) => {
    const lost = links.links[index]?.commLost === true;
    const parts = [...(name === primary ? ['carrying'] : []), ...(lost ? ['no contact'] : [])];
    return DetailRow({ label: name, value: (parts.length === 0 ? ['standing by'] : parts).join(' · ') });
  });
};
