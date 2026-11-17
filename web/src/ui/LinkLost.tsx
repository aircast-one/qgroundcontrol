import { useEffect, useRef, useState } from 'react';
import type { Json } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { useValue } from '@/kotlin/State';
import { Surface, Text, TextButton } from '@/m3';
import { LAST_SEEN, LINK_LOST, type LostVehicle, keptSighting, lostVehicle } from '@/map/LostVehicle';
import { isPlottable } from '@/map/VehicleMap';
import { VEHICLES_VIEW, vehicleChoices } from '@/map/VehiclePicker';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { useHasVehicle } from '@/ui/FlyState';
import { BATTERY_VIEW, LOST_LINK_HINT } from '@/ui/InstrumentDetail';
import { batteryPercent, batteryReadings } from '@/ui/StatusStrip';
import { VEHICLE_LINKS, vehicleLinks } from '@/ui/VehicleLinks';

const COORDINATE_DECIMALS = 5;

const batteryOf = (json: Json | null): number | null => {
  const first = batteryReadings(json)[0];
  return first === undefined ? null : batteryPercent(first.text);
};

export const lostPlace = (lost: LostVehicle): string =>
  [
    isPlottable(lost.at.latitude, lost.at.longitude) ? `${LAST_SEEN} at ${lost.at.latitude.toFixed(COORDINATE_DECIMALS)}, ${lost.at.longitude.toFixed(COORDINATE_DECIMALS)}` : `${LAST_SEEN} position unknown`,
    lost.altitude === null || lost.altitude.trim() === '' ? null : `${lost.altitude.trim()} up`,
    lost.battery === null ? null : `${lost.battery}% battery`,
  ]
    .filter((it): it is string => it !== null)
    .join(' · ');

export const lostFailsafe = (lost: LostVehicle): string => `Failsafe on link loss: ${lost.failsafe === null ? 'unknown' : lost.failsafe.action}.`;

export const LinkLostWatch = () => {
  const connected = useHasVehicle();
  const vehicles = useQgcPath(VEHICLES_VIEW);
  const battery = useQgcPath(BATTERY_VIEW);
  const links = useQgcPath(VEHICLE_LINKS);
  const sighting = useRef<LostVehicle | null>(null);
  useEffect(() => {
    if (connected) {
      sighting.current = keptSighting(sighting.current, vehicleChoices(vehicles).active, batteryOf(battery), vehicleLinks(links)?.failsafe ?? null);
      lostVehicle.value = null;
      return;
    }
    if (sighting.current === null) return;
    lostVehicle.value = sighting.current;
    sighting.current = null;
  }, [connected, vehicles, battery, links]);
  return null;
};

export const LinkLostCard = ({ className }: { className?: string }) => {
  const lost = useValue(lostVehicle);
  const [hidden, setHidden] = useState<LostVehicle | null>(null);
  if (lost === null || hidden === lost) return null;
  return (
    <div role="alert" className={`w-full ${className ?? ''}`}>
      <Surface shape={12} color={md('errorContainer')} contentColor={md('onErrorContainer')}>
        <div className="flex items-start gap-3 px-3 py-2">
          <Drawable name="ic_warning" size={24} className="mt-1 shrink-0" style={{ color: md('error') }} />
          <div className="flex min-w-0 flex-1 flex-col gap-0.5">
            <Text typography="titleSmall">{`${LINK_LOST} · ${lost.name}`}</Text>
            <Text typography="bodyMedium">{lostPlace(lost)}</Text>
            <Text typography="bodyMedium">{`${lostFailsafe(lost)} ${LOST_LINK_HINT}`}</Text>
          </div>
          <TextButton onClick={() => setHidden(lost)}>Hide</TextButton>
        </div>
      </Surface>
    </div>
  );
};
