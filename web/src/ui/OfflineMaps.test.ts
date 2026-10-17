import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { offlineMaps, offlineMapsPath, offlineRegion, regionCentre, renameWanted } from './OfflineMaps';

describe('OfflineMapsTest', () => {
  it('the visible corners become the region the url factory counts', () => {
    const region = offlineRegion([
      { latitude: 47.4, longitude: 8.5 },
      { latitude: 47.4, longitude: 8.6 },
      { latitude: 47.35, longitude: 8.6 },
      { latitude: 47.35, longitude: 8.5 },
    ]);
    expect(region).toEqual({ west: 8.5, north: 47.4, east: 8.6, south: 47.35 });
    expect(offlineMapsPath('Google Satellite', region, 13, 19)).toBe('view.offlineMaps(Google Satellite,8.5000000,47.4000000,8.6000000,47.3500000,13,19,true)');
    expect(offlineMapsPath('Google Satellite', null, 13, 19)).toBe('view.offlineMaps');
    expect(offlineRegion([])).toBeNull();
  });

  it('sets and the estimate read from the view', () => {
    const read = offlineMaps(
      parseJson(
        '{"class":"OfflineMaps","available":true,"sets":[{"id":1,"name":"System Wide Tile Cache","defaultSet":true,' +
          '"downloadStatus":"1.0MB","downloading":false,"complete":true}],"mapList":["Bing Road"],"uniqueName":"Tile Set 001",' +
          '"takenNames":["Default Tile Set"],"estimate":{"tileCountText":"1,234","tileSizeText":"5.0MB","tooMany":false}}',
      ),
    );
    expect(read?.sets.map((it) => it.name)).toEqual(['System Wide Tile Cache']);
    expect(read?.uniqueName).toBe('Tile Set 001');
    expect(read?.estimate).toEqual({ tileCountText: '1,234', tileSizeText: '5.0MB', tooMany: false });
  });

  it('ok renames only to a new non-blank name', () => {
    expect(renameWanted('Set 1', ' Valley ')).toBe('Valley');
    expect(renameWanted('Set 1', 'Set 1')).toBeNull();
    expect(renameWanted('Set 1', '  ')).toBeNull();
  });

  it("zoom previews centre on the chosen region, like OfflineMapEditor's preview maps", () => {
    expect(regionCentre({ west: 44.0, north: 42.0, east: 45.0, south: 41.0 })).toEqual({ latitude: 41.5, longitude: 44.5 });
  });
});
