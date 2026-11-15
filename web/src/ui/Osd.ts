import { createContext, useContext } from 'react';
import { Color, withAlpha } from '@/theme/Colors';

export const OSD_SCRIM = withAlpha(Color.Black, 0.55);

export const OSD_CONTROL_SIZE = 48;

export const LocalFlyOsd = createContext(false);

export const useOsdBackdrop = (color: string): string => (useContext(LocalFlyOsd) ? Color.Transparent : color);

export const useOsdTint = (color: string, osdColor: string): string => (useContext(LocalFlyOsd) ? osdColor : color);

export const osdShadow = (): string => 'drop-shadow-[0_0.5px_1.5px_rgb(0_0_0/0.6)]';
