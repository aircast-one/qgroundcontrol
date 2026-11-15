import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useSyncExternalStore } from 'react';

const appWindow = { full: false };

const subscribeToWindow = (changed: () => void) => {
  const adopt = (full: boolean) => {
    appWindow.full = full;
    changed();
  };
  const stop = listen<boolean>('full-screen', ({ payload }) => adopt(payload));
  void invoke<boolean>('full_screen').then(adopt);
  return () => void stop.then((unlisten) => unlisten());
};

const subscribeToDocument = (changed: () => void) => {
  document.addEventListener('fullscreenchange', changed);
  return () => document.removeEventListener('fullscreenchange', changed);
};

const subscribe = __DESKTOP__ ? subscribeToWindow : subscribeToDocument;

const snapshot = (): boolean => (__DESKTOP__ ? appWindow.full : document.fullscreenElement !== null);

export const useBrowserFullScreen = (): boolean => useSyncExternalStore(subscribe, snapshot, () => false);

export const browserFullScreenSupported = (): boolean => __DESKTOP__ || document.fullscreenEnabled;

export const toggleBrowserFullScreen = (): Promise<void> =>
  __DESKTOP__
    ? invoke('toggle_full_screen').then(() => undefined)
    : document.fullscreenElement === null
      ? document.documentElement.requestFullscreen({ navigationUI: 'hide' })
      : document.exitFullscreen();
