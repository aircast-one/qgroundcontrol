import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { offsetArgs, transformHome, transformUnit } from './PlanTransform';

describe('PlanTransformTest', () => {
  it('an offset sends east, north, up and the two scope flags', () => {
    const metres = { name: 'm', metresPerUnit: 1.0 };
    expect(offsetArgs('10', '-5', '', metres, metres, true, false)).toEqual([10.0, -5.0, 0.0, true, false]);
    expect(offsetArgs('ten', '0', '0', metres, metres, false, false)).toBeNull();
  });

  it('offsets typed in feet are sent in metres like the cooked QGC facts', () => {
    const view = parseJson('{"horizontalUnit":"ft","horizontalMetresPerUnit":0.3048,"verticalUnit":"m","verticalMetresPerUnit":1.0}');
    const feet = transformUnit(view, 'horizontal');
    expect(feet).toEqual({ name: 'ft', metresPerUnit: 0.3048 });
    expect(offsetArgs('10', '0', '2', feet, transformUnit(view, 'vertical'), false, false)).toEqual([3.048, 0.0, 2.0, false, false]);
    expect(transformUnit(null, 'horizontal')).toEqual({ name: 'm', metresPerUnit: 1.0 });
  });

  it('home comes from the transform view', () => {
    expect(transformHome(parseJson('{"home":{"latitude":47.0,"longitude":8.0}}'))).toEqual({ latitude: 47.0, longitude: 8.0 });
    expect(transformHome(parseJson('{"home":null}'))).toBeNull();
  });
});
