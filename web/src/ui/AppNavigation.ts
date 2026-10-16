import { createContext, useContext, useEffect } from 'react';
import { mutableStateOf } from '@/kotlin/State';
import type { SettingsGroup } from '@/ui/SettingsScreen';

export class AppNavigationState {
  readonly settingsPage = mutableStateOf<string | null>(null);
  readonly settingsOpen = mutableStateOf(false);
  readonly aircraftRequested = mutableStateOf(false);
  readonly settingsShowing = mutableStateOf<SettingsGroup | null>(null);

  openAircraft(): void {
    this.aircraftRequested.value = true;
    this.settingsOpen.value = true;
  }

  readonly setupPage = mutableStateOf<string | null>(null);
  readonly parametersSearch = mutableStateOf('');
  readonly addLinkRequested = mutableStateOf(false);
  readonly blockedReason = mutableStateOf<string | null>(null);
}

export class OpenPlanDocument {
  readonly document = mutableStateOf<File | null>(null);
  readonly name = mutableStateOf<string | null>(null);
}

const unprovided = <T extends object>(message: string): T =>
  new Proxy({} as T, {
    get: () => {
      throw new Error(message);
    },
  });

export const LocalAppNavigation = createContext<AppNavigationState>(unprovided('AppNavigationState is provided by MainActivity'));

export const LocalOpenPlan = createContext<OpenPlanDocument>(unprovided('OpenPlanDocument is provided by MainActivity'));

export const LOG_DOWNLOAD_BLOCK = 'Wait for the log download to complete or cancel it first';
export const CALIBRATION_BLOCK = 'Complete or cancel the current calibration first';

export const navigationRefusal = (blockedReason: string | null, leaving: boolean): string | null => (leaving ? blockedReason : null);

export const BlocksNavigation = ({ blocking, reason }: { blocking: boolean; reason: string }) => {
  const navigation = useContext(LocalAppNavigation);
  useEffect(() => {
    if (blocking) navigation.blockedReason.value = reason;
    return () => {
      if (navigation.blockedReason.value === reason) navigation.blockedReason.value = null;
    };
  }, [blocking]);
  return null;
};
