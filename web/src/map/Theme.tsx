import { type CSSProperties, type ReactNode, createContext, useEffect, useSyncExternalStore } from 'react';
import { AircastShapes as SharedShapes, AircastSpace as SharedSpace } from '@/theme/Colors';

export type AircastColors = {
  success: string;
  onSuccess: string;
  successContainer: string;
  warning: string;
  onWarning: string;
  warningContainer: string;
  mission: string;
  alert: string;
  mapLand: string;
  mapWater: string;
  mapRoad: string;
  outdoorBackground: string;
  outdoorForeground: string;
  outdoorAccent: string;
  onOutdoorAccent: string;
};

export const AircastDarkColors: AircastColors = {
  success: '#40CC6D',
  onSuccess: '#0D0E11',
  successContainer: '#1D3823',
  warning: '#F4A437',
  onWarning: '#0D0E11',
  warningContainer: '#402F1A',
  mission: '#FFB870',
  alert: '#FFB870',
  mapLand: '#1B2A24',
  mapWater: '#12263A',
  mapRoad: '#3A4A44',
  outdoorBackground: '#000000',
  outdoorForeground: '#FFFFFF',
  outdoorAccent: '#FFD60A',
  onOutdoorAccent: '#000000',
};

export const AircastLightColors: AircastColors = {
  success: '#1D7D3E',
  onSuccess: '#FFFFFF',
  successContainer: '#C9DECC',
  warning: '#B75F0B',
  onWarning: '#FFFFFF',
  warningContainer: '#F0D7C7',
  mission: '#B45F00',
  alert: '#B45F00',
  mapLand: '#DDE8DC',
  mapWater: '#B9D7F0',
  mapRoad: '#FFFFFF',
  outdoorBackground: '#000000',
  outdoorForeground: '#FFFFFF',
  outdoorAccent: '#FFD60A',
  onOutdoorAccent: '#000000',
};

export const AircastShapes = SharedShapes;

type TypographyRole =
  | 'displayLarge' | 'displayMedium' | 'displaySmall'
  | 'headlineLarge' | 'headlineMedium' | 'headlineSmall'
  | 'titleLarge' | 'titleMedium' | 'titleSmall'
  | 'bodyLarge' | 'bodyMedium' | 'bodySmall'
  | 'labelLarge' | 'labelMedium' | 'labelSmall';

const style = (fontSize: number, lineHeight: number, fontWeight: number): CSSProperties => ({ fontSize, lineHeight: `${lineHeight}px`, fontWeight, letterSpacing: 0 });

export const AircastTypography: Readonly<Record<TypographyRole, CSSProperties>> = {
  displayLarge: style(57, 64, 400),
  displayMedium: style(45, 52, 400),
  displaySmall: style(36, 44, 400),
  headlineLarge: style(32, 40, 400),
  headlineMedium: style(28, 36, 400),
  headlineSmall: style(24, 32, 400),
  titleLarge: style(22, 28, 400),
  titleMedium: style(16, 24, 500),
  titleSmall: style(14, 20, 500),
  bodyLarge: style(16, 24, 400),
  bodyMedium: style(14, 20, 400),
  bodySmall: style(12, 16, 400),
  labelLarge: style(14, 20, 500),
  labelMedium: style(12, 16, 500),
  labelSmall: style(11, 16, 500),
};

export const TelemetryNumber: CSSProperties = {
  fontWeight: 500,
  fontSize: 22,
  lineHeight: '26.4px',
  fontVariantNumeric: 'tabular-nums',
};

export const TelemetryNumberMedium: CSSProperties = { ...TelemetryNumber, fontSize: 16, lineHeight: '20px' };

export const AircastSpace = SharedSpace;

const LocalAircastColors = createContext<AircastColors>(AircastDarkColors);

const DARK_QUERY = '(prefers-color-scheme: dark)';

const darkQuery = (): MediaQueryList | null => (typeof window === 'undefined' || !window.matchMedia ? null : window.matchMedia(DARK_QUERY));

const subscribeDark = (listener: () => void) => {
  const query = darkQuery();
  query?.addEventListener('change', listener);
  return () => query?.removeEventListener('change', listener);
};

export const useIsSystemInDarkTheme = (): boolean => useSyncExternalStore(subscribeDark, () => darkQuery()?.matches ?? false, () => false);

export const AircastTheme = ({ dark, children }: { dark?: boolean; children?: ReactNode }) => {
  const system = useIsSystemInDarkTheme();
  const isDark = dark ?? system;
  useEffect(() => {
    const root = document.documentElement;
    root.classList.toggle('dark', isDark);
    root.classList.toggle('light', !isDark);
  }, [isDark]);
  return <LocalAircastColors.Provider value={isDark ? AircastDarkColors : AircastLightColors}>{children}</LocalAircastColors.Provider>;
};
