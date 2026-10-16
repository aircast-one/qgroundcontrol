import { type Json, isNull, jsonObjects, optBoolean, optDouble, optJSONArray, optText } from '@/bridge/Json';

export const VEHICLE_LINKS = 'view.vehicleLinks';

export type VehicleLink = { commLost: boolean | null };

export const VehicleLink = (commLost: boolean | null): VehicleLink => ({ commLost });

export type LossFailsafe = { action: string; after: number | null };

export const LossFailsafe = (action: string, after: number | null): LossFailsafe => ({ action, after });

export type VehicleLinks = {
  available: boolean;
  links: readonly VehicleLink[];
  contactLost: boolean | null;
  failsafe: LossFailsafe | null;
};

export const VehicleLinks = (fields: { available: boolean; links: readonly VehicleLink[]; contactLost?: boolean | null; failsafe?: LossFailsafe | null }): VehicleLinks => ({
  contactLost: null,
  failsafe: null,
  ...fields,
});

export const vehicleLinks = (view: Json | null): VehicleLinks | null => {
  if (view === null || optText(view, 'class') !== 'VehicleLinks') return null;
  const action = optText(view, 'lossAction');
  return VehicleLinks({
    available: optBoolean(view, 'available'),
    contactLost: isNull(view, 'contactLost') ? null : optBoolean(view, 'contactLost'),
    failsafe: action === '' ? null : LossFailsafe(action, isNull(view, 'lossAfter') ? null : optDouble(view, 'lossAfter')),
    links: jsonObjects(optJSONArray(view, 'links')).map((link) => VehicleLink(isNull(link, 'commLost') ? null : optBoolean(link, 'commLost'))),
  });
};

export type LinkCell = { text: string; degraded: boolean };

export const LinkCell = (text: string, degraded: boolean): LinkCell => ({ text, degraded });

export const linkCell = (links: VehicleLinks | null): LinkCell | null => {
  if (links === null || !links.available || links.links.length < 2) return null;
  const silent = links.links.filter((it) => it.commLost === true);
  if (silent.length === 0) return LinkCell(`${links.links.length} links`, false);
  if (silent.length === links.links.length) return LinkCell('no link heard', true);
  if (silent.length === 1) return LinkCell('1 link lost', true);
  return LinkCell(`${silent.length} links lost`, true);
};

export const linkNames = (view: Json | null): readonly string[] => jsonObjects(optJSONArray(view, 'links')).map((link) => optText(link, 'name'));
