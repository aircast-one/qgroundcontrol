import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { type Clearance, clearanceOf, terrainWarning } from './TerrainProfile';

const clearance = ({ collides = false, metres = null, text = '', complete = true }: { collides?: boolean; metres?: number | null; text?: string; complete?: boolean } = {}): Clearance => ({
  collides,
  metres,
  text,
  complete,
});

describe('TerrainWarningTest', () => {
  it('a route below the ground is named with how far below', () => {
    expect(terrainWarning(clearance({ collides: true, metres: -150.0, text: '150.0 m' }))).toBe('The route goes 150.0 m below the ground.');
  });

  it('a route below the ground is still named when the ground is only partly known', () => {
    expect(terrainWarning(clearance({ collides: true, metres: -12.0, text: '12.0 m', complete: false }))).toBe('The route goes 12.0 m below the ground.');
  });

  it('a route that clears is not announced, because clearing is the ordinary case', () => {
    expect(terrainWarning(clearance({ metres: 50.0, text: '50.0 m' }))).toBeNull();
  });

  it('a clearance measured over ground that is only partly known says nothing', () => {
    expect(terrainWarning(clearance({ metres: 4.0, text: '4.0 m', complete: false }))).toBeNull();
  });

  it('no terrain at all says nothing rather than guessing either way', () => {
    expect(terrainWarning(clearance({ metres: null, complete: false }))).toBeNull();
    expect(terrainWarning(null)).toBeNull();
  });

  it('a collision with no figure still warns rather than falling silent', () => {
    expect(terrainWarning(clearance({ collides: true, metres: null, text: '' }))).toBe('The route goes below the ground.');
  });

  it('the clearance is read from the view the core serves', () => {
    const read = clearanceOf(parseJson('{"points":[],"hasCollision":true,"minClearanceMetres":-8.5,' + '"clearanceText":"8.5 m","clearanceComplete":true}'));
    expect(read?.collides).toBe(true);
    expect(read?.metres ?? 0.0).toBeCloseTo(-8.5, 9);
    expect(read?.text).toBe('8.5 m');
  });

  it('a null clearance from the core reads as unknown, not as zero', () => {
    const view = parseJson('{"points":[],"hasCollision":false,"minClearanceMetres":null,' + '"clearanceText":null,"clearanceComplete":false}');
    expect(clearanceOf(view)?.metres).toBeNull();
    expect(clearanceOf(view)?.text).toBe('');
  });
});
