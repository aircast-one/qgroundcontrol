import { describe, expect, it } from 'vitest';
import { Tab } from '@/app/MainActivity';
import { SettingsGroup } from '@/ui/SettingsScreen';
import { openingGroup } from './SettingsSheet';

describe('SettingsSheetTest', () => {
  it('the sheet opens on the tab that holds the asked-for page, or Safety', () => {
    expect(openingGroup('Connections')).toBe(SettingsGroup.Transmission);
    expect(openingGroup('Video')).toBe(SettingsGroup.Camera);
    expect(openingGroup(null)).toBe(SettingsGroup.General);
    expect(Tab.from('settings')).toBeNull();
    expect(Tab.from('setup')).toBeNull();
  });
});
