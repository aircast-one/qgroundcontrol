import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';

const ANDROID = new URL('../../android/', import.meta.url).pathname;
const SOURCES = ['app/src/main/res/drawable', 'map/src/main/res/drawable'].map((dir) => join(ANDROID, dir));
const OUT = new URL('../src/res/drawables.ts', import.meta.url).pathname;

const attribute = (tag, name) => tag.match(new RegExp(`android:${name}="([^"]*)"`))?.[1];

const color = (value) => {
  if (!value) return undefined;
  if (/^#FF000000$/i.test(value) || /^#000000$/i.test(value)) return 'currentColor';
  const argb = value.match(/^#([0-9a-f]{2})([0-9a-f]{6})$/i);
  return argb ? `#${argb[2]}${argb[1]}` : value;
};

const drawable = (file) => {
  const xml = readFileSync(file, 'utf8');
  const root = xml.match(/<vector[^>]*>/)?.[0] ?? '';
  const paths = [...xml.matchAll(/<path[^>]*\/>/g)].map(([tag]) => Object.fromEntries(Object.entries({
    d: attribute(tag, 'pathData'),
    fill: color(attribute(tag, 'fillColor')) ?? 'none',
    fillOpacity: attribute(tag, 'fillAlpha'),
    fillRule: attribute(tag, 'fillType') === 'evenOdd' ? 'evenodd' : undefined,
    stroke: color(attribute(tag, 'strokeColor')),
    strokeWidth: attribute(tag, 'strokeWidth'),
    strokeLinecap: attribute(tag, 'strokeLineCap'),
    strokeLinejoin: attribute(tag, 'strokeLineJoin'),
  }).filter(([, value]) => value !== undefined)));
  return [basename(file, '.xml'), { viewBox: `0 0 ${attribute(root, 'viewportWidth') ?? 24} ${attribute(root, 'viewportHeight') ?? 24}`, paths }];
};

const entries = SOURCES.flatMap((dir) => readdirSync(dir).filter((name) => name.endsWith('.xml')).map((name) => drawable(join(dir, name))))
  .filter(([, value]) => value.paths.length > 0)
  .toSorted(([a], [b]) => a.localeCompare(b));

writeFileSync(OUT, `export type DrawablePath = { d: string; fill: string; fillOpacity?: string; fillRule?: 'evenodd'; stroke?: string; strokeWidth?: string; strokeLinecap?: string; strokeLinejoin?: string };
export type DrawableData = { viewBox: string; paths: DrawablePath[] };

export const drawables = ${JSON.stringify(Object.fromEntries(entries), null, 2)} as const satisfies Record<string, DrawableData>;

export type DrawableName = keyof typeof drawables;
`);
console.log(`${entries.length} drawables -> ${OUT}`);
