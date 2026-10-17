import { describe, expect, it } from 'vitest';
import { ProfilePoint, TerrainProfile } from './TerrainProfile';

const at = (distance: number, terrain: number | null) => ProfilePoint({ distance, terrain, planned: 100.0 });

describe('TerrainCoverageTest', () => {
  it('terrain under the whole route is full coverage', () => {
    expect(TerrainProfile({ points: [at(0.0, 400.0), at(500.0, 410.0), at(1000.0, 420.0)] }).terrainCoverage).toBeCloseTo(1.0, 9);
  });

  it('a leg with no ground under it does not count', () => {
    expect(TerrainProfile({ points: [at(0.0, 400.0), at(500.0, 410.0), at(1000.0, null)] }).terrainCoverage).toBeCloseTo(0.5, 9);
  });

  it('ground at one end only is measured by distance not by sample count', () => {
    const dense = Array.from({ length: 10 }, (_, it) => at(it * 10.0, 400.0));
    const bare = [at(1000.0, null), at(2000.0, null)];
    expect(TerrainProfile({ points: [...dense, ...bare] }).terrainCoverage).toBeCloseTo(0.045, 9);
  });

  it('no terrain anywhere is no coverage', () => {
    expect(TerrainProfile({ points: [at(0.0, null), at(1000.0, null)] }).terrainCoverage).toBeCloseTo(0.0, 9);
  });

  it('a route of no length has no coverage rather than dividing by zero', () => {
    expect(TerrainProfile({ points: [] }).terrainCoverage).toBeCloseTo(0.0, 9);
  });
});
