import { readFile } from 'node:fs/promises';
import { convert } from 'fontverter';
import { describe, expect, it } from 'vitest';
import { FULL_ICON_FONT, iconCodepoints, iconSubset, quotedWords } from './iconFont';

describe('the icon font the build ships', () => {
  it('collects every quoted lowercase word in the sources once', () => {
    expect(quotedWords([`<MaterialIcon name="menu" />`, `open ? 'expand_less' : 'expand_more'`, "`check` 'menu' 'Title' 'x'"])).toEqual(['check', 'expand_less', 'expand_more', 'menu']);
  });

  it('keeps the icons the app names and drops the rest', async () => {
    const full = await readFile(FULL_ICON_FONT);
    const { font, icons } = await iconSubset(full, ['keyboard_arrow_right', 'menu', 'not_an_icon', 'speed']);
    expect(icons).toEqual(['keyboard_arrow_right', 'menu', 'speed']);
    const codepointOf = iconCodepoints(await convert(font, 'sfnt'));
    expect(icons.map(codepointOf).every((codepoint) => codepoint !== undefined)).toBe(true);
    expect(codepointOf('home')).toBeUndefined();
    expect(font.length).toBeLessThan(full.length / 50);
  });
});
