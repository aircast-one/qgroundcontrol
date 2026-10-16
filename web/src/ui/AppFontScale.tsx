import { type ReactNode, useEffect, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { opt } from '@/bridge/Json';
import { useTicker } from '@/kotlin/Compose';

const APP_FONT_POINT_SIZE = 'settings.appSettings.appFontPointSize';
const PLATFORM_FONT_POINT_SIZE = 14;
const FONT_POINT_SIZE_MIN = 6;
const FONT_POINT_SIZE_MAX = 48;
const FONT_SIZE_POLL_MS = 1000;
const FONT_SCALE_PROPERTY = '--aircast-font-scale';

export const appFontScale = (pointSize: unknown): number =>
  typeof pointSize === 'number' && pointSize >= FONT_POINT_SIZE_MIN && pointSize <= FONT_POINT_SIZE_MAX ? pointSize / PLATFORM_FONT_POINT_SIZE : 1;

export const AppFontScale = ({ children }: { children?: ReactNode }) => {
  const [scale, setScale] = useState(1);
  useTicker(FONT_SIZE_POLL_MS, [], async () => setScale(appFontScale(opt(await Qgc.get(APP_FONT_POINT_SIZE), 'value'))));
  useEffect(() => {
    const root = document.documentElement.style;
    root.setProperty(FONT_SCALE_PROPERTY, String(scale));
    return () => {
      root.removeProperty(FONT_SCALE_PROPERTY);
    };
  }, [scale]);
  return <>{children}</>;
};
