import { useSyncExternalStore } from 'react';

const subscribe = (changed: () => void) => {
  document.addEventListener('fullscreenchange', changed);
  return () => document.removeEventListener('fullscreenchange', changed);
};

export const useBrowserFullScreen = (): boolean => useSyncExternalStore(subscribe, () => document.fullscreenElement !== null, () => false);

export const browserFullScreenSupported = (): boolean => document.fullscreenEnabled;

export const toggleBrowserFullScreen = (): Promise<void> =>
  document.fullscreenElement === null ? document.documentElement.requestFullscreen({ navigationUI: 'hide' }) : document.exitFullscreen();
