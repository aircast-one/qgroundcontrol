import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { altitudeHint, altitudesRelative, previousCoordinate } from './ItemEditor';

describe('PreviousCoordinateTest', () => {
  it('move to previous item is offered only when the core names a previous position', () => {
    expect(previousCoordinate(parseJson('{"previousCoordinate":{"latitude":47.1,"longitude":8.1}}'))).toEqual([47.1, 8.1]);
    expect(previousCoordinate(parseJson('{"previousCoordinate":null}'))).toBeNull();
    expect(previousCoordinate(null)).toBeNull();
  });

  it("the altitude hint is the core's sentence or nothing", () => {
    expect(altitudeHint(parseJson('{"altitudeHint":"Actual AMSL alt sent: 512.3 m"}'))).toBe('Actual AMSL alt sent: 512.3 m');
    expect(altitudeHint(parseJson('{"altitudeHint":null}'))).toBeNull();
  });

  it('a landing pattern offers altitudes relative to launch', () => {
    expect(altitudesRelative(parseJson('{"landing":true,"altitudesAreRelative":false}'))).toBe(false);
    expect(altitudesRelative(parseJson('{"landing":false,"altitudesAreRelative":null}'))).toBeNull();
  });
});
