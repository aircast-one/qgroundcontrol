import { globSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { convert } from 'fontverter';
import * as hb from 'harfbuzzjs';
import subsetFont from 'subset-font';
import type { Plugin } from 'vite';

export const ICON_FONT = 'material-symbols-rounded.woff2';
export const FULL_ICON_FONT = join('node_modules', 'material-symbols', ICON_FONT);
export const ICON_FONT_SUBSET = join('node_modules', '.icon-font', ICON_FONT);

const PRIVATE_USE = 0xe000;
const QUOTED = /['"`]([a-z][a-z0-9_]+)['"`]/g;

export const quotedWords = (sources: readonly string[]): string[] => [...new Set(sources.flatMap((text) => [...text.matchAll(QUOTED)].map((match) => match[1] ?? '')))].toSorted();

export const iconCodepoints = (sfnt: Buffer): ((name: string) => number | undefined) => {
  const face = new hb.Face(new hb.Blob(sfnt));
  const font = new hb.Font(face);
  const icons = new Map([...face.collectUnicodes()].filter((unicode) => unicode >= PRIVATE_USE).map((unicode) => [font.nominalGlyph(unicode), unicode]));
  return (name) => {
    const buffer = new hb.Buffer();
    buffer.addText(name);
    buffer.guessSegmentProperties();
    hb.shape(font, buffer);
    const glyphs = buffer.getGlyphInfos();
    return glyphs.length === 1 ? icons.get(glyphs[0]?.codepoint) : undefined;
  };
};

export const iconSubset = async (woff2: Buffer, words: readonly string[]): Promise<{ font: Buffer; icons: string[] }> => {
  const codepointOf = iconCodepoints(await convert(woff2, 'sfnt'));
  const icons = words.filter((word) => codepointOf(word) !== undefined);
  const letters = [...new Set(icons.join(''))].join('');
  const glyphs = icons.map((icon) => String.fromCodePoint(codepointOf(icon) ?? 0)).join('');
  return { icons, font: await subsetFont(woff2, letters + glyphs, { targetFormat: 'woff2', noLayoutClosure: true }) };
};

const sources = (root: string): Promise<string[]> => Promise.all(globSync('src/**/*.{ts,tsx}', { cwd: root }).map((file) => readFile(join(root, file), 'utf8')));

export const iconFont = (root: string): Plugin => ({
  name: 'icon-font',
  apply: 'build',
  async buildStart() {
    const { font } = await iconSubset(await readFile(join(root, FULL_ICON_FONT)), quotedWords(await sources(root)));
    await mkdir(dirname(join(root, ICON_FONT_SUBSET)), { recursive: true });
    await writeFile(join(root, ICON_FONT_SUBSET), font);
  },
});
