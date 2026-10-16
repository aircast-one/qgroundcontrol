import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { TerrainLoad, terrainLoad } from './TerrainDataWarning';

const json = (text: string): Json => parseJson(text) ?? {};

describe('TerrainDataWarningTest', () => {
  it('the warning watches for blocks still pending, and a load that finished or never started is quiet', () => {
    const loading = terrainLoad(json('{"loaded":3,"pending":1,"fraction":0.75}'));
    expect(loading).toEqual(TerrainLoad(3, 1));
    expect(loading.waiting).toBe(true);
    expect(terrainLoad(json('{"loaded":5,"pending":0}')).waiting, 'everything loaded is nothing to warn about').toBe(false);
    expect(terrainLoad(json('{"kind":"null"}')).waiting, 'no vehicle, no terrain view').toBe(false);
    expect(terrainLoad(null).waiting).toBe(false);
  });
});
