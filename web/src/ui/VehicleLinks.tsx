import type { Json } from '@/bridge/Json';

export const VEHICLE_LINKS = 'view.vehicleLinks';

export type LossFailsafe = { action: string; after: number | null };

export type VehicleLink = { commLost: boolean | null };

export type VehicleLinks = { available: boolean; links: readonly VehicleLink[]; contactLost: boolean | null; failsafe: LossFailsafe | null };

export type LinkCell = { text: string; degraded: boolean };

export const vehicleLinks = (_view: Json | null): VehicleLinks | null => null;

export const linkCell = (links: VehicleLinks | null): LinkCell | null => {
  if (links === null || !links.available || links.links.length < 2) return null;
  const silent = links.links.filter((it) => it.commLost === true);
  return silent.length === 0
    ? { text: `${links.links.length} links`, degraded: false }
    : silent.length === links.links.length
      ? { text: 'no link heard', degraded: true }
      : silent.length === 1
        ? { text: '1 link lost', degraded: true }
        : { text: `${silent.length} links lost`, degraded: true };
};

export const linkNames = (_view: Json | null): readonly string[] => [];
