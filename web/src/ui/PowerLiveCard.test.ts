import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { PowerTile, powerTiles } from './PowerLiveCard';

describe('PowerLiveCardTest', () => {
  it('theTilesReadTheFirstPackInOrder', () => {
    const view = parseJson(
      '{"available":true,"packs":[{"facts":[{"name":"mahConsumed","units":"mAh","valueString":"1800"},{"name":"voltage","units":"v","valueString":"11.10"},{"name":"current","units":"A","valueString":"32.00"}]}]}',
    );
    expect(powerTiles(view)).toEqual([PowerTile('VOLTAGE', '11.10', 'V'), PowerTile('CURRENT', '32.00', 'A'), PowerTile('USED', '1800', 'mAh')]);
  });

  it('noBatteryMeansNoCard', () => {
    expect(powerTiles(parseJson('{"available":false}'))).toEqual([]);
    expect(powerTiles(null)).toEqual([]);
  });
});
