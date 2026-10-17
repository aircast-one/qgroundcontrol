import { useSyncExternalStore } from 'react';

export const DESKTOP_QUERY = '(min-width: 1200px) and (pointer: fine)';

const desktopQuery = () => window.matchMedia(DESKTOP_QUERY);

const subscribeDesktop = (listener: () => void) => {
  const query = desktopQuery();
  query.addEventListener('change', listener);
  return () => query.removeEventListener('change', listener);
};

export const useDesktopLayout = (): boolean => useSyncExternalStore(subscribeDesktop, () => desktopQuery().matches, () => false);
