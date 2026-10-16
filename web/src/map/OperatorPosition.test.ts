import { describe, expect, it } from 'vitest';
import { operatorFeatures } from './MissionLayers';
import { operatorPoint } from './OperatorPosition';

describe('OperatorPositionTest', () => {
  it('a usable fix is a point on the map', () => {
    const view = { usable: true, latitude: 41.6983, longitude: 44.85148 };
    expect(operatorPoint(view)).toEqual({ latitude: 41.6983, longitude: 44.85148 });
    expect(operatorFeatures(operatorPoint(view)).features.length).toBe(1);
  });

  it('a fix the core will not stand behind is not drawn', () => {
    const stale = { usable: false, latitude: 41.6983, longitude: 44.85148 };
    expect(operatorPoint(stale)).toBeNull();
    expect(operatorPoint({ usable: true, latitude: null, longitude: null })).toBeNull();
    expect(operatorPoint(null)).toBeNull();
    expect(operatorFeatures(null).features.length).toBe(0);
  });
});
