import { getSharedPreferences } from '@/kotlin/Platform';
import { mutableStateOf, useValue } from '@/kotlin/State';

const STORE = 'fly-map-3d';
const SHOWN = 'shown';
const HINTED = 'hinted';

const shown = mutableStateOf(getSharedPreferences(STORE).getBoolean(SHOWN, false));

export const Map3d = {
  show(on: boolean): void {
    shown.value = on;
    getSharedPreferences(STORE).putBoolean(SHOWN, on);
  },
  hinted(): boolean {
    return getSharedPreferences(STORE).getBoolean(HINTED, false);
  },
  markHinted(): void {
    getSharedPreferences(STORE).putBoolean(HINTED, true);
  },
};

export const useMap3dShown = (): boolean => useValue(shown);
