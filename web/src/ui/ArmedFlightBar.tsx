import { useMemo } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Surface, Text, TextButton } from '@/m3';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { LAND_FROM, landFrom } from '@/ui/ConfirmTrack';
import { FLY_STATE, type FlyState, flyState } from '@/ui/FlyState';
import { BATTERY_VIEW } from '@/ui/InstrumentDetail';
import { type BatteryReading, BatteryLevel, batteryPercent, batteryReadings, compactStatusText } from '@/ui/StatusStrip';
import { VEHICLE_LINKS, type VehicleLinks, linkCell, vehicleLinks } from '@/ui/VehicleLinks';

export type FlightGlance = { mode: string; altitude: string | null; battery: string | null; batteryLow: boolean; link: string; linkLost: boolean };

const lowestBattery = (batteries: readonly BatteryReading[]): BatteryReading | null =>
  batteries.reduce<BatteryReading | null>((lowest, it) => (lowest === null || (batteryPercent(it.text) ?? 101) < (batteryPercent(lowest.text) ?? 101) ? it : lowest), null);

export const flightGlance = (state: FlyState | null, altitude: readonly [string, string] | null, batteries: readonly BatteryReading[], links: VehicleLinks | null): FlightGlance | null => {
  if (state === null || !state.connected || !state.armed) return null;
  const battery = lowestBattery(batteries);
  const linkLost = state.contactLost === true || links?.contactLost === true;
  return {
    mode: state.mode,
    altitude: altitude === null ? null : altitude.join(' ').trim(),
    battery: battery === null ? null : compactStatusText(battery.text) || battery.text,
    batteryLow: battery !== null && battery.level.ordinal >= BatteryLevel.Warning.ordinal,
    link: linkLost ? 'Lost' : (linkCell(links)?.text ?? 'OK'),
    linkLost,
  };
};

const Reading = ({ label, value, alert = false }: { label: string; value: string; alert?: boolean }) => (
  <span aria-label={`${label} ${value}`} className="flex items-baseline gap-1 whitespace-nowrap">
    <Text typography="labelSmall" color={md('onSurfaceVariant')}>
      {label}
    </Text>
    <Text typography="labelLarge" color={alert ? md('error') : md('onSurface')} className="tabular-nums">
      {value}
    </Text>
  </span>
);

export const ArmedFlightBar = ({ onFly }: { onFly: () => void }) => {
  const stateJson = useQgcPath(FLY_STATE);
  const altitudeJson = useQgcPath(LAND_FROM);
  const batteryJson = useQgcPath(BATTERY_VIEW);
  const linksJson = useQgcPath(VEHICLE_LINKS);
  const glance = useMemo(
    () => flightGlance(flyState(stateJson), landFrom(altitudeJson), batteryReadings(batteryJson), vehicleLinks(linksJson)),
    [stateJson, altitudeJson, batteryJson, linksJson],
  );
  if (glance === null) return null;
  return (
    <Surface color={md('surfaceContainerHigh')} contentColor={md('onSurface')} className="shrink-0">
      <div role="status" aria-label="Flight status" className="flex min-h-12 items-center gap-4 overflow-x-auto px-4 py-1">
        <span className="flex items-center gap-2 whitespace-nowrap">
          <Drawable name="ic_flight" size={20} />
          <Text typography="titleSmall">{glance.mode}</Text>
        </span>
        {glance.altitude !== null ? <Reading label="Alt" value={glance.altitude} /> : null}
        {glance.battery !== null ? <Reading label="Battery" value={glance.battery} alert={glance.batteryLow} /> : null}
        <Reading label="Link" value={glance.link} alert={glance.linkLost} />
        <span className="flex-1" />
        <TextButton onClick={onFly}>Back to Fly</TextButton>
      </div>
    </Surface>
  );
};
