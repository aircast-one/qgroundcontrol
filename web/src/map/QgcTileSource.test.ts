import { describe, expect, it } from 'vitest';
import { QGC_TILE_HOST, mapTypeName, osmTileUrl, qgcRasterStyle, qgcTileUrl, tileAddress } from './QgcTileSource';
import { OSM_RASTER_STYLE, OSM_TILE_URL } from './VehicleMap';

const pathFor = (mapType: string, z: number, x: number, y: number): string =>
  qgcTileUrl(mapType).slice(`https://${QGC_TILE_HOST}`.length).replace('{z}', `${z}`).replace('{x}', `${x}`).replace('{y}', `${y}`);

describe('QgcTileSourceTest', () => {
  it('a tile path names the map type and the tile', () => {
    expect(tileAddress(pathFor('Bing Hybrid', 14, 9876, 6543))).toEqual({ mapType: 'Bing Hybrid', z: 14, x: 9876, y: 6543 });
    expect(tileAddress(pathFor('Google Street Map', 0, 0, 0))).toEqual({ mapType: 'Google Street Map', z: 0, x: 0, y: 0 });
  });

  it('anything that is not a tile path names no tile', () => {
    expect(tileAddress('/14/9876/6543')).toBeNull();
    expect(tileAddress('/style.json')).toBeNull();
    expect(tileAddress('/Bing/14/9876/6543.png')).toBeNull();
  });

  it('a tile nothing else serves falls back to OpenStreetMap', () => {
    expect(osmTileUrl({ mapType: 'Bing Hybrid', z: 14, x: 9876, y: 6543 })).toBe('https://tile.openstreetmap.org/14/9876/6543.png');
    expect(OSM_RASTER_STYLE.includes(OSM_TILE_URL)).toBe(true);
  });

  it('the map type is the provider and type the settings name', () => {
    expect(mapTypeName('Bing', 'Hybrid')).toBe('Bing Hybrid');
    expect(mapTypeName('Bing', '')).toBe('Bing');
    expect(qgcRasterStyle('Esri World Satellite').includes(qgcTileUrl('Esri World Satellite'))).toBe(true);
  });
});
