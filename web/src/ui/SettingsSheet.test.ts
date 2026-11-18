import { describe, expect, it } from 'vitest';
import { Tab } from '@/app/MainActivity';
import { SettingsGroup } from '@/ui/SettingsScreen';
import { openingGroup, scrollMoves } from './SettingsSheet';

describe('SettingsSheetTest', () => {
  it('the sheet opens on the tab that holds the asked-for page, or Safety', () => {
    expect(openingGroup('Connections')).toBe(SettingsGroup.Transmission);
    expect(openingGroup('Video')).toBe(SettingsGroup.Camera);
    expect(openingGroup(null)).toBe(SettingsGroup.General);
    expect(Tab.from('settings')).toBeNull();
    expect(Tab.from('setup')).toBeNull();
  });

  it('a tab shown while the pager is closed or already there starts no animation that would hold it off its page later', () => {
    expect(scrollMoves(null, 1), 'Setup was open, so no pager existed and no scroll event would ever end the animation').toBe(false);
    expect(scrollMoves({ scrollLeft: 900, clientWidth: 900 }, 1)).toBe(false);
    expect(scrollMoves({ scrollLeft: 0, clientWidth: 0 }, 1)).toBe(false);
    expect(scrollMoves({ scrollLeft: 0, clientWidth: 900 }, 1)).toBe(true);
  });
});
