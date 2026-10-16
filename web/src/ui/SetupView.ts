import { type Json, has, isNull, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';

export const SETUP = 'view.setup';
export const FIRMWARE = 'view.firmware';

export const parametersReady = (view: Json | null): boolean => optBoolean(view, 'connected') && optBoolean(view, 'parametersReady');

export type SetupPage = { name: string; parameterSections: boolean; screen: string };

export const SetupPage = (fields: { name: string; parameterSections: boolean; screen?: string }): SetupPage => ({ screen: '', ...fields });

export type SetupGroup = { title: string; pages: SetupPage[] };

export const SetupGroup = (fields: SetupGroup): SetupGroup => ({ ...fields });

export const setupGroups = (view: Json | null): SetupGroup[] =>
  jsonObjects(optJSONArray(view, 'groups')).map((group) =>
    SetupGroup({
      title: optText(group, 'title'),
      pages: jsonObjects(optJSONArray(group, 'pages')).map((it) =>
        SetupPage({ name: optText(it, 'name'), parameterSections: optBoolean(it, 'parameterSections'), screen: optText(it, 'screen') }),
      ),
    }),
  );

export type SetupReadiness = {
  ready: boolean | null;
  setupComplete: boolean | null;
  headline: string;
  detail: string;
  connected: boolean;
  firmware: string;
  vehicleId: number | null;
};

export const SetupReadiness = (fields: Omit<SetupReadiness, 'setupComplete' | 'vehicleId'> & { setupComplete?: boolean | null; vehicleId?: number | null }): SetupReadiness => ({
  setupComplete: null,
  vehicleId: null,
  ...fields,
});

export const NOTHING_TO_CONFIGURE = 'Nothing to Configure';
export const NOTHING_TO_CONFIGURE_TEXT = "Aircast doesn't support setup for this vehicle type. If it is already configured, you can still fly.";

export const setupReadiness = (view: Json | null): SetupReadiness | null =>
  view === null
    ? null
    : SetupReadiness({
        ready: isNull(view, 'ready') ? null : optBoolean(view, 'ready'),
        setupComplete: isNull(view, 'setupComplete') || !has(view, 'setupComplete') ? null : optBoolean(view, 'setupComplete'),
        headline: optText(view, 'headline'),
        detail: optText(view, 'detail'),
        connected: optBoolean(view, 'connected'),
        firmware: optText(view, 'firmware'),
        vehicleId: isNull(view, 'vehicleId') ? null : optInt(view, 'vehicleId'),
      });

export const isPx4 = (readiness: SetupReadiness | null): boolean => readiness?.firmware === 'px4';

export const setupPage = (view: Json | null, name: string): SetupPage | null =>
  setupGroups(view)
    .flatMap((it) => it.pages)
    .find((it) => it.name === name) ?? null;

export const setupPagePath = (name: string): string => `${SETUP}(${name})`;
