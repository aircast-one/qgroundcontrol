import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { type Json, optBoolean } from '@/bridge/Json';

export const ADVANCED_UI_PATH = 'view.advancedUi';
export const SET_ADVANCED_UI = 'advancedUi.set';

export const advancedUiShown = (view: Json | null): boolean => (view === null ? true : optBoolean(view, 'shown', true));

export const useAdvancedUiShown = (): boolean => advancedUiShown(useQgcPath(ADVANCED_UI_PATH));

export const toggleAdvancedUi = (shown: boolean): void => {
  void Qgc.invoke(SET_ADVANCED_UI, !shown);
};
