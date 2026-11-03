import { getSharedPreferences } from '@/kotlin/Platform';
import { mutableStateOf, useValue } from '@/kotlin/State';

const STORE = 'fly-map-3d';
const SHOWN = 'shown';

const shown = mutableStateOf(getSharedPreferences(STORE).getBoolean(SHOWN, false));

export const Map3d = {
  show(on: boolean): void {
    shown.value = on;
    getSharedPreferences(STORE).putBoolean(SHOWN, on);
  },
};

export const useMap3dShown = (): boolean => useValue(shown);
