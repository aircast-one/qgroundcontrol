import { describe, expect, it } from 'vitest';
import type { Fact } from '@/bridge/Qgc';
import { rcCenter, rcToParam } from './RcToParamDialog';

describe('RcToParamTest', () => {
  it('every field must be a number before the mapping is sent', () => {
    expect(rcToParam('1.0', '6.5', 2, '0', '12')).toEqual({ scale: 1.0, center: 6.5, tuningIndex: 2, min: 0.0, max: 12.0 });
    expect(rcToParam('1.0', '', 0, '0', '12')).toBeNull();
    expect(rcToParam('x', '6.5', 0, '0', '12')).toBeNull();
  });
});

describe('RcCenterTest', () => {
  it('the center starts at the value as the parameter list shows it, not the float the vehicle stored', () => {
    expect(rcCenter({ value: 0.20000000298023224, valueString: '0.20' } as Fact)).toBe('0.20');
    expect(rcCenter({ value: 3, valueString: '3' } as Fact)).toBe('3');
    expect(rcCenter({ value: 1.5, valueString: '' } as Fact)).toBe('1.5');
  });
});
