import { type Fact, Qgc, makeFact } from '@/bridge/Qgc';
import { settingControl, useQgcPath } from '@/bridge/QgcState';
import { type Json, isNull, opt, optBoolean } from '@/bridge/Json';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { getSharedPreferences } from '@/kotlin/Platform';
import { useIsSystemInDarkTheme } from '@/map/Theme';

export const PALETTE_SETTING = 'settings.appSettings.indoorPalette';
export const PALETTE_INDOOR = 1;
export const PALETTE_OUTDOOR = 0;
const PALETTE_STORE = 'appearance';
const PALETTE_DEFAULTED = 'paletteDefaultedToDark';

const PALETTE_NAMES: ReadonlyMap<string, string> = new Map([
  [`${PALETTE_INDOOR}`, 'Dark'],
  [`${PALETTE_OUTDOOR}`, 'Light'],
]);

export const paletteNamed = (fact: Fact): Fact =>
  fact.path !== PALETTE_SETTING || fact.enumValues.length !== fact.enumStrings.length
    ? fact
    : makeFact({ ...fact, enumStrings: fact.enumValues.map((raw, index) => PALETTE_NAMES.get(raw) ?? fact.enumStrings[index] ?? '') });

export const paletteIsDark = (value: number | null, systemDark: boolean): boolean => {
  if (value === PALETTE_INDOOR) return true;
  if (value === PALETTE_OUTDOOR) return false;
  return value === null ? true : systemDark;
};

const changedFromDefault = (control: Json): boolean => isNull(control, 'changedFromDefault') || optBoolean(control, 'changedFromDefault');

export const shouldDefaultToDark = (changedFromDefault: boolean, alreadyDefaulted: boolean): boolean => !changedFromDefault && !alreadyDefaulted;

const intValue = (value: unknown): number | null => {
  if (typeof value === 'number') return Number.isFinite(value) ? Math.trunc(value) : null;
  if (value === null || value === undefined) return null;
  const text = String(value);
  return /^[+-]?\d+$/.test(text) ? Number(text) : null;
};

export const useAppDarkTheme = (): boolean => {
  const control = useQgcPath(settingControl(PALETTE_SETTING));
  const value = control === null ? null : intValue(opt(control, 'value'));
  useLaunchedEffect([control !== null], () => {
    if (control === null) return;
    const store = getSharedPreferences(PALETTE_STORE);
    if (shouldDefaultToDark(changedFromDefault(control), store.getBoolean(PALETTE_DEFAULTED, false))) {
      void Qgc.set(PALETTE_SETTING, PALETTE_INDOOR);
    }
    store.putBoolean(PALETTE_DEFAULTED, true);
  });
  const pending = control !== null && shouldDefaultToDark(changedFromDefault(control), getSharedPreferences(PALETTE_STORE).getBoolean(PALETTE_DEFAULTED, false));
  return paletteIsDark(pending ? PALETTE_INDOOR : value, useIsSystemInDarkTheme());
};
