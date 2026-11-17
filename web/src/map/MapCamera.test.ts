import { describe, expect, it } from 'vitest';
import { launchCamera, vehicleSpot } from './MapCamera';

describe('MapCamera', () => {
  it('a launch opens on the last map view, else where the vehicle was last seen, else nowhere in particular', () => {
    const saved = { centre: { latitude: 47.39, longitude: 8.54 }, zoom: 15 };
    const vehicle = { latitude: 41.7, longitude: 44.8 };
    expect(launchCamera(saved, vehicle)).toBe(saved);
    expect(launchCamera(null, vehicle)).toEqual({ centre: vehicle, zoom: 17 });
    expect(launchCamera(null, null)).toBeNull();
  });

  it('the vehicle spot is kept to about ten metres so a hovering aircraft does not rewrite it every frame', () => {
    expect(vehicleSpot({ latitude: 47.3977419, longitude: 8.5455938 })).toBe('47.3977,8.5456');
  });
});
