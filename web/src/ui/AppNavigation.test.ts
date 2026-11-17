import { describe, expect, it } from 'vitest';
import { SettingsGroup } from '@/ui/SettingsScreen';
import { AppNavigationState, CALIBRATION_BLOCK, LOG_DOWNLOAD_BLOCK, navigationRefusal } from './AppNavigation';

describe('NavigationBlockTest', () => {
  it("a set reason refuses leaving with QGC's words, like MainWindow allowViewSwitch", () => {
    expect(navigationRefusal(LOG_DOWNLOAD_BLOCK, true)).toBe(LOG_DOWNLOAD_BLOCK);
    expect(navigationRefusal(null, true)).toBeNull();
    expect(navigationRefusal(CALIBRATION_BLOCK, false)).toBeNull();
    expect(CALIBRATION_BLOCK).toBe('Complete or cancel the current calibration first');
  });
});

describe('opening a Settings tab is a navigation action of its own', () => {
  it('names the tab and opens Settings, without posing as a page', () => {
    const navigation = new AppNavigationState();
    navigation.openSettingsTab(SettingsGroup.Safety);
    expect(navigation.settingsTab.value).toBe(SettingsGroup.Safety);
    expect(navigation.settingsOpen.value).toBe(true);
    expect(navigation.settingsPage.value).toBeNull();
  });
});
