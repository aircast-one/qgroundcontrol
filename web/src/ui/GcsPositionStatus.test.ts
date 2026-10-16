import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { gcsPositionRows } from './GcsPositionStatus';

describe('GcsPositionStatusTest', () => {
  it('rows read as GcsPositionStatus does and hide without a valid position', () => {
    const position = parseJson('{"kind":"value","value":{"latitude":47.123456789,"longitude":8.5,"valid":true}}');
    expect(gcsPositionRows(position, parseJson('{"value":2.345}'))).toEqual([
      ['Latitude', '47.1234568'],
      ['Longitude', '8.5000000'],
      ['HDOP', '2.3 m'],
    ]);
    expect(gcsPositionRows(position, parseJson('{"value":0}'))!.at(-1)![1]).toBe('N/A');
    expect(gcsPositionRows(parseJson('{"value":{"valid":false}}'), null)).toBeNull();
    expect(gcsPositionRows(null, null)).toBeNull();
  });
});
