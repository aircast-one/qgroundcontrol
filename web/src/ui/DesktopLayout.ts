import { useSyncExternalStore } from 'react';

export const LIST_DETAIL_MIN_WIDTH = 840;

export const DESKTOP_QUERY = `(min-width: ${LIST_DETAIL_MIN_WIDTH}px) and (pointer: fine)`;

export const LIST_PANE_WIDTH = 'clamp(380px, 34%, 520px)';
export const DETAIL_PANE_MAX_WIDTH = 720;
export const SETTINGS_PANE_MAX_WIDTH = 880;
export const DASHBOARD_MAX_WIDTH = 1200;

const desktopQuery = () => window.matchMedia(DESKTOP_QUERY);

const subscribeDesktop = (listener: () => void) => {
  const query = desktopQuery();
  query.addEventListener('change', listener);
  return () => query.removeEventListener('change', listener);
};

export const useDesktopLayout = (): boolean => useSyncExternalStore(subscribeDesktop, () => desktopQuery().matches, () => false);
