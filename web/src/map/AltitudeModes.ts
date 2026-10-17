import { type Json, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { PLAN_ITEMS } from './PlanBridge';

export const MISSION_CONTEXT = 'mission';
export const ITEM_CONTEXT = 'item';
export const ALT_MODE_MIXED = 0;
export const FRAME_UNKNOWN = 'Frame';

export type AltitudeModeOffer = { raw: number; title: string; help: string; enabled: boolean; current: boolean; reason: string };

export type AltitudeModesView = { context: string; current: number; offers: readonly AltitudeModeOffer[]; omitted: readonly AltitudeModeOffer[] };

export const altitudeModesPath = (context: string, current: number): string => `view.altitudeModes(${context},${current})`;

const offers = (view: Json | null, key: string): readonly AltitudeModeOffer[] =>
  jsonObjects(optJSONArray(view, key)).map((it) => ({
    raw: optInt(it, 'raw', -1),
    title: optText(it, 'title'),
    help: optText(it, 'help'),
    enabled: optBoolean(it, 'enabled'),
    current: optBoolean(it, 'current'),
    reason: optText(it, 'reason'),
  }));

export const altitudeModesView = (view: Json | null): AltitudeModesView | null => {
  if (view === null || optText(view, 'class') !== 'AltitudeModes') return null;
  return { context: optText(view, 'context'), current: optInt(view, 'current', -1), offers: offers(view, 'modes'), omitted: offers(view, 'omitted') };
};

export const choosable = (view: AltitudeModesView | null): readonly AltitudeModeOffer[] => (view?.offers ?? []).filter((it) => it.raw !== ALT_MODE_MIXED);

export const offersChoice = (view: AltitudeModesView | null): boolean => choosable(view).filter((it) => it.enabled).length > 1;

export const refusalFor = (view: AltitudeModesView | null, raw: number): string | null => {
  const offer = (view?.offers ?? []).find((it) => it.raw === raw);
  return offer === undefined || offer.enabled || offer.reason.trim() === '' ? null : offer.reason;
};

export const altitudeModePath = (index: number): string => `${PLAN_ITEMS}.${index}.altitudeMode`;
