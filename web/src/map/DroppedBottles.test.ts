import { describe, expect, it } from 'vitest';
import { bottleFeatures, landedBottles } from './DroppedBottles';

describe('DroppedBottlesTest', () => {
  it('the map marks where each landed bottle lies, not one still falling', () => {
    const view = {
      available: true,
      bottles: [
        { bay: 1, latitude: -35.3631, longitude: 149.1652, aboveHome: 0, landed: true },
        { bay: 2, latitude: -35.3632, longitude: 149.1653, aboveHome: 7.5, landed: false },
        { bay: 3, latitude: -35.3633, longitude: 149.1654, aboveHome: 0, landed: true },
      ],
    };
    expect(landedBottles(view)).toEqual([
      { latitude: -35.3631, longitude: 149.1652 },
      { latitude: -35.3633, longitude: 149.1654 },
    ]);
    expect(bottleFeatures(landedBottles(view)).features.length).toBe(2);
  });

  it('nothing is marked before a drop', () => {
    expect(landedBottles(null)).toEqual([]);
    expect(landedBottles({ available: true, bottles: [] })).toEqual([]);
  });
});
