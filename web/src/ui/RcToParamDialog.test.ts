import { describe, expect, it } from 'vitest';
import { rcToParam } from './RcToParamDialog';

describe('RcToParamTest', () => {
  it('every field must be a number before the mapping is sent', () => {
    expect(rcToParam('1.0', '6.5', 2, '0', '12')).toEqual({ scale: 1.0, center: 6.5, tuningIndex: 2, min: 0.0, max: 12.0 });
    expect(rcToParam('1.0', '', 0, '0', '12')).toBeNull();
    expect(rcToParam('x', '6.5', 0, '0', '12')).toBeNull();
  });
});
