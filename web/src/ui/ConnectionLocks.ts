import { useEffect } from 'react';
import { type Json, optInt } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { keepScreenOn } from '@/app/VehicleLinkService';
import { VEHICLES_VIEW } from '@/map/VehiclePicker';
import { linkRows } from '@/ui/LinksScreen';

export const vehicleConnected = (view: Json | null): boolean => optInt(view, 'count') > 0;

export const linkAwaitingVehicle = (view: Json | null): boolean => linkRows(view).some((it: { connected: boolean }) => it.connected);

export const ConnectionLocks = () => {
  const connected = vehicleConnected(useQgcPath(VEHICLES_VIEW));
  useEffect(() => (connected ? keepScreenOn() : undefined), [connected]);
  return null;
};
