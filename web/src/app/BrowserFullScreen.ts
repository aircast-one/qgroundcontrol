import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useSyncExternalStore } from 'react';

const desktop: boolean = import.meta.env.DESKTOP === true;

const windowState = { full: false, listeners: new Set<() => void>() };

const adopt = (full: boolean) => {
  if (full === windowState.full) return;
  windowState.full = full;
  windowState.listeners.forEach((changed) => changed());
};

const subscribe = (changed: () => void) => {
  if (!desktop) {
    document.addEventListener('fullscreenchange', changed);
    return () => document.removeEventListener('fullscreenchange', changed);
  }
  windowState.listeners.add(changed);
  const stop = listen<boolean>('full-screen', ({ payload }) => adopt(payload));
  void invoke<boolean>('full_screen').then(adopt);
  return () => {
    windowState.listeners.delete(changed);
    void stop.then((unlisten) => unlisten());
  };
};

const snapshot = (): boolean => (desktop ? windowState.full : document.fullscreenElement !== null);

export const useBrowserFullScreen = (): boolean => useSyncExternalStore(subscribe, snapshot, () => false);

export const browserFullScreenSupported = (): boolean => desktop || document.fullscreenEnabled;

export const toggleBrowserFullScreen = (): Promise<void> => {
  if (desktop) return invoke<boolean>('toggle_full_screen').then(adopt);
  return document.fullscreenElement === null ? document.documentElement.requestFullscreen({ navigationUI: 'hide' }) : document.exitFullscreen();
};
