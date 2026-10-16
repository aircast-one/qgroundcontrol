import type { Json } from '@/bridge/Json';
import type { AppNavigationState } from './AppNavigation';

export const OFFLINE_SHEET = 'offline';
export const OFFLINE_STATUS_VIEW = 'view.offlineStatus';

const CONNECTIONS_SETTINGS_PAGE = 'Connections';

export type OfflineLink = { index: number; text: string; description: string; retry: boolean; failed: boolean; connected: boolean };

export type OfflineStatus = { title: string; footnote: string; busy: boolean; noLinks: boolean; editAddress: boolean; links: readonly OfflineLink[] };

export const offlineStatus = (_view: Json | null): OfflineStatus | null => null;

export const openAddLink = (navigation: AppNavigationState): void => {
  navigation.addLinkRequested.value = true;
  navigation.settingsPage.value = CONNECTIONS_SETTINGS_PAGE;
};

export const OfflineStatusSheet = (_props: { onDismiss: () => void }) => null;
